use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use rho_mcp::{McpBackend, McpServer};
use rho_protocol::{
    ApiError, ApiResponse, Approval, ApprovalStatus, Artifact, ArtifactKind, ClientKind, DeepLink,
    DependencyReport, DependencySummary, EventPage, EventStreamMessage, ExecuteRunRequest,
    Object as ProtocolObject, Problem, ProblemSeverity, Provenance, ProvenanceEdge, ProvenanceNode,
    ProvenanceNodeKind, Run as ProtocolRun, RuntimeHealth, Workspace, WorkspaceLifecycle,
    workbench_protocol_schema,
};
use rho_runtime_deps::EnsureOptions;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock, broadcast};
use uuid::Uuid;

use crate::policy::{PolicyAction, PolicyDecision, PolicyEngine, PolicyPrincipal};
use crate::runtime::RuntimeService;

const MAX_EVENT_PAGE: usize = 1_000;
const DEFAULT_EVENT_PAGE: usize = 100;
const MAX_MCP_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_MCP_SESSIONS: usize = 256;
const MCP_SESSION_TTL: Duration = Duration::from_secs(30 * 60);
const MCP_SESSION_ID_HEADER: HeaderName = HeaderName::from_static("mcp-session-id");

#[derive(Clone)]
struct ApiState {
    runtime: RuntimeService,
    mcp_sessions: Arc<TokioRwLock<HashMap<String, McpHttpSession>>>,
}

struct McpHttpSession {
    workspace_id: String,
    server: Arc<TokioMutex<McpServer>>,
    last_activity: Instant,
}

#[derive(Debug, Default, Deserialize)]
struct EventQuery {
    after: Option<u64>,
    limit: Option<usize>,
    client: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ListQuery {
    limit: Option<usize>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DependencyActionRequest {
    action: String,
    #[serde(default)]
    confirmed: bool,
    #[serde(default)]
    offline: bool,
}

pub fn router(runtime: RuntimeService) -> Router {
    let state = ApiState {
        runtime,
        mcp_sessions: Arc::new(TokioRwLock::new(HashMap::new())),
    };
    Router::new()
        .route("/", get(web_index))
        .route("/app.js", get(web_javascript))
        .route("/styles.css", get(web_styles))
        .route("/agent-setup.md", get(agent_setup))
        .route(
            "/agent/skills/operate-rho-runtime/SKILL.md",
            get(agent_skill),
        )
        .route(
            "/agent/skills/operate-rho-runtime/agents/openai.yaml",
            get(agent_skill_metadata),
        )
        .route(
            "/agent/skills/operate-rho-runtime/references/workbench-protocol.md",
            get(agent_skill_protocol),
        )
        .route("/healthz", get(health))
        .route("/readyz", get(readiness))
        .route("/mcp", get(mcp_get).post(mcp_post).delete(mcp_delete))
        .route(
            "/v1/runtime/dependencies",
            get(runtime_dependencies).post(runtime_dependency_action),
        )
        .route("/v1/schema", get(schema))
        .route("/v1/workspaces/current", get(current_workspace))
        .route("/v1/workspaces/{workspace_id}", get(workspace))
        .route(
            "/v1/workspaces/{workspace_id}/runs",
            get(list_runs).post(execute_run),
        )
        .route("/v1/workspaces/{workspace_id}/problems", get(list_problems))
        .route("/v1/workspaces/{workspace_id}/plots", get(list_plots))
        .route(
            "/v1/workspaces/{workspace_id}/approvals",
            get(list_approvals),
        )
        .route("/v1/workspaces/{workspace_id}/objects", get(list_objects))
        .route(
            "/v1/workspaces/{workspace_id}/objects/{object_id}",
            get(get_object),
        )
        .route(
            "/v1/workspaces/{workspace_id}/provenance",
            get(get_provenance),
        )
        .route("/v1/workspaces/{workspace_id}/events", get(list_events))
        .route(
            "/v1/workspaces/{workspace_id}/events/ws",
            get(workspace_events_ws),
        )
        .route(
            "/v1/workspaces/{workspace_id}/mcp",
            get(mcp_workspace_get)
                .post(mcp_workspace_post)
                .delete(mcp_workspace_delete),
        )
        .with_state(state)
}

async fn web_index() -> Html<&'static str> {
    Html(include_str!("../../../web/index.html"))
}

async fn web_javascript() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/app.js"),
    )
}

async fn web_styles() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../web/styles.css"),
    )
}

async fn agent_setup() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/markdown; charset=utf-8")],
        include_str!("../../../docs/agent-setup.md"),
    )
}

async fn agent_skill() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/markdown; charset=utf-8")],
        include_str!("../../../.agents/skills/operate-rho-runtime/SKILL.md"),
    )
}

async fn agent_skill_metadata() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "application/yaml; charset=utf-8")],
        include_str!("../../../.agents/skills/operate-rho-runtime/agents/openai.yaml"),
    )
}

async fn agent_skill_protocol() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/markdown; charset=utf-8")],
        include_str!(
            "../../../.agents/skills/operate-rho-runtime/references/workbench-protocol.md"
        ),
    )
}

async fn mcp_get() -> Response {
    mcp_method_not_allowed()
}

async fn mcp_workspace_get(AxumPath(_workspace_id): AxumPath<String>) -> Response {
    mcp_method_not_allowed()
}

async fn mcp_post(State(state): State<ApiState>, headers: HeaderMap, body: Bytes) -> Response {
    handle_mcp_post(state, None, headers, body).await
}

async fn mcp_workspace_post(
    AxumPath(workspace_id): AxumPath<String>,
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_mcp_post(state, Some(workspace_id), headers, body).await
}

