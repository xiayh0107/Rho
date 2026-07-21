use anyhow::{Context, Result};
use axum::Json;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use rho_protocol::{
    ApiError, ApiResponse, Approval, ApprovalStatus, Artifact, ArtifactKind, ClientKind, DeepLink,
    EventPage, EventStreamMessage, ExecuteRunRequest, Object as ProtocolObject, Problem,
    ProblemSeverity, Provenance, ProvenanceEdge, ProvenanceNode, ProvenanceNodeKind,
    Run as ProtocolRun, RuntimeHealth, Workspace, WorkspaceLifecycle, workbench_protocol_schema,
};
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::policy::{PolicyAction, PolicyDecision, PolicyEngine, PolicyPrincipal};
use crate::runtime::RuntimeService;

const MAX_EVENT_PAGE: usize = 1_000;
const DEFAULT_EVENT_PAGE: usize = 100;

#[derive(Clone)]
struct ApiState {
    runtime: RuntimeService,
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

pub fn router(runtime: RuntimeService) -> Router {
    Router::new()
        .route("/", get(web_index))
        .route("/app.js", get(web_javascript))
        .route("/styles.css", get(web_styles))
        .route("/healthz", get(health))
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
        .with_state(ApiState { runtime })
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

pub async fn serve(listener: TcpListener, runtime: RuntimeService) -> Result<()> {
    axum::serve(listener, router(runtime))
        .await
        .context("serving Rho Workbench Protocol")
}

async fn health(State(state): State<ApiState>) -> Json<ApiResponse<RuntimeHealth>> {
    let workspace = state.runtime.workspace().await;
    let connected_clients = state.runtime.client_sessions().await.len();
    Json(ApiResponse::new(
        request_id(),
        RuntimeHealth {
            status: "ok".to_string(),
            workspace_id: workspace.workspace_id,
            lifecycle: workspace.lifecycle,
            connected_clients,
        },
    ))
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
    Json(mut request): Json<ExecuteRunRequest>,
) -> Result<Json<ApiResponse<ProtocolRun>>, ApiFailure> {
    let request_id = request_id();
    let workspace = require_workspace(&state.runtime, &workspace_id, &request_id).await?;
    if request.code.trim().is_empty() {
        return Err(ApiFailure::bad_request(
            &request_id,
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
                &request_id,
                "approval_required",
                "Agent mutations require a live broker approval bound to the exact arguments",
            ),
            PolicyDecision::Deny => ApiFailure::forbidden(
                &request_id,
                "policy_denied",
                "This client boundary is read-only",
            ),
            _ => ApiFailure::forbidden(
                &request_id,
                "policy_denied",
                "The policy engine denied this execution",
            ),
        });
    }
    if !state.runtime.has_executor().await {
        let message = if workspace.lifecycle == WorkspaceLifecycle::Disconnected {
            "Workspace R is disconnected; start the configured Ark runtime before executing code"
        } else {
            "This runtime does not have an execution backend attached"
        };
        return Err(ApiFailure::unavailable(
            &request_id,
            "runtime_execution_unavailable",
            message,
        ));
    }
    state
        .runtime
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
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    let execution = state
        .runtime
        .execute(request)
        .await
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    let run = protocol_run_detail(execution.run, &workspace_id)
        .map_err(|error| ApiFailure::internal(&request_id, error))?;
    Ok(Json(ApiResponse::new(request_id, run)))
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
                .with_workspace_id("ws_api"),
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

    #[tokio::test]
    async fn health_and_workspace_are_available_without_a_gui_client() {
        let (_directory, runtime) = test_runtime();
        let app = router(runtime);
        let (status, health) = get(app.clone(), "/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(health["data"]["workspace_id"], "ws_api");
        assert_eq!(health["data"]["connected_clients"], 0);

        let (status, workspace) = get(app, "/v1/workspaces/current").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(workspace["data"]["workspace_id"], "ws_api");
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
        assert!(script.contains("renderProvenance"));
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
