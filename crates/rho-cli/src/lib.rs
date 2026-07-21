use anyhow::{Context, Result, bail, ensure};
use reqwest::{Client, Response, Url};
use rho_protocol::{
    ApiError, ApiResponse, Artifact, ExecuteRunRequest, Object, Problem, Provenance, Run,
    WORKBENCH_PROTOCOL_VERSION, Workspace,
};
use serde::de::DeserializeOwned;

pub mod bootstrap;

#[derive(Clone)]
pub struct RhoClient {
    base_url: Url,
    client: Client,
}

impl RhoClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let mut base_url = Url::parse(base_url).context("parsing Rho server URL")?;
        ensure!(
            matches!(base_url.scheme(), "http" | "https"),
            "Rho server URL must use http or https"
        );
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }
        Ok(Self {
            base_url,
            client: Client::new(),
        })
    }

    pub async fn status(&self) -> Result<Workspace> {
        self.get(self.endpoint(&["v1", "workspaces", "current"])?)
            .await
    }

    pub async fn execute(&self, request: &ExecuteRunRequest) -> Result<Run> {
        let workspace = self.status().await?;
        let url = self.endpoint(&["v1", "workspaces", &workspace.workspace_id, "runs"])?;
        decode(self.client.post(url).json(request).send().await?).await
    }

    pub async fn objects(&self) -> Result<Vec<Object>> {
        let workspace = self.status().await?;
        self.get(self.endpoint(&["v1", "workspaces", &workspace.workspace_id, "objects"])?)
            .await
    }

    pub async fn runs(&self) -> Result<Vec<Run>> {
        let workspace = self.status().await?;
        self.get(self.endpoint(&["v1", "workspaces", &workspace.workspace_id, "runs"])?)
            .await
    }

    pub async fn provenance(&self) -> Result<Provenance> {
        let workspace = self.status().await?;
        self.get(self.endpoint(&["v1", "workspaces", &workspace.workspace_id, "provenance"])?)
            .await
    }

    pub async fn inspect_object(&self, object_id: &str) -> Result<Object> {
        let workspace = self.status().await?;
        self.get(self.endpoint(&[
            "v1",
            "workspaces",
            &workspace.workspace_id,
            "objects",
            object_id,
        ])?)
        .await
    }

    pub async fn problems(&self) -> Result<Vec<Problem>> {
        let workspace = self.status().await?;
        self.get(self.endpoint(&["v1", "workspaces", &workspace.workspace_id, "problems"])?)
            .await
    }

    pub async fn plots(&self) -> Result<Vec<Artifact>> {
        let workspace = self.status().await?;
        self.get(self.endpoint(&["v1", "workspaces", &workspace.workspace_id, "plots"])?)
            .await
    }

    async fn get<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        decode(self.client.get(url).send().await?).await
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| anyhow::anyhow!("Rho server URL cannot be a base URL"))?;
            path.pop_if_empty();
            for segment in segments {
                path.push(segment);
            }
        }
        Ok(url)
    }
}

async fn decode<T: DeserializeOwned>(response: Response) -> Result<T> {
    let status = response.status();
    let bytes = response.bytes().await.context("reading Rho response")?;
    if !status.is_success() {
        if let Ok(error) = serde_json::from_slice::<ApiError>(&bytes) {
            bail!(
                "Rho API {} (HTTP {}): {} [retryable={}]",
                error.code,
                status.as_u16(),
                error.message,
                error.retryable
            );
        }
        bail!(
            "Rho API returned HTTP {}: {}",
            status.as_u16(),
            String::from_utf8_lossy(&bytes)
        );
    }
    let response: ApiResponse<T> =
        serde_json::from_slice(&bytes).context("decoding Rho API response")?;
    ensure!(
        response.protocol_version == WORKBENCH_PROTOCOL_VERSION,
        "Workbench Protocol mismatch: server={}, client={}",
        response.protocol_version,
        WORKBENCH_PROTOCOL_VERSION
    );
    Ok(response.data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rho_server::runtime::{RuntimeService, RuntimeServiceConfig};
    use tempfile::TempDir;

    #[test]
    fn endpoint_segments_are_percent_encoded() {
        let client = RhoClient::new("http://127.0.0.1:8787").unwrap();
        let url = client
            .endpoint(&["v1", "workspaces", "ws_1", "objects", "object with space"])
            .unwrap();
        assert_eq!(
            url.as_str(),
            "http://127.0.0.1:8787/v1/workspaces/ws_1/objects/object%20with%20space"
        );
    }

    #[tokio::test]
    async fn client_reads_runtime_state_and_empty_scientific_collections() {
        let directory = TempDir::new().unwrap();
        let runtime = RuntimeService::open(
            RuntimeServiceConfig::new(directory.path().join("runtime.sqlite"))
                .with_workspace_id("ws_cli"),
        )
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(rho_server::api::serve(listener, runtime));
        let client = RhoClient::new(&format!("http://{address}")).unwrap();

        assert_eq!(client.status().await.unwrap().workspace_id, "ws_cli");
        assert!(client.objects().await.unwrap().is_empty());
        assert!(client.problems().await.unwrap().is_empty());
        assert!(client.plots().await.unwrap().is_empty());

        server.abort();
    }

    #[tokio::test]
    async fn client_surfaces_structured_execution_unavailability() {
        let directory = TempDir::new().unwrap();
        let runtime = RuntimeService::open(
            RuntimeServiceConfig::new(directory.path().join("runtime.sqlite"))
                .with_workspace_id("ws_cli"),
        )
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(rho_server::api::serve(listener, runtime));
        let client = RhoClient::new(&format!("http://{address}")).unwrap();
        let error = client
            .execute(&ExecuteRunRequest {
                code: "1 + 1".to_string(),
                source_path: None,
                execution_mode: None,
                document_version: None,
                client_kind: Some(rho_protocol::ClientKind::Cli),
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("runtime_execution_unavailable"));
        assert!(error.to_string().contains("retryable=true"));

        server.abort();
    }
}