async fn handle_mcp_post(
    state: ApiState,
    requested_workspace_id: Option<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !is_json_content_type(&headers) {
        return mcp_error_response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Value::Null,
            -32600,
            "MCP requests require Content-Type: application/json",
        );
    }
    if body.len() > MAX_MCP_MESSAGE_BYTES {
        return mcp_error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            Value::Null,
            -32600,
            "MCP message exceeds 1 MiB",
        );
    }
    let message = match serde_json::from_slice::<Value>(&body) {
        Ok(Value::Object(message)) => Value::Object(message),
        Ok(_) => {
            return mcp_error_response(
                StatusCode::BAD_REQUEST,
                Value::Null,
                -32600,
                "MCP message must be a JSON object",
            );
        }
        Err(error) => {
            return mcp_error_response(
                StatusCode::BAD_REQUEST,
                Value::Null,
                -32700,
                &format!("Parse error: {error}"),
            );
        }
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let workspace = state.runtime.workspace().await;
    if requested_workspace_id
        .as_deref()
        .is_some_and(|requested| requested != workspace.workspace_id)
    {
        return mcp_error_response(
            StatusCode::NOT_FOUND,
            id,
            -32004,
            &format!(
                "Workspace `{}` does not exist",
                requested_workspace_id.as_deref().unwrap_or_default()
            ),
        );
    }

    let method = message.get("method").and_then(Value::as_str);
    if method == Some("initialize") {
        if headers.contains_key(&MCP_SESSION_ID_HEADER) {
            return mcp_error_response(
                StatusCode::BAD_REQUEST,
                id,
                -32600,
                "initialize must not include an MCP session ID",
            );
        }
        if message.get("id").is_none()
            || message.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || message
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .is_none()
        {
            return mcp_error_response(
                StatusCode::BAD_REQUEST,
                id,
                -32600,
                "initialize requires JSON-RPC 2.0, an id, and params.protocolVersion",
            );
        }
        let workspace_id = workspace.workspace_id;
        let mut server = McpServer::with_backend(RuntimeMcpBackend {
            runtime: state.runtime.clone(),
            workspace_id: workspace_id.clone(),
        });
        let Some(response) = server.handle(message).await else {
            return mcp_error_response(
                StatusCode::BAD_REQUEST,
                id,
                -32600,
                "initialize did not produce a response",
            );
        };
        let session_id = format!("mcp_session_{}", Uuid::new_v4().simple());
        let mut sessions = state.mcp_sessions.write().await;
        retain_live_mcp_sessions(&mut sessions);
        if sessions.len() >= MAX_MCP_SESSIONS {
            return mcp_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                id,
                -32000,
                "Rho has reached the active MCP session limit",
            );
        }
        sessions.insert(
            session_id.clone(),
            McpHttpSession {
                workspace_id,
                server: Arc::new(TokioMutex::new(server)),
                last_activity: Instant::now(),
            },
        );
        return mcp_json_response(StatusCode::OK, response, Some(&session_id));
    }

    let Some(session_id) = mcp_session_id(&headers) else {
        return mcp_error_response(
            StatusCode::BAD_REQUEST,
            id,
            -32001,
            "Mcp-Session-Id is required after initialize",
        );
    };
    let (session_workspace_id, server) = {
        let mut sessions = state.mcp_sessions.write().await;
        retain_live_mcp_sessions(&mut sessions);
        let Some(session) = sessions.get_mut(session_id) else {
            return mcp_error_response(
                StatusCode::NOT_FOUND,
                id,
                -32001,
                "MCP session was not found; initialize a new session",
            );
        };
        session.last_activity = Instant::now();
        (session.workspace_id.clone(), session.server.clone())
    };
    if session_workspace_id != workspace.workspace_id {
        return mcp_error_response(
            StatusCode::NOT_FOUND,
            id,
            -32004,
            "The MCP session's workspace is no longer active",
        );
    }
    let response = server.lock().await.handle(message).await;
    match response {
        Some(response) => mcp_json_response(StatusCode::OK, response, Some(session_id)),
        None => mcp_empty_response(StatusCode::ACCEPTED, Some(session_id)),
    }
}

async fn mcp_delete(State(state): State<ApiState>, headers: HeaderMap) -> Response {
    handle_mcp_delete(state, None, headers).await
}

async fn mcp_workspace_delete(
    AxumPath(workspace_id): AxumPath<String>,
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Response {
    handle_mcp_delete(state, Some(workspace_id), headers).await
}

async fn handle_mcp_delete(
    state: ApiState,
    requested_workspace_id: Option<String>,
    headers: HeaderMap,
) -> Response {
    let Some(session_id) = mcp_session_id(&headers) else {
        return mcp_error_response(
            StatusCode::BAD_REQUEST,
            Value::Null,
            -32001,
            "Mcp-Session-Id is required",
        );
    };
    let mut sessions = state.mcp_sessions.write().await;
    retain_live_mcp_sessions(&mut sessions);
    let Some(session_workspace_id) = sessions
        .get(session_id)
        .map(|session| session.workspace_id.clone())
    else {
        return mcp_error_response(
            StatusCode::NOT_FOUND,
            Value::Null,
            -32001,
            "MCP session was not found",
        );
    };
    if requested_workspace_id
        .as_deref()
        .is_some_and(|requested| requested != session_workspace_id)
    {
        return mcp_error_response(
            StatusCode::NOT_FOUND,
            Value::Null,
            -32004,
            "MCP session does not belong to this workspace",
        );
    }
    sessions.remove(session_id);
    mcp_empty_response(StatusCode::NO_CONTENT, None)
}

fn retain_live_mcp_sessions(sessions: &mut HashMap<String, McpHttpSession>) {
    let now = Instant::now();
    sessions.retain(|_, session| {
        now.saturating_duration_since(session.last_activity) <= MCP_SESSION_TTL
    });
}

fn is_json_content_type(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
}

fn mcp_session_id(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(&MCP_SESSION_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
}

fn mcp_json_response(status: StatusCode, body: Value, session_id: Option<&str>) -> Response {
    let mut response = (status, Json(body)).into_response();
    if let Some(session_id) = session_id
        && let Ok(value) = HeaderValue::from_str(session_id)
    {
        response
            .headers_mut()
            .insert(MCP_SESSION_ID_HEADER.clone(), value);
    }
    response
}

fn mcp_empty_response(status: StatusCode, session_id: Option<&str>) -> Response {
    let mut response = status.into_response();
    if let Some(session_id) = session_id
        && let Ok(value) = HeaderValue::from_str(session_id)
    {
        response
            .headers_mut()
            .insert(MCP_SESSION_ID_HEADER.clone(), value);
    }
    response
}

fn mcp_error_response(status: StatusCode, id: Value, code: i64, message: &str) -> Response {
    mcp_json_response(
        status,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": code, "message": message}
        }),
        None,
    )
}

fn mcp_method_not_allowed() -> Response {
    let mut response = mcp_error_response(
        StatusCode::METHOD_NOT_ALLOWED,
        Value::Null,
        -32600,
        "This Rho MCP endpoint does not provide an SSE GET stream",
    );
    response
        .headers_mut()
        .insert("allow", HeaderValue::from_static("POST, DELETE"));
    response
}

pub async fn serve(listener: TcpListener, runtime: RuntimeService) -> Result<()> {
    axum::serve(listener, router(runtime))
        .await
        .context("serving Rho Workbench Protocol")
}

async fn health(State(state): State<ApiState>) -> Json<ApiResponse<RuntimeHealth>> {
    Json(ApiResponse::new(
        request_id(),
        runtime_health(&state.runtime).await,
    ))
}

