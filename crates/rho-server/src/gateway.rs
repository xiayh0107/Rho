use anyhow::{Context, Result, ensure};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{OriginalUri, State};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE, HOST};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, Url};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

const MAX_GATEWAY_BODY: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct GatewayState {
    upstream: Url,
    client: Client,
}

impl GatewayState {
    pub fn new(upstream: &str) -> Result<Self> {
        let mut upstream = Url::parse(upstream).context("parsing remote runtime URL")?;
        ensure!(
            matches!(upstream.scheme(), "http" | "https"),
            "remote runtime URL must use http or https"
        );
        ensure!(
            upstream.username().is_empty(),
            "URL credentials are not supported"
        );
        ensure!(
            upstream.password().is_none(),
            "URL credentials are not supported"
        );
        if !upstream.path().ends_with('/') {
            upstream.set_path(&format!("{}/", upstream.path()));
        }
        Ok(Self {
            upstream,
            client: Client::builder().build()?,
        })
    }

    fn target(&self, uri: &Uri) -> Result<Url> {
        let mut target = self
            .upstream
            .join(uri.path().trim_start_matches('/'))
            .context("joining remote runtime URL")?;
        target.set_query(uri.query());
        Ok(target)
    }
}

pub fn router(state: GatewayState) -> Router {
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
        .route(
            "/v1/workspaces/{workspace_id}/events/ws",
            get(proxy_websocket),
        )
        .fallback(proxy_http)
        .with_state(state)
}

pub async fn serve(listener: TcpListener, state: GatewayState) -> Result<()> {
    axum::serve(listener, router(state))
        .await
        .context("serving Rho remote runtime gateway")
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

async fn proxy_http(
    State(state): State<GatewayState>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, GatewayFailure> {
    let target = state
        .target(&uri)
        .map_err(|error| GatewayFailure::bad_gateway(error.to_string()))?;
    let body = to_bytes(body, MAX_GATEWAY_BODY)
        .await
        .map_err(|error| GatewayFailure::payload(error.to_string()))?;
    let mut request = state.client.request(method, target).body(body);
    for (name, value) in &headers {
        if !is_hop_by_hop(name.as_str()) && name != HOST && name != CONTENT_LENGTH {
            request = request.header(name, value);
        }
    }
    let upstream = request
        .send()
        .await
        .map_err(|error| GatewayFailure::bad_gateway(error.to_string()))?;
    let status = upstream.status();
    let headers = upstream.headers().clone();
    let bytes = upstream
        .bytes()
        .await
        .map_err(|error| GatewayFailure::bad_gateway(error.to_string()))?;
    let mut response = Response::builder().status(status);
    for (name, value) in &headers {
        if !is_hop_by_hop(name.as_str()) && name != CONTENT_LENGTH {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from(bytes))
        .map_err(|error| GatewayFailure::bad_gateway(error.to_string()))
}

async fn proxy_websocket(
    websocket: WebSocketUpgrade,
    State(state): State<GatewayState>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, GatewayFailure> {
    let mut target = state
        .target(&uri)
        .map_err(|error| GatewayFailure::bad_gateway(error.to_string()))?;
    let websocket_scheme = if target.scheme() == "https" {
        "wss"
    } else {
        "ws"
    };
    target
        .set_scheme(websocket_scheme)
        .map_err(|_| GatewayFailure::bad_gateway("invalid WebSocket URL"))?;
    Ok(websocket.on_upgrade(move |socket| async move {
        if let Ok((upstream, _)) = connect_async(target.as_str()).await {
            proxy_socket(socket, upstream).await;
        }
    }))
}

async fn proxy_socket(
    mut client: WebSocket,
    mut upstream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    loop {
        tokio::select! {
            message = client.next() => match message {
                Some(Ok(message)) => {
                    let Some(message) = to_upstream(message) else { break; };
                    if upstream.send(message).await.is_err() { break; }
                }
                _ => break,
            },
            message = upstream.next() => match message {
                Some(Ok(message)) => {
                    let Some(message) = to_client(message) else { break; };
                    if client.send(message).await.is_err() { break; }
                }
                _ => break,
            }
        }
    }
}

fn to_upstream(message: AxumMessage) -> Option<TungsteniteMessage> {
    match message {
        AxumMessage::Text(value) => Some(TungsteniteMessage::Text(value.to_string().into())),
        AxumMessage::Binary(value) => Some(TungsteniteMessage::Binary(value.to_vec().into())),
        AxumMessage::Ping(value) => Some(TungsteniteMessage::Ping(value.to_vec().into())),
        AxumMessage::Pong(value) => Some(TungsteniteMessage::Pong(value.to_vec().into())),
        AxumMessage::Close(_) => None,
    }
}

fn to_client(message: TungsteniteMessage) -> Option<AxumMessage> {
    match message {
        TungsteniteMessage::Text(value) => Some(AxumMessage::Text(value.to_string().into())),
        TungsteniteMessage::Binary(value) => Some(AxumMessage::Binary(value.to_vec().into())),
        TungsteniteMessage::Ping(value) => Some(AxumMessage::Ping(value.to_vec().into())),
        TungsteniteMessage::Pong(value) => Some(AxumMessage::Pong(value.to_vec().into())),
        TungsteniteMessage::Close(_) => None,
        TungsteniteMessage::Frame(_) => None,
    }
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

struct GatewayFailure {
    status: StatusCode,
    message: String,
}

impl GatewayFailure {
    fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: message.into(),
        }
    }

    fn payload(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: message.into(),
        }
    }
}

impl IntoResponse for GatewayFailure {
    fn into_response(self) -> Response {
        (
            self.status,
            axum::Json(json!({
                "code": "remote_runtime_unavailable",
                "message": self.message,
                "retryable": true
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rho_protocol::EventStreamMessage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn gateway_projects_remote_http_and_websocket_state() {
        let directory = TempDir::new().unwrap();
        let runtime = crate::runtime::RuntimeService::open(
            crate::runtime::RuntimeServiceConfig::new(directory.path().join("runtime.sqlite"))
                .with_workspace_id("ws_remote"),
        )
        .unwrap();
        let upstream_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_address = upstream_listener.local_addr().unwrap();
        let upstream_task = tokio::spawn(crate::api::serve(upstream_listener, runtime));

        let gateway_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway_address = gateway_listener.local_addr().unwrap();
        let state = GatewayState::new(&format!("http://{upstream_address}")).unwrap();
        let gateway_task = tokio::spawn(serve(gateway_listener, state));

        let workspace: serde_json::Value =
            reqwest::get(format!("http://{gateway_address}/v1/workspaces/current"))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
        assert_eq!(workspace["data"]["workspace_id"], "ws_remote");

        let setup = reqwest::get(format!("http://{gateway_address}/agent-setup.md"))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(setup.contains("# Connect an Agent to Rho"));

        let (mut socket, _) = connect_async(format!(
            "ws://{gateway_address}/v1/workspaces/ws_remote/events/ws?client=web"
        ))
        .await
        .unwrap();
        let message = socket.next().await.unwrap().unwrap();
        let TungsteniteMessage::Text(message) = message else {
            panic!("expected text session message");
        };
        let message: EventStreamMessage = serde_json::from_str(&message).unwrap();
        assert!(matches!(message, EventStreamMessage::Session(_)));

        gateway_task.abort();
        upstream_task.abort();
    }
}
