use std::sync::Arc;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use rho_cli::RhoClient;
use rho_protocol::{Artifact, ClientKind, ExecuteRunRequest, Object, Problem, Run, Workspace};
use serde_json::{Map, Value, json};

pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";
pub const MCP_SERVER_NAME: &str = "rho-mcp";

const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

#[async_trait]
pub trait McpBackend: Send + Sync {
    async fn status(&self) -> Result<Workspace>;
    async fn execute(&self, request: ExecuteRunRequest) -> Result<Run>;
    async fn objects(&self) -> Result<Vec<Object>>;
    async fn inspect_object(&self, object: &str) -> Result<Object>;
    async fn runs(&self) -> Result<Vec<Run>>;
    async fn problems(&self) -> Result<Vec<Problem>>;
    async fn plots(&self) -> Result<Vec<Artifact>>;
}

#[async_trait]
impl McpBackend for RhoClient {
    async fn status(&self) -> Result<Workspace> {
        RhoClient::status(self).await
    }

    async fn execute(&self, request: ExecuteRunRequest) -> Result<Run> {
        RhoClient::execute(self, &request).await
    }

    async fn objects(&self) -> Result<Vec<Object>> {
        RhoClient::objects(self).await
    }

    async fn inspect_object(&self, object: &str) -> Result<Object> {
        RhoClient::inspect_object(self, object).await
    }

    async fn runs(&self) -> Result<Vec<Run>> {
        RhoClient::runs(self).await
    }

    async fn problems(&self) -> Result<Vec<Problem>> {
        RhoClient::problems(self).await
    }

    async fn plots(&self) -> Result<Vec<Artifact>> {
        RhoClient::plots(self).await
    }
}

pub struct McpServer {
    backend: Arc<dyn McpBackend>,
    initialized: bool,
}

impl McpServer {
    pub fn new(server_url: &str) -> Result<Self> {
        Ok(Self::with_backend(RhoClient::new(server_url)?))
    }

    pub fn with_backend(backend: impl McpBackend + 'static) -> Self {
        Self {
            backend: Arc::new(backend),
            initialized: false,
        }
    }