async fn readiness(
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<RuntimeHealth>>, ApiFailure> {
    let request_id = request_id();
    let health = runtime_health(&state.runtime).await;
    if health.workspace_r_ready {
        Ok(Json(ApiResponse::new(request_id, health)))
    } else {
        Err(ApiFailure::unavailable(
            &request_id,
            "workspace_r_not_ready",
            "Workspace R execution is not ready; inspect /healthz for lifecycle details",
        ))
    }
}

async fn runtime_health(runtime: &RuntimeService) -> RuntimeHealth {
    let workspace = runtime.workspace().await;
    let dependency_report = runtime.dependency_report().await;
    let executor_attached = runtime.has_executor().await;
    let workspace_r_ready = executor_attached
        && matches!(
            workspace.lifecycle,
            WorkspaceLifecycle::Ready | WorkspaceLifecycle::Busy
        );
    RuntimeHealth {
        status: "ok".to_string(),
        control_plane_ready: true,
        workspace_id: workspace.workspace_id,
        project_root: workspace.project_root,
        lifecycle: workspace.lifecycle,
        executor_attached,
        workspace_r_ready,
        dependencies: DependencySummary::from(&dependency_report),
        connected_clients: runtime.client_sessions().await.len(),
    }
}

async fn runtime_dependencies(
    State(state): State<ApiState>,
) -> Json<ApiResponse<DependencyReport>> {
    Json(ApiResponse::new(
        request_id(),
        state.runtime.dependency_report().await,
    ))
}

async fn runtime_dependency_action(
    State(state): State<ApiState>,
    Json(request): Json<DependencyActionRequest>,
) -> Result<Json<ApiResponse<DependencyReport>>, ApiFailure> {
    let request_id = request_id();
    if request.action == "open_r_installer" {
        if !request.confirmed {
            return Err(ApiFailure::bad_request(
                &request_id,
                "dependency_confirmation_required",
                "Opening the operating-system R installer requires confirmed=true",
            ));
        }
        let manager = state.runtime.dependency_manager().await.ok_or_else(|| {
            ApiFailure::unavailable(
                &request_id,
                "dependency_manager_unavailable",
                "Rho Dependency Manager is not configured",
            )
        })?;
        manager
            .open_verified_r_installer()
            .await
            .map_err(|error| ApiFailure::internal(&request_id, error))?;
        return Ok(Json(ApiResponse::new(
            request_id,
            state.runtime.dependency_report().await,
        )));
    }
    let options = match request.action.as_str() {
        "ensure" => EnsureOptions {
            offline: request.offline,
            ..EnsureOptions::default()
        },
        "repair" if request.confirmed => EnsureOptions {
            offline: request.offline,
            repair: true,
            ..EnsureOptions::default()
        },
        "install_r" if request.confirmed => EnsureOptions {
            offline: request.offline,
            install_r: true,
            ..EnsureOptions::default()
        },
        "repair" | "install_r" => {
            return Err(ApiFailure::bad_request(
                &request_id,
                "dependency_confirmation_required",
                "This dependency action changes installed runtime components and requires confirmed=true",
            ));
        }
        _ => {
            return Err(ApiFailure::bad_request(
                &request_id,
                "unknown_dependency_action",
                "Dependency action must be ensure, repair, install_r, or open_r_installer",
            ));
        }
    };
    state
        .runtime
        .ensure_managed_workspace(options)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(
        request_id,
        state.runtime.dependency_report().await,
    )))
}

async fn schema() -> Json<ApiResponse<Value>> {
    Json(ApiResponse::new(request_id(), workbench_protocol_schema()))
}

async fn current_workspace(State(state): State<ApiState>) -> Json<ApiResponse<Workspace>> {
    Json(ApiResponse::new(
        request_id(),
        state.runtime.workspace().await,
    ))
}

async fn workspace(
    AxumPath(workspace_id): AxumPath<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Workspace>>, ApiFailure> {
    let request_id = request_id();
    let workspace = require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    Ok(Json(ApiResponse::new(request_id, workspace)))
}

async fn list_events(
    AxumPath(workspace_id): AxumPath<String>,
    Query(query): Query<EventQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<EventPage>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let after = query.after.unwrap_or_default();
    let limit = query
        .limit
        .unwrap_or(DEFAULT_EVENT_PAGE)
        .clamp(1, MAX_EVENT_PAGE);
    let events = state
        .runtime
        .replay_events(after, Some(limit))
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    let next_after = events.last().map_or(after, |event| event.sequence);
    Ok(Json(ApiResponse::new(
        request_id,
        EventPage { events, next_after },
    )))
}

async fn list_runs(
    AxumPath(workspace_id): AxumPath<String>,
    Query(query): Query<ListQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<ProtocolRun>>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let runs = state
        .runtime
        .list_runs(query.limit)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?
        .into_iter()
        .map(|run| protocol_run(run, &workspace_id))
        .collect::<Result<Vec<_>>>()
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(request_id, runs)))
}

async fn execute_run(
    AxumPath(workspace_id): AxumPath<String>,
    State(state): State<ApiState>,
    Json(request): Json<ExecuteRunRequest>,
) -> Result<Json<ApiResponse<ProtocolRun>>, ApiFailure> {
    let request_id = request_id();
    let run = execute_runtime_request(&state.runtime, &workspace_id, request, &request_id).await?;
    Ok(Json(ApiResponse::new(request_id, run)))
}

async fn execute_runtime_request(
    runtime: &RuntimeService,
    workspace_id: &str,
    mut request: ExecuteRunRequest,
    request_id: &str,
) -> Result<ProtocolRun, ApiFailure> {
    let workspace = require_workspace(runtime, workspace_id, request_id).await?;
    if request.code.trim().is_empty() {
        return Err(ApiFailure::bad_request(
            request_id,
            "empty_code",
            "R code must not be empty",
        ));
    }
    let client_kind = request.client_kind.unwrap_or(ClientKind::Cli);
    request.client_kind = Some(client_kind);
    let policy = PolicyEngine::evaluate(
        PolicyPrincipal::from_client_kind(client_kind),
        PolicyAction::ModifyObject,
    );
    if !policy.permits_execution() {
        return Err(match policy {
            PolicyDecision::RequireBrokerApproval => ApiFailure::approval_required(
                request_id,
                "approval_required",
                "Agent mutations require a live broker approval bound to the exact arguments",
            ),
            PolicyDecision::Deny => ApiFailure::forbidden(
                request_id,
                "policy_denied",
                "This client boundary is read-only",
            ),
            _ => ApiFailure::forbidden(
                request_id,
                "policy_denied",
                "The policy engine denied this execution",
            ),
        });
    }
    if !runtime.has_executor().await {
        let message = if workspace.lifecycle == WorkspaceLifecycle::Disconnected {
            "Workspace R is disconnected; start the configured Ark runtime before executing code"
        } else {
            "This runtime does not have an execution backend attached"
        };
        return Err(ApiFailure::unavailable(
            request_id,
            "runtime_execution_unavailable",
            message,
        ));
    }
    runtime
        .emit_event(
            rho_protocol::RuntimeEventKind::RunStarted,
            serde_json::json!({
                "client_kind": client_kind,
                "policy": policy.policy_name(),
                "source_path": request.source_path,
                "execution_mode": request.execution_mode
            }),
            Some(workspace.deep_link.clone()),
        )
        .await
        .map_err(|error| ApiFailure::internal(request_id, error))?;
    let execution = runtime
        .execute(request)
        .await
        .map_err(|error| ApiFailure::internal(request_id, error))?;
    let run = protocol_run_detail(execution.run, workspace_id)
        .map_err(|error| ApiFailure::internal(request_id, error))?;
    Ok(run)
}

#[derive(Clone)]
struct RuntimeMcpBackend {
    runtime: RuntimeService,
    workspace_id: String,
}

impl RuntimeMcpBackend {
    async fn bound_workspace(&self) -> Result<Workspace> {
        let workspace = self.runtime.workspace().await;
        ensure!(
            workspace.workspace_id == self.workspace_id,
            "Workspace `{}` is no longer active",
            self.workspace_id
        );
        Ok(workspace)
    }
}

#[async_trait]
impl McpBackend for RuntimeMcpBackend {
    async fn status(&self) -> Result<Workspace> {
        self.bound_workspace().await
    }

    async fn execute(&self, request: ExecuteRunRequest) -> Result<ProtocolRun> {
        self.bound_workspace().await?;
        execute_runtime_request(&self.runtime, &self.workspace_id, request, &request_id())
            .await
            .map_err(api_failure_error)
    }

    async fn objects(&self) -> Result<Vec<ProtocolObject>> {
        self.bound_workspace().await?;
        self.runtime.scientific_objects().await
    }

    async fn inspect_object(&self, object: &str) -> Result<ProtocolObject> {
        self.bound_workspace().await?;
        self.runtime
            .inspect_object(object)
            .await?
            .with_context(|| format!("Object `{object}` was not found"))
    }

    async fn runs(&self) -> Result<Vec<ProtocolRun>> {
        self.bound_workspace().await?;
        self.runtime
            .list_runs(None)
            .await?
            .into_iter()
            .map(|run| protocol_run(run, &self.workspace_id))
            .collect()
    }

    async fn problems(&self) -> Result<Vec<Problem>> {
        self.bound_workspace().await?;
        self.runtime
            .list_problems(None)
            .await?
            .into_iter()
            .map(|problem| protocol_problem(problem, &self.workspace_id))
            .collect()
    }

    async fn plots(&self) -> Result<Vec<Artifact>> {
        self.bound_workspace().await?;
        self.runtime
            .list_plot_artifacts(None)
            .await?
            .into_iter()
            .map(|plot| protocol_plot(plot, &self.workspace_id))
            .collect()
    }
}

fn api_failure_error(failure: ApiFailure) -> anyhow::Error {
    anyhow::anyhow!(
        "Rho API {} (HTTP {}): {} [retryable={}]",
        failure.body.code,
        failure.status.as_u16(),
        failure.body.message,
        failure.body.retryable
    )
}

async fn list_problems(
    AxumPath(workspace_id): AxumPath<String>,
    Query(query): Query<ListQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<Problem>>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let problems = state
        .runtime
        .list_problems(query.limit)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?
        .into_iter()
        .map(|problem| protocol_problem(problem, &workspace_id))
        .collect::<Result<Vec<_>>>()
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(request_id, problems)))
}

async fn list_plots(
    AxumPath(workspace_id): AxumPath<String>,
    Query(query): Query<ListQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<Artifact>>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let plots = state
        .runtime
        .list_plot_artifacts(query.limit)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?
        .into_iter()
        .map(|plot| protocol_plot(plot, &workspace_id))
        .collect::<Result<Vec<_>>>()
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(request_id, plots)))
}

async fn list_approvals(
    AxumPath(workspace_id): AxumPath<String>,
    Query(query): Query<ListQuery>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<Approval>>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let approvals = state
        .runtime
        .list_approvals(query.limit, query.status.as_deref())
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?
        .into_iter()
        .map(|approval| protocol_approval(approval, &workspace_id))
        .collect::<Result<Vec<_>>>()
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(request_id, approvals)))
}

async fn list_objects(
    AxumPath(workspace_id): AxumPath<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Vec<ProtocolObject>>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let objects = state
        .runtime
        .scientific_objects()
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(request_id, objects)))
}

async fn get_object(
    AxumPath((workspace_id, object_id)): AxumPath<(String, String)>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<ProtocolObject>>, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    match state
        .runtime
        .inspect_object(&object_id)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?
    {
        Some(object) => Ok(Json(ApiResponse::new(request_id, object))),
        None => Err(ApiFailure::not_found(
            &request_id,
            "object_not_found",
            format!("Object `{object_id}` is not present in the current workspace snapshot"),
        )),
    }
}

async fn get_provenance(
    AxumPath(workspace_id): AxumPath<String>,
    State(state): State<ApiState>,
) -> Result<Json<ApiResponse<Provenance>>, ApiFailure> {
    let request_id = request_id();
    let workspace = require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let runs = state
        .runtime
        .list_runs(None)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    let plots = state
        .runtime
        .list_plot_artifacts(None)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    let objects = state.runtime.cached_scientific_objects().await;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for summary in runs {
        let detail = state
            .runtime
            .get_run(&summary.run_id)
            .await
            .map_err(|error| ApiFailure::internal(&request_id, error))?
            .with_context(|| format!("run `{}` disappeared", summary.run_id))
            .map_err(|error| ApiFailure::internal(&request_id, error))?;
        let node_id = run_node_id(&summary.run_id);
        nodes.push(ProvenanceNode {
            node_id: node_id.clone(),
            workspace_id: workspace_id.clone(),
            kind: ProvenanceNodeKind::Run,
            label: format!("{} · {}", summary.request_type, summary.status),
            attributes: serde_json::json!({
                "run_id": summary.run_id,
                "origin": summary.origin,
                "status": summary.status,
                "operation_class": summary.operation_class,
                "code": detail.code,
                "parameters": serde_json::from_str::<Value>(&detail.arguments_json).unwrap_or(Value::Null),
                "environment": {
                    "workspace_id": workspace.workspace_id,
                    "kernel_instance_id": workspace.identity.kernel_instance_id,
                    "project_root": workspace.project_root
                },
                "source_path": detail.source_path,
                "execution_mode": detail.execution_mode,
                "document_version": detail.document_version,
                "started_at": detail.started_at,
                "finished_at": detail.finished_at,
                "execution_time_ms": elapsed_milliseconds(&detail.started_at, detail.finished_at.as_deref()),
                "state_revision_before": detail.state_revision_before,
                "state_revision_after": detail.state_revision_after,
                "project_revision_before": detail.project_revision_before,
                "project_revision_after": detail.project_revision_after
            }),
            deep_link: DeepLink::run(&workspace_id, &summary.run_id)
                .map_err(|error| ApiFailure::internal(&request_id, error))?,
        });
        if let Some(parent_run_id) = summary.parent_run_id {
            edges.push(ProvenanceEdge {
                edge_id: format!("edge_{}", edges.len() + 1),
                from_node_id: run_node_id(&parent_run_id),
                to_node_id: node_id,
                relation: "derived_from".to_string(),
                run_id: Some(summary.run_id),
            });
        }
    }

    for plot in plots {
        let node_id = artifact_node_id(&plot.plot_id);
        nodes.push(ProvenanceNode {
            node_id: node_id.clone(),
            workspace_id: workspace_id.clone(),
            kind: ProvenanceNodeKind::Figure,
            label: format!("{} · {}", plot.plot_id, plot.media_type),
            attributes: serde_json::json!({
                "artifact_id": plot.plot_id,
                "media_type": plot.media_type,
                "source_path": plot.source_path,
                "execution_mode": plot.execution_mode,
                "document_version": plot.document_version,
                "state_revision": plot.state_revision,
                "project_revision": plot.project_revision,
                "provenance_complete": plot.provenance_complete,
                "created_at": plot.created_at
            }),
            deep_link: DeepLink::artifact(&workspace_id, &plot.plot_id)
                .map_err(|error| ApiFailure::internal(&request_id, error))?,
        });
        edges.push(ProvenanceEdge {
            edge_id: format!("edge_{}", edges.len() + 1),
            from_node_id: run_node_id(&plot.run_id),
            to_node_id: node_id,
            relation: "produced".to_string(),
            run_id: Some(plot.run_id),
        });
    }

    for object in objects {
        let node_id = format!("object:{}", object.object_id);
        nodes.push(ProvenanceNode {
            node_id: node_id.clone(),
            workspace_id: workspace_id.clone(),
            kind: semantic_object_kind(&object),
            label: object.name.clone(),
            attributes: serde_json::json!({
                "object_id": object.object_id,
                "r_type": object.r_type,
                "class": object.class,
                "dimensions": object.dimensions,
                "metadata": object.metadata
            }),
            deep_link: object.deep_link,
        });
        for run_id in object.lineage {
            edges.push(ProvenanceEdge {
                edge_id: format!("edge_{}", edges.len() + 1),
                from_node_id: run_node_id(&run_id),
                to_node_id: node_id.clone(),
                relation: "observed_as".to_string(),
                run_id: Some(run_id),
            });
        }
        for artifact_id in object.related_artifacts {
            edges.push(ProvenanceEdge {
                edge_id: format!("edge_{}", edges.len() + 1),
                from_node_id: node_id.clone(),
                to_node_id: artifact_node_id(&artifact_id),
                relation: "related_artifact".to_string(),
                run_id: None,
            });
        }
    }

    Ok(Json(ApiResponse::new(
        request_id,
        Provenance {
            workspace_id,
            nodes,
            edges,
            revision: workspace.identity.execution_seq,
        },
    )))
}