    pub async fn handle(&mut self, message: Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return id.map(|id| rpc_error(id, -32600, "Invalid Request"));
        };
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return id.map(|id| rpc_error(id, -32600, "Invalid JSON-RPC version"));
        }

        match method {
            "initialize" => {
                self.initialized = false;
                let requested = message["params"]["protocolVersion"]
                    .as_str()
                    .unwrap_or(MCP_PROTOCOL_VERSION);
                let protocol_version = if SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
                    requested
                } else {
                    MCP_PROTOCOL_VERSION
                };
                id.map(|id| {
                    rpc_result(
                        id,
                        json!({
                            "protocolVersion": protocol_version,
                            "capabilities": {"tools": {"listChanged": false}},
                            "serverInfo": {
                                "name": MCP_SERVER_NAME,
                                "title": "Rho Scientific Runtime",
                                "version": env!("CARGO_PKG_VERSION"),
                                "description": "Scientific semantics and safe execution for the authoritative R Workspace"
                            },
                            "instructions": "Use Rho for R state, scientific objects, run history, problems, and artifacts. General reasoning remains in the Agent client."
                        }),
                    )
                })
            }
            "notifications/initialized" => {
                self.initialized = true;
                None
            }
            "notifications/cancelled" => None,
            "ping" => id.map(|id| rpc_result(id, json!({}))),
            "tools/list" => id.map(|id| {
                if self.initialized {
                    rpc_result(id, json!({"tools": tool_definitions()}))
                } else {
                    rpc_error(id, -32002, "Server is not initialized")
                }
            }),
            "tools/call" => {
                let id = id?;
                if !self.initialized {
                    return Some(rpc_error(id, -32002, "Server is not initialized"));
                }
                let Some(name) = message["params"]["name"].as_str() else {
                    return Some(rpc_error(id, -32602, "tools/call requires a tool name"));
                };
                if !tool_definitions()
                    .iter()
                    .any(|tool| tool["name"].as_str() == Some(name))
                {
                    return Some(rpc_error(id, -32602, &format!("Unknown tool: {name}")));
                }
                let arguments = message["params"]["arguments"]
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                let result = match self.call_tool(name, &arguments).await {
                    Ok(value) => tool_result(value, false),
                    Err(error) => tool_result(json!({"error": format!("{error:#}")}), true),
                };
                Some(rpc_result(id, result))
            }
            _ => id.map(|id| rpc_error(id, -32601, "Method not found")),
        }
    }

    async fn call_tool(&self, name: &str, arguments: &Map<String, Value>) -> Result<Value> {
        match name {
            "workspace_open" => {
                let workspace = self.backend.status().await?;
                let objects = self.backend.objects().await?;
                Ok(json!({
                    "workspace": workspace,
                    "objects": objects,
                    "next": {
                        "inspect": "Call object_inspect with an object name or stable object ID.",
                        "execute": "Call workspace_execute only after the requested R code and its effects are understood."
                    }
                }))
            }
            "workspace_status" => Ok(serde_json::to_value(self.backend.status().await?)?),
            "workspace_execute" => {
                let code = required_string(arguments, "code")?;
                Ok(serde_json::to_value(
                    self.backend
                        .execute(ExecuteRunRequest {
                            code,
                            source_path: optional_string(arguments, "source_path")?,
                            execution_mode: optional_string(arguments, "execution_mode")?,
                            document_version: arguments
                                .get("document_version")
                                .map(|value| {
                                    value
                                        .as_i64()
                                        .context("document_version must be an integer")
                                })
                                .transpose()?,
                            client_kind: Some(ClientKind::Mcp),
                        })
                        .await?,
                )?)
            }
            "object_inspect" => {
                let object = required_string(arguments, "object")?;
                Ok(serde_json::to_value(
                    self.backend.inspect_object(&object).await?,
                )?)
            }
            "run_history" => Ok(serde_json::to_value(self.backend.runs().await?)?),
            "problem_list" => Ok(serde_json::to_value(self.backend.problems().await?)?),
            "artifact_export" => {
                let artifact_id = required_string(arguments, "artifact_id")?;
                let artifacts = self.backend.plots().await?;
                Ok(serde_json::to_value(
                    artifacts
                        .into_iter()
                        .find(|artifact| artifact.artifact_id == artifact_id)
                        .with_context(|| format!("Artifact `{artifact_id}` was not found"))?,
                )?)
            }
            "plot_view" => {
                let plot_id = required_string(arguments, "plot_id")?;
                let plots = self.backend.plots().await?;
                Ok(serde_json::to_value(
                    plots
                        .into_iter()
                        .find(|plot| plot.artifact_id == plot_id)
                        .with_context(|| format!("Plot `{plot_id}` was not found"))?,
                )?)
            }
            _ => bail!("Unknown tool: {name}"),
        }
    }
}

fn required_string(arguments: &Map<String, Value>, name: &str) -> Result<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .with_context(|| format!("`{name}` must be a non-empty string"))
}

fn optional_string(arguments: &Map<String, Value>, name: &str) -> Result<Option<String>> {
    arguments
        .get(name)
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .with_context(|| format!("`{name}` must be a string"))
        })
        .transpose()
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

fn tool_result(value: Value, is_error: bool) -> Value {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": {"data": value},
        "isError": is_error
    })
}

pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "workspace_open",
            "Open Workspace",
            "Connect to the authoritative Rho workspace and return its durable identity and lifecycle.",
            empty_schema(),
            true,
        ),
        tool(
            "workspace_status",
            "Workspace Status",
            "Read the authoritative workspace lifecycle, project root, and revision identity.",
            empty_schema(),
            true,
        ),
        tool(
            "workspace_execute",
            "Execute R",
            "Execute R code in the persistent authoritative Workspace R and record a provenance-bearing run.",
            json!({
                "type": "object",
                "properties": {
                    "code": {"type": "string", "description": "R code to execute"},
                    "source_path": {"type": "string"},
                    "execution_mode": {"type": "string"},
                    "document_version": {"type": "integer"}
                },
                "required": ["code"],
                "additionalProperties": false
            }),
            false,
        ),
        tool(
            "object_inspect",
            "Inspect R Object",
            "Return bounded semantic metadata and a safe preview for one live R object.",
            one_string_schema("object", "Stable object ID or R object name"),
            true,
        ),
        tool(
            "run_history",
            "Run History",
            "List recorded Rho runs and their state/project revision transitions.",
            empty_schema(),
            true,
        ),
        tool(
            "problem_list",
            "Problems",
            "List structured scientific execution problems with calls and tracebacks.",
            empty_schema(),
            true,
        ),
        tool(
            "artifact_export",
            "Export Artifact",
            "Return a scientific artifact payload and provenance metadata without writing client files.",
            one_string_schema("artifact_id", "Stable artifact ID"),
            true,
        ),
        tool(
            "plot_view",
            "View Plot",
            "Return one plot payload, media type, and provenance metadata.",
            one_string_schema("plot_id", "Stable plot artifact ID"),
            true,
        ),
    ]
}