fn run_node_id(run_id: &str) -> String {
    format!("run:{run_id}")
}

fn artifact_node_id(artifact_id: &str) -> String {
    format!("artifact:{artifact_id}")
}

fn elapsed_milliseconds(started_at: &str, finished_at: Option<&str>) -> Option<i64> {
    let start = chrono::DateTime::parse_from_rfc3339(started_at).ok()?;
    let finish = chrono::DateTime::parse_from_rfc3339(finished_at?).ok()?;
    Some(finish.signed_duration_since(start).num_milliseconds())
}

fn semantic_object_kind(object: &ProtocolObject) -> ProvenanceNodeKind {
    if object.class.iter().any(|class| class == "ggplot") {
        ProvenanceNodeKind::Figure
    } else if object
        .class
        .iter()
        .any(|class| matches!(class.as_str(), "lm" | "glm" | "nls" | "randomForest"))
    {
        ProvenanceNodeKind::Model
    } else if object.metadata["preview_kind"] == "tabular" {
        ProvenanceNodeKind::Table
    } else {
        ProvenanceNodeKind::Object
    }
}

async fn workspace_events_ws(
    ws: WebSocketUpgrade,
    AxumPath(workspace_id): AxumPath<String>,
    Query(query): Query<EventQuery>,
    State(state): State<ApiState>,
) -> Result<Response, ApiFailure> {
    let request_id = request_id();
    require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    let after = query.after.unwrap_or_default();
    let client_kind = parse_client_kind(query.client.as_deref())
        .map_err(|message| ApiFailure::bad_request(&request_id, "invalid_client_kind", message))?;
    Ok(ws
        .on_upgrade(move |socket| handle_socket(socket, state.runtime, client_kind, after))
        .into_response())
}

async fn handle_socket(
    socket: WebSocket,
    runtime: RuntimeService,
    client_kind: ClientKind,
    after: u64,
) {
    let session = runtime.connect_client(client_kind).await;
    let connection_id = session.connection_id.clone();
    let mut events = runtime.subscribe();
    let _ = run_socket(socket, &runtime, session, &mut events, after).await;
    runtime.disconnect_client(&connection_id).await;
}

async fn run_socket(
    mut socket: WebSocket,
    runtime: &RuntimeService,
    session: rho_protocol::ClientSession,
    events: &mut broadcast::Receiver<rho_protocol::RuntimeEvent>,
    after: u64,
) -> Result<()> {
    send_message(&mut socket, &EventStreamMessage::Session(session)).await?;
    let mut last_sequence = after;
    for event in runtime.replay_events(after, Some(MAX_EVENT_PAGE)).await? {
        last_sequence = last_sequence.max(event.sequence);
        send_message(&mut socket, &EventStreamMessage::Event(event)).await?;
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Ping(payload))) => socket.send(Message::Pong(payload)).await?,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
            event = events.recv() => {
                match event {
                    Ok(event) if event.sequence > last_sequence => {
                        last_sequence = event.sequence;
                        send_message(&mut socket, &EventStreamMessage::Event(event)).await?;
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        for event in runtime.replay_events(last_sequence, Some(MAX_EVENT_PAGE)).await? {
                            last_sequence = last_sequence.max(event.sequence);
                            send_message(&mut socket, &EventStreamMessage::Event(event)).await?;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    Ok(())
}

async fn send_message<T: Serialize>(socket: &mut WebSocket, message: &T) -> Result<()> {
    let payload = serde_json::to_string(message).context("encoding WebSocket event")?;
    socket
        .send(Message::Text(payload.into()))
        .await
        .context("sending WebSocket event")
}

fn protocol_run(summary: rho_store::RunSummary, workspace_id: &str) -> Result<ProtocolRun> {
    Ok(ProtocolRun {
        deep_link: DeepLink::run(workspace_id, &summary.run_id)?,
        run_id: summary.run_id,
        parent_run_id: summary.parent_run_id,
        workspace_id: summary
            .workspace_id
            .unwrap_or_else(|| workspace_id.to_string()),
        origin: summary.origin,
        status: summary.status,
        request_type: summary.request_type,
        operation_class: summary.operation_class,
        code: None,
        started_at: summary.started_at,
        finished_at: summary.finished_at,
        state_revision_before: nonnegative_revision(summary.state_revision_before),
        state_revision_after: nonnegative_revision(summary.state_revision_after),
        project_revision_before: nonnegative_revision(summary.project_revision_before),
        project_revision_after: nonnegative_revision(summary.project_revision_after),
    })
}

fn protocol_run_detail(detail: rho_store::RunDetail, workspace_id: &str) -> Result<ProtocolRun> {
    Ok(ProtocolRun {
        deep_link: DeepLink::run(workspace_id, &detail.run_id)?,
        run_id: detail.run_id,
        parent_run_id: detail.parent_run_id,
        workspace_id: detail
            .workspace_id
            .unwrap_or_else(|| workspace_id.to_string()),
        origin: detail.origin,
        status: detail.status,
        request_type: detail.request_type,
        operation_class: detail.operation_class,
        code: Some(detail.code),
        started_at: detail.started_at,
        finished_at: detail.finished_at,
        state_revision_before: nonnegative_revision(detail.state_revision_before),
        state_revision_after: nonnegative_revision(detail.state_revision_after),
        project_revision_before: nonnegative_revision(detail.project_revision_before),
        project_revision_after: nonnegative_revision(detail.project_revision_after),
    })
}

fn protocol_problem(summary: rho_store::ProblemSummary, workspace_id: &str) -> Result<Problem> {
    let problem_id = format!("problem_{}", summary.run_id);
    Ok(Problem {
        deep_link: DeepLink::problem(workspace_id, &problem_id)?,
        problem_id,
        workspace_id: summary
            .workspace_id
            .unwrap_or_else(|| workspace_id.to_string()),
        run_id: Some(summary.run_id),
        severity: ProblemSeverity::Error,
        message: summary.message,
        call: summary.call,
        traceback: summary.traceback,
        source_path: summary.source_path,
        created_at: summary.finished_at.unwrap_or(summary.started_at),
    })
}

fn protocol_plot(summary: rho_store::PlotArtifactSummary, workspace_id: &str) -> Result<Artifact> {
    let payload = serde_json::from_str(&summary.payload_json)
        .unwrap_or_else(|_| Value::String(summary.payload_json.clone()));
    Ok(Artifact {
        deep_link: DeepLink::artifact(workspace_id, &summary.plot_id)?,
        uri: format!("/v1/workspaces/{workspace_id}/plots/{}", summary.plot_id),
        artifact_id: summary.plot_id,
        workspace_id: summary
            .workspace_id
            .unwrap_or_else(|| workspace_id.to_string()),
        run_id: Some(summary.run_id),
        kind: ArtifactKind::Plot,
        media_type: summary.media_type,
        metadata: serde_json::json!({
            "payload": payload,
            "source_path": summary.source_path,
            "execution_mode": summary.execution_mode,
            "document_version": summary.document_version,
            "state_revision": summary.state_revision,
            "project_revision": summary.project_revision,
            "provenance_complete": summary.provenance_complete
        }),
        created_at: summary.created_at,
    })
}

fn protocol_approval(
    summary: rho_store::ApprovalRequestSummary,
    workspace_id: &str,
) -> Result<Approval> {
    let status = match summary.status.as_str() {
        "waiting" => ApprovalStatus::Waiting,
        "approved" => ApprovalStatus::Approved,
        "rejected" => ApprovalStatus::Rejected,
        "cancelled" | "canceled" => ApprovalStatus::Cancelled,
        "interrupted" => ApprovalStatus::Interrupted,
        value => anyhow::bail!("unsupported stored approval status `{value}`"),
    };
    Ok(Approval {
        deep_link: DeepLink::approval(workspace_id, &summary.request_id)?,
        approval_id: summary.request_id,
        workspace_id: summary
            .workspace_id
            .unwrap_or_else(|| workspace_id.to_string()),
        run_id: None,
        action: summary.tool,
        policy: summary.policy,
        status,
        arguments: serde_json::from_str(&summary.arguments_json)
            .context("decoding stored approval arguments")?,
        requested_at: summary.requested_at,
        responded_at: summary.responded_at,
    })
}

fn nonnegative_revision(value: Option<i64>) -> Option<u64> {
    value.and_then(|value| u64::try_from(value).ok())
}

async fn require_workspace(
    runtime: &RuntimeService,
    workspace_id: &str,
    request_id: &str,
) -> Result<Workspace, ApiFailure> {
    let workspace = runtime.workspace().await;
    if workspace.workspace_id == workspace_id {
        Ok(workspace)
    } else {
        Err(ApiFailure::not_found(
            request_id,
            "workspace_not_found",
            format!("Workspace `{workspace_id}` does not exist"),
        ))
    }
}

fn parse_client_kind(value: Option<&str>) -> Result<ClientKind, String> {
    match value.unwrap_or("web") {
        "web" => Ok(ClientKind::Web),
        "cli" => Ok(ClientKind::Cli),
        "mcp" => Ok(ClientKind::Mcp),
        "desktop" => Ok(ClientKind::Desktop),
        "agent" => Ok(ClientKind::Agent),
        value => Err(format!("Unsupported client kind `{value}`")),
    }
}

fn request_id() -> String {
    format!("request_{}", Uuid::new_v4().simple())
}

struct ApiFailure {
    status: StatusCode,
    body: ApiError,
}

impl ApiFailure {
    fn bad_request(request_id: &str, code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, request_id, code, message, false)
    }

    fn not_found(request_id: &str, code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, request_id, code, message, false)
    }

    fn unavailable(request_id: &str, code: &str, message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            request_id,
            code,
            message,
            true,
        )
    }

    fn forbidden(request_id: &str, code: &str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, request_id, code, message, false)
    }

    fn approval_required(request_id: &str, code: &str, message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::PRECONDITION_REQUIRED,
            request_id,
            code,
            message,
            true,
        )
    }

    fn internal(request_id: &str, error: impl std::fmt::Display) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            request_id,
            "internal_error",
            error.to_string(),
            true,
        )
    }

    fn new(
        status: StatusCode,
        request_id: &str,
        code: &str,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            status,
            body: ApiError {
                protocol_version: rho_protocol::WORKBENCH_PROTOCOL_VERSION.to_string(),
                request_id: request_id.to_string(),
                code: code.to_string(),
                message: message.into(),
                retryable,
            },
        }
    }
}

impl IntoResponse for ApiFailure {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use rho_core::ExecutionOrigin;
    use rho_protocol::{OperationClass, WorkspaceIdentity, WorkspaceLifecycle};
    use rho_store::{PlotArtifactDraft, RunDetail, RunDraft, RunFinish, Store};
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    use crate::execution::{WorkspaceExecution, WorkspaceExecutor};

    struct FakeExecutor {
        identity: Mutex<WorkspaceIdentity>,
    }

    impl FakeExecutor {
        fn new(identity: WorkspaceIdentity) -> Self {
            Self {
                identity: Mutex::new(identity),
            }
        }
    }

    #[async_trait]
    impl WorkspaceExecutor for FakeExecutor {
        async fn identity(&self) -> anyhow::Result<WorkspaceIdentity> {
            Ok(self.identity.lock().await.clone())
        }

        async fn dispatch(
            &self,
            request_type: &str,
            arguments: Value,
            origin: ExecutionOrigin,
            expected: WorkspaceIdentity,
        ) -> anyhow::Result<WorkspaceExecution> {
            let mut identity = self.identity.lock().await;
            assert_eq!(*identity, expected);
            let before = identity.clone();
            let (operation_class, operation_name, code, result, run_id) = match request_type {
                "workspace.execute" => {
                    assert_eq!(origin, ExecutionOrigin::User);
                    (
                        OperationClass::StateCapable,
                        "state_capable",
                        arguments["code"].as_str().unwrap().to_string(),
                        json!({"execution": {"ok": true, "value": "[1] 2"}}),
                        "exec_fake",
                    )
                }
                "workspace.snapshot" => (
                    OperationClass::Probe,
                    "probe",
                    "snapshot workspace".to_string(),
                    json!({
                        "execution": {
                            "ok": true,
                            "objects": [{
                                "name": "data",
                                "classes": "data.frame",
                                "dimensions": [3, 2],
                                "size_bytes": 1024,
                                "typeof": "list",
                                "preview_kind": "tabular"
                            }]
                        }
                    }),
                    "exec_fake_snapshot",
                ),
                "workspace.inspect_object" => (
                    OperationClass::Probe,
                    "probe",
                    format!("inspect {}", arguments["name"].as_str().unwrap()),
                    json!({
                        "execution": {
                            "ok": true,
                            "name": "data",
                            "classes": "data.frame",
                            "dimensions": [3, 2],
                            "size_bytes": 1024,
                            "typeof": "list",
                            "preview_kind": "tabular",
                            "preview": {"kind": "tabular", "rows": [{"a": 1}]},
                            "structure": "data.frame: 3 obs. of 2 variables"
                        }
                    }),
                    "exec_fake_inspect",
                ),
                value => anyhow::bail!("unexpected fake request `{value}`"),
            };
            identity.apply(operation_class);
            let now = "2026-07-20T00:00:00Z".to_string();
            Ok(WorkspaceExecution {
                run: RunDetail {
                    run_id: run_id.to_string(),
                    parent_run_id: None,
                    origin: "user".to_string(),
                    status: "completed".to_string(),
                    started_at: now.clone(),
                    finished_at: Some(now),
                    terminal_reason: None,
                    request_type: request_type.to_string(),
                    operation_class: operation_name.to_string(),
                    code,
                    arguments_json: arguments.to_string(),
                    source_path: None,
                    execution_mode: None,
                    document_version: None,
                    workspace_id: Some(identity.workspace_id.clone()),
                    state_revision_before: Some(before.state_revision as i64),
                    project_revision_before: Some(before.project_revision as i64),
                    state_revision_after: Some(identity.state_revision as i64),
                    project_revision_after: Some(identity.project_revision as i64),
                    stdout: None,
                    value_text: Some("[1] 2".to_string()),
                    messages: Vec::new(),
                    warnings: Vec::new(),
                    error_message: None,
                    error_call: None,
                    traceback: Vec::new(),
                },
                result,
                identity: identity.clone(),
            })
        }
    }