fn tool(name: &str, title: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": read_only,
            "destructiveHint": !read_only,
            "idempotentHint": read_only,
            "openWorldHint": false
        }
    })
}

fn empty_schema() -> Value {
    json!({"type": "object", "additionalProperties": false})
}

fn one_string_schema(name: &str, description: &str) -> Value {
    let mut properties = Map::new();
    properties.insert(
        name.to_string(),
        json!({"type": "string", "description": description}),
    );
    json!({
        "type": "object",
        "properties": properties,
        "required": [name],
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rho_server::runtime::{RuntimeService, RuntimeServiceConfig};
    use tempfile::TempDir;

    async fn server() -> (
        TempDir,
        McpServer,
        tokio::task::JoinHandle<anyhow::Result<()>>,
    ) {
        let directory = TempDir::new().unwrap();
        let runtime = RuntimeService::open(
            RuntimeServiceConfig::new(directory.path().join("runtime.sqlite"))
                .with_workspace_id("ws_mcp"),
        )
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(rho_server::api::serve(listener, runtime));
        let server = McpServer::new(&format!("http://{address}")).unwrap();
        (directory, server, task)
    }

    async fn initialize(server: &mut McpServer) -> Value {
        let response = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": MCP_PROTOCOL_VERSION, "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}
            }))
            .await
            .unwrap();
        assert!(
            server
                .handle(json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                }))
                .await
                .is_none()
        );
        response
    }

    #[tokio::test]
    async fn negotiates_latest_protocol_and_lists_exact_scientific_tools() {
        let (_directory, mut server, task) = server().await;
        let response = initialize(&mut server).await;
        assert_eq!(response["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(
            response["result"]["capabilities"]["tools"]["listChanged"],
            false
        );

        let response = server
            .handle(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}))
            .await
            .unwrap();
        let names = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "workspace_open",
                "workspace_status",
                "workspace_execute",
                "object_inspect",
                "run_history",
                "problem_list",
                "artifact_export",
                "plot_view"
            ]
        );
        let inspect = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "object_inspect")
            .unwrap();
        assert!(inspect["inputSchema"]["properties"]["object"].is_object());
        task.abort();
    }

    #[tokio::test]
    async fn tool_results_include_text_and_structured_content() {
        let (_directory, mut server, task) = server().await;
        initialize(&mut server).await;
        let response = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": "workspace_status", "arguments": {}}
            }))
            .await
            .unwrap();
        assert_eq!(response["result"]["isError"], false);
        assert_eq!(
            response["result"]["structuredContent"]["data"]["workspace_id"],
            "ws_mcp"
        );
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("ws_mcp")
        );
        task.abort();
    }

    #[tokio::test]
    async fn workspace_open_discovers_live_objects_without_an_internal_rpc() {
        let (_directory, mut server, task) = server().await;
        initialize(&mut server).await;
        let response = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": "workspace_open", "arguments": {}}
            }))
            .await
            .unwrap();
        assert_eq!(
            response["result"]["structuredContent"]["data"]["workspace"]["workspace_id"],
            "ws_mcp"
        );
        assert_eq!(
            response["result"]["structuredContent"]["data"]["objects"],
            json!([])
        );
        task.abort();
    }

    #[tokio::test]
    async fn execution_failures_are_tool_errors_and_unknown_tools_are_protocol_errors() {
        let (_directory, mut server, task) = server().await;
        initialize(&mut server).await;
        let unavailable = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": "workspace_execute", "arguments": {"code": "1 + 1"}}
            }))
            .await
            .unwrap();
        assert_eq!(unavailable["result"]["isError"], true);
        assert!(unavailable.get("error").is_none());

        let unknown = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "internal_rpc", "arguments": {}}
            }))
            .await
            .unwrap();
        assert_eq!(unknown["error"]["code"], -32602);
        task.abort();
    }
}