    fn test_runtime() -> (TempDir, RuntimeService) {
        let directory = TempDir::new().unwrap();
        let runtime = RuntimeService::open(
            crate::runtime::RuntimeServiceConfig::new(directory.path().join("runtime.sqlite"))
                .with_workspace_id("ws_api")
                .with_project_root(directory.path()),
        )
        .unwrap();
        (directory, runtime)
    }

    async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    async fn post_json(app: Router, uri: &str, value: Value) -> (StatusCode, Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(value.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    async fn mcp_request(
        app: Router,
        uri: &str,
        body: impl Into<Body>,
        session_id: Option<&str>,
    ) -> (StatusCode, HeaderMap, Option<Value>) {
        let mut request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream");
        if let Some(session_id) = session_id {
            request = request.header(&MCP_SESSION_ID_HEADER, session_id);
        }
        let response = app
            .oneshot(request.body(body.into()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value = (!body.is_empty()).then(|| serde_json::from_slice(&body).unwrap());
        (status, headers, value)
    }

    #[tokio::test]
    async fn streamable_http_mcp_initializes_lists_tools_and_opens_the_workspace() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);
        let (status, headers, initialize) = mcp_request(
            app.clone(),
            "/mcp",
            Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": "api-test", "version": "1"}
                    }
                })
                .to_string(),
            ),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let session_id = headers[MCP_SESSION_ID_HEADER].to_str().unwrap();
        let initialize = initialize.unwrap();
        assert_eq!(initialize["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(initialize["result"]["serverInfo"]["name"], "rho-mcp");

        let (status, headers, initialized) = mcp_request(
            app.clone(),
            "/mcp",
            Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                    "params": {}
                })
                .to_string(),
            ),
            Some(session_id),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(headers[MCP_SESSION_ID_HEADER], session_id);
        assert!(initialized.is_none());

        let (status, _, tools) = mcp_request(
            app.clone(),
            "/mcp",
            Body::from(
                json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
                    .to_string(),
            ),
            Some(session_id),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let tools = tools.unwrap();
        let names = tools["result"]["tools"]
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

        let (status, _, workspace) = mcp_request(
            app,
            "/mcp",
            Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {"name": "workspace_open", "arguments": {}}
                })
                .to_string(),
            ),
            Some(session_id),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let workspace = workspace.unwrap();
        assert_eq!(workspace["result"]["isError"], false);
        assert_eq!(
            workspace["result"]["structuredContent"]["data"]["workspace"]["workspace_id"],
            "ws_api"
        );
        assert_eq!(
            workspace["result"]["structuredContent"]["data"]["objects"],
            json!([])
        );
    }

    #[tokio::test]
    async fn streamable_http_mcp_rejects_wrong_workspaces_and_invalid_requests() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "api-test", "version": "1"}
            }
        });

        let (status, _, wrong_workspace) = mcp_request(
            app.clone(),
            "/v1/workspaces/ws_missing/mcp",
            Body::from(initialize.to_string()),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(wrong_workspace.unwrap()["error"]["code"], -32004);

        let (status, _, parse_error) =
            mcp_request(app.clone(), "/mcp", Body::from("not json"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(parse_error.unwrap()["error"]["code"], -32700);

        let (status, _, invalid_shape) =
            mcp_request(app.clone(), "/mcp", Body::from("[]"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(invalid_shape.unwrap()["error"]["code"], -32600);

        let (status, _, missing_session) = mcp_request(
            app,
            "/mcp",
            Body::from(
                json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
                    .to_string(),
            ),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(missing_session.unwrap()["error"]["code"], -32001);
    }

    #[tokio::test]
    async fn health_and_workspace_are_available_without_a_gui_client() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);
        let (status, health) = get(app.clone(), "/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(health["data"]["status"], "ok");
        assert_eq!(health["data"]["control_plane_ready"], true);
        assert_eq!(health["data"]["workspace_id"], "ws_api");
        assert!(health["data"]["project_root"].as_str().is_some());
        assert_eq!(health["data"]["lifecycle"], "disconnected");
        assert_eq!(health["data"]["executor_attached"], false);
        assert_eq!(health["data"]["workspace_r_ready"], false);
        assert_eq!(health["data"]["dependencies"]["ready"], false);
        assert_eq!(
            health["data"]["dependencies"]["issue_code"],
            "dependency.manager_not_configured"
        );
        assert_eq!(health["data"]["connected_clients"], 0);

        let (status, workspace) = get(app, "/v1/workspaces/current").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(workspace["data"]["workspace_id"], "ws_api");
    }

    #[tokio::test]
    async fn dependency_endpoint_is_structured_without_a_manager() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);

        let (status, report) = get(app.clone(), "/v1/runtime/dependencies").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(report["data"]["schema_version"], "1");
        assert_eq!(report["data"]["status"], "action_required");
        assert_eq!(report["data"]["ready"], false);
        assert_eq!(
            report["data"]["issue"]["code"],
            "dependency.manager_not_configured"
        );

        let (status, error) = post_json(
            app.clone(),
            "/v1/runtime/dependencies",
            json!({"action": "repair"}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["code"], "dependency_confirmation_required");

        let (status, error) = post_json(
            app,
            "/v1/runtime/dependencies",
            json!({"action": "launch_an_unmanaged_r"}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["code"], "unknown_dependency_action");
    }

    #[tokio::test]
    async fn readiness_requires_an_attached_ready_or_busy_executor() {
        let (_directory, runtime) = test_runtime();

        let (status, unavailable) = get(router(runtime.clone()), "/readyz").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(unavailable["code"], "workspace_r_not_ready");
        assert_eq!(unavailable["retryable"], true);

        let identity = runtime.workspace().await.identity;
        runtime
            .attach_executor(Arc::new(FakeExecutor::new(identity)))
            .await
            .unwrap();
        let (status, ready) = get(router(runtime.clone()), "/readyz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(ready["data"]["executor_attached"], true);
        assert_eq!(ready["data"]["workspace_r_ready"], true);
        assert_eq!(ready["data"]["lifecycle"], "ready");

        runtime
            .set_lifecycle(WorkspaceLifecycle::Busy)
            .await
            .unwrap();
        let (status, busy) = get(router(runtime.clone()), "/readyz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(busy["data"]["workspace_r_ready"], true);
        assert_eq!(busy["data"]["lifecycle"], "busy");

        runtime
            .set_lifecycle(WorkspaceLifecycle::Disconnected)
            .await
            .unwrap();
        let (status, unavailable) = get(router(runtime), "/readyz").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(unavailable["code"], "workspace_r_not_ready");
    }

    #[tokio::test]
    async fn browser_control_plane_is_served_by_the_runtime() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Runtime dependencies"));
        for page in [
            "Runs",
            "Objects",
            "Plots",
            "Problems",
            "Approvals",
            "Provenance",
        ] {
            assert!(html.contains(page), "missing browser page {page}");
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let script = String::from_utf8(body.to_vec()).unwrap();
        assert!(script.contains("/events/ws?client=web"));
        assert!(script.contains("/v1/runtime/dependencies"));
        assert!(script.contains("Start Workspace R"));
        assert!(script.contains("install Rho's official skill"));
        assert!(script.contains("renderProvenance"));
        assert!(script.contains("/agent-setup.md"));
    }

    #[tokio::test]
    async fn agent_setup_contract_and_official_skill_are_served() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/agent-setup.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[CONTENT_TYPE],
            "text/markdown; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let setup = String::from_utf8(body.to_vec()).unwrap();
        assert!(setup.contains("install Rho's official runtime skill"));
        assert!(setup.contains("workspace_open"));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/agent/skills/operate-rho-runtime/SKILL.md")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let skill = String::from_utf8(body.to_vec()).unwrap();
        assert!(skill.contains("name: operate-rho-runtime"));
        assert!(skill.contains("## Connect"));
    }

    #[tokio::test]
    async fn schema_endpoint_exports_the_workbench_contract() {
        let (_directory, runtime) = test_runtime();
        let (status, schema) = get(router(runtime), "/v1/schema").await;
        assert_eq!(status, StatusCode::OK);
        assert!(schema["data"]["entities"]["workspace"].is_object());
        assert!(schema["data"]["entities"]["runtime_event"].is_object());
    }

    #[tokio::test]
    async fn event_endpoint_supports_cursor_replay() {
        let (_directory, runtime) = test_runtime();
        runtime
            .set_lifecycle(WorkspaceLifecycle::Ready)
            .await
            .unwrap();
        runtime
            .emit_event(
                rho_protocol::RuntimeEventKind::RunStarted,
                json!({"run_id": "run_1"}),
                None,
            )
            .await
            .unwrap();
        let (status, page) = get(
            router(runtime),
            "/v1/workspaces/ws_api/events?after=1&limit=10",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["data"]["events"].as_array().unwrap().len(), 1);
        assert_eq!(page["data"]["events"][0]["sequence"], 2);
        assert_eq!(page["data"]["next_after"], 2);
    }

    #[tokio::test]
    async fn unknown_workspace_returns_a_stable_api_error() {
        let (_directory, runtime) = test_runtime();
        let (status, error) = get(router(runtime), "/v1/workspaces/ws_missing").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(error["code"], "workspace_not_found");
        assert_eq!(
            error["protocol_version"],
            rho_protocol::WORKBENCH_PROTOCOL_VERSION
        );
    }

    #[tokio::test]
    async fn scientific_collection_routes_are_machine_readable_when_empty() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);
        for path in ["runs", "problems", "plots", "approvals", "objects"] {
            let (status, response) =
                get(app.clone(), &format!("/v1/workspaces/ws_api/{path}")).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(response["data"], json!([]), "{path}");
        }
    }

    #[tokio::test]
    async fn execution_route_reports_a_disconnected_runtime_structurally() {
        let (_directory, runtime) = test_runtime();
        let response = router(runtime)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/workspaces/ws_api/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"1 + 1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let error: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(error["code"], "runtime_execution_unavailable");
        assert_eq!(error["retryable"], true);
    }

    #[tokio::test]
    async fn execution_route_uses_an_attached_workspace_executor() {
        let (_directory, runtime) = test_runtime();
        let identity = runtime.workspace().await.identity;
        runtime
            .attach_executor(Arc::new(FakeExecutor::new(identity)))
            .await
            .unwrap();
        let response = router(runtime.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/workspaces/ws_api/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"1 + 1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let run: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(run["data"]["run_id"], "exec_fake");
        assert_eq!(run["data"]["status"], "completed");
        assert_eq!(run["data"]["state_revision_before"], 0);
        assert_eq!(run["data"]["state_revision_after"], 1);
        let workspace = runtime.workspace().await;
        assert_eq!(workspace.lifecycle, WorkspaceLifecycle::Ready);
        assert_eq!(workspace.identity.state_revision, 1);
    }

    #[tokio::test]
    async fn object_routes_project_bounded_workspace_semantics() {
        let (_directory, runtime) = test_runtime();
        let identity = runtime.workspace().await.identity;
        runtime
            .attach_executor(Arc::new(FakeExecutor::new(identity)))
            .await
            .unwrap();
        let app = router(runtime);

        let (status, objects) = get(app.clone(), "/v1/workspaces/ws_api/objects").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(objects["data"][0]["object_id"], "object_64617461");
        assert_eq!(objects["data"][0]["class"], json!(["data.frame"]));
        assert_eq!(objects["data"][0]["dimensions"], json!([3, 2]));

        let (status, object) = get(app, "/v1/workspaces/ws_api/objects/object_64617461").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(object["data"]["name"], "data");
        assert_eq!(object["data"]["metadata"]["preview"]["kind"], "tabular");
        assert!(
            object["data"]["metadata"]["structure"]
                .as_str()
                .unwrap()
                .contains("3 obs")
        );
    }

    #[tokio::test]
    async fn provenance_graph_connects_runs_to_scientific_artifacts() {
        let directory = TempDir::new().unwrap();
        let store_path = directory.path().join("runtime.sqlite");
        let mut store = Store::open(&store_path).unwrap();
        let identity = WorkspaceIdentity::new("ws_provenance");
        store.save_identity(&identity).unwrap();
        store
            .create_run(&RunDraft {
                run_id: "exec_provenance".to_string(),
                parent_run_id: None,
                origin: "agent".to_string(),
                request_type: "workspace.execute".to_string(),
                operation_class: "state_capable".to_string(),
                code: "plot(1:3)".to_string(),
                arguments_json: json!({"code": "plot(1:3)", "temperature": 0}).to_string(),
                source_path: Some("analysis.R".to_string()),
                execution_mode: Some("source".to_string()),
                document_version: Some(3),
                workspace_id: "ws_provenance".to_string(),
                state_revision_before: 0,
                project_revision_before: 0,
            })
            .unwrap();
        store
            .finish_run(&RunFinish {
                run_id: "exec_provenance".to_string(),
                status: "completed".to_string(),
                terminal_reason: None,
                workspace_id: Some("ws_provenance".to_string()),
                state_revision_after: Some(1),
                project_revision_after: Some(0),
                stdout: None,
                value_text: None,
                messages: Vec::new(),
                warnings: Vec::new(),
                error_message: None,
                error_call: None,
                traceback: Vec::new(),
            })
            .unwrap();
        store
            .create_plot_artifact(&PlotArtifactDraft {
                plot_id: "plot_provenance".to_string(),
                run_id: "exec_provenance".to_string(),
                source_path: Some("analysis.R".to_string()),
                execution_mode: Some("source".to_string()),
                document_version: Some(3),
                workspace_id: Some("ws_provenance".to_string()),
                state_revision: Some(1),
                project_revision: Some(0),
                media_type: "image/svg+xml".to_string(),
                payload_json: json!({"data": "<svg/>"}).to_string(),
                provenance_complete: true,
            })
            .unwrap();
        drop(store);
        let runtime = RuntimeService::open(
            crate::runtime::RuntimeServiceConfig::new(store_path)
                .with_workspace_id("ws_provenance")
                .with_project_root(directory.path()),
        )
        .unwrap();

        let (status, graph) = get(router(runtime), "/v1/workspaces/ws_provenance/provenance").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(graph["data"]["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(graph["data"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(graph["data"]["edges"][0]["relation"], "produced");
        assert_eq!(graph["data"]["nodes"][0]["attributes"]["code"], "plot(1:3)");
        assert_eq!(
            graph["data"]["nodes"][0]["attributes"]["parameters"]["temperature"],
            0
        );
        assert_eq!(
            graph["data"]["nodes"][1]["attributes"]["provenance_complete"],
            true
        );
    }
}
