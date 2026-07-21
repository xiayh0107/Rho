use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::WorkspaceIdentity;

/// The independently versioned HTTP/WebSocket contract exposed to clients.
pub const WORKBENCH_PROTOCOL_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceLifecycle {
    Starting,
    Ready,
    Busy,
    Restarting,
    Disconnected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Workspace {
    pub workspace_id: String,
    pub lifecycle: WorkspaceLifecycle,
    pub identity: WorkspaceIdentity,
    pub project_root: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deep_link: DeepLink,
}

/// Connection state is intentionally separate from persistent scientific state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ClientSession {
    pub connection_id: String,
    pub client_kind: ClientKind,
    pub connected_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    Web,
    Cli,
    Mcp,
    Desktop,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Run {
    pub run_id: String,
    pub parent_run_id: Option<String>,
    pub workspace_id: String,
    pub origin: String,
    pub status: String,
    pub request_type: String,
    pub operation_class: String,
    pub code: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub state_revision_before: Option<u64>,
    pub state_revision_after: Option<u64>,
    pub project_revision_before: Option<u64>,
    pub project_revision_after: Option<u64>,
    pub deep_link: DeepLink,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Object {
    pub object_id: String,
    pub workspace_id: String,
    pub name: String,
    pub r_type: String,
    pub class: Vec<String>,
    pub dimensions: Vec<u64>,
    pub metadata: Value,
    pub lineage: Vec<String>,
    pub related_artifacts: Vec<String>,
    pub deep_link: DeepLink,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Plot,
    Table,
    File,
    Model,
    Report,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Artifact {
    pub artifact_id: String,
    pub workspace_id: String,
    pub run_id: Option<String>,
    pub kind: ArtifactKind,
    pub media_type: String,
    pub uri: String,
    pub metadata: Value,
    pub created_at: String,
    pub deep_link: DeepLink,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProblemSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Problem {
    pub problem_id: String,
    pub workspace_id: String,
    pub run_id: Option<String>,
    pub severity: ProblemSeverity,
    pub message: String,
    pub call: Option<String>,
    pub traceback: Vec<String>,
    pub source_path: Option<String>,
    pub created_at: String,
    pub deep_link: DeepLink,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Waiting,
    Approved,
    Rejected,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Approval {
    pub approval_id: String,
    pub workspace_id: String,
    pub run_id: Option<String>,
    pub action: String,
    pub policy: String,
    pub status: ApprovalStatus,
    pub arguments: Value,
    pub requested_at: String,
    pub responded_at: Option<String>,
    pub deep_link: DeepLink,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceNodeKind {
    RawData,
    Object,
    Model,
    Table,
    Figure,
    Report,
    Run,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProvenanceNode {
    pub node_id: String,
    pub workspace_id: String,
    pub kind: ProvenanceNodeKind,
    pub label: String,
    pub attributes: Value,
    pub deep_link: DeepLink,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProvenanceEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub relation: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Provenance {
    pub workspace_id: String,
    pub nodes: Vec<ProvenanceNode>,
    pub edges: Vec<ProvenanceEdge>,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventKind {
    DependencyStatusChanged,
    WorkspaceLifecycleChanged,
    WorkspaceUpdated,
    RunStarted,
    RunOutput,
    RunFinished,
    ObjectChanged,
    ArtifactCreated,
    ProblemReported,
    ApprovalRequested,
    ApprovalResolved,
    ProvenanceChanged,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    #[default]
    Checking,
    ActionRequired,
    Preparing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyPhase {
    #[default]
    Idle,
    Discovering,
    Downloading,
    Verifying,
    Installing,
    GeneratingKernelspec,
    SmokeTesting,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyComponentStatus {
    Missing,
    CandidateFound,
    Incompatible,
    Downloading,
    Verifying,
    Installing,
    Ready,
    Invalid,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencySource {
    Explicit,
    System,
    Managed,
    Bundled,
    Downloaded,
    Embedded,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DependencyComponent {
    pub name: String,
    pub status: DependencyComponentStatus,
    pub requirement: Option<String>,
    pub version: Option<String>,
    pub source: Option<DependencySource>,
    pub path: Option<String>,
    pub verified: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DependencyAction {
    pub id: String,
    pub label: String,
    pub requires_human: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DependencyIssue {
    pub code: String,
    pub title: String,
    pub message: String,
    pub retryable: bool,
    pub requires_user_action: bool,
    pub action_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DependencyReport {
    pub schema_version: String,
    pub revision: u64,
    pub status: DependencyStatus,
    pub ready: bool,
    pub phase: DependencyPhase,
    pub managed_by: String,
    pub platform: String,
    pub components: Vec<DependencyComponent>,
    pub issue: Option<DependencyIssue>,
    pub available_actions: Vec<DependencyAction>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DependencySummary {
    pub ready: bool,
    pub status: DependencyStatus,
    pub phase: DependencyPhase,
    pub issue_code: Option<String>,
    pub requires_user_action: bool,
    pub action_url: Option<String>,
}

impl Default for DependencySummary {
    fn default() -> Self {
        Self {
            ready: false,
            status: DependencyStatus::Checking,
            phase: DependencyPhase::Idle,
            issue_code: None,
            requires_user_action: false,
            action_url: None,
        }
    }
}

impl From<&DependencyReport> for DependencySummary {
    fn from(report: &DependencyReport) -> Self {
        Self {
            ready: report.ready,
            status: report.status,
            phase: report.phase,
            issue_code: report.issue.as_ref().map(|issue| issue.code.clone()),
            requires_user_action: report
                .issue
                .as_ref()
                .is_some_and(|issue| issue.requires_user_action),
            action_url: report
                .issue
                .as_ref()
                .and_then(|issue| issue.action_url.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RuntimeEvent {
    pub sequence: u64,
    pub event_id: String,
    pub workspace_id: String,
    pub timestamp: String,
    pub kind: RuntimeEventKind,
    pub payload: Value,
    pub deep_link: Option<DeepLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RuntimeHealth {
    pub status: String,
    #[serde(default = "control_plane_ready_default")]
    pub control_plane_ready: bool,
    pub workspace_id: String,
    #[serde(default)]
    pub project_root: Option<String>,
    pub lifecycle: WorkspaceLifecycle,
    #[serde(default)]
    pub executor_attached: bool,
    #[serde(default)]
    pub workspace_r_ready: bool,
    #[serde(default)]
    pub dependencies: DependencySummary,
    pub connected_clients: usize,
}

fn control_plane_ready_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EventPage {
    pub events: Vec<RuntimeEvent>,
    pub next_after: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EventStreamMessage {
    Session(ClientSession),
    Event(RuntimeEvent),
    Error(ApiError),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExecuteRunRequest {
    pub code: String,
    pub source_path: Option<String>,
    pub execution_mode: Option<String>,
    pub document_version: Option<i64>,
    pub client_kind: Option<ClientKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ApiResponse<T> {
    pub protocol_version: String,
    pub request_id: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(request_id: impl Into<String>, data: T) -> Self {
        Self {
            protocol_version: WORKBENCH_PROTOCOL_VERSION.to_string(),
            request_id: request_id.into(),
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ApiError {
    pub protocol_version: String,
    pub request_id: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(transparent)]
pub struct DeepLink(pub String);

impl DeepLink {
    pub fn workspace(workspace_id: &str) -> Result<Self, ProtocolValidationError> {
        validate_identifier("workspace_id", workspace_id)?;
        Ok(Self(format!("rho://workspace/{workspace_id}")))
    }

    pub fn run(workspace_id: &str, run_id: &str) -> Result<Self, ProtocolValidationError> {
        validate_identifier("workspace_id", workspace_id)?;
        validate_identifier("run_id", run_id)?;
        Ok(Self(format!("rho://workspace/{workspace_id}/run/{run_id}")))
    }

    pub fn object(workspace_id: &str, object_id: &str) -> Result<Self, ProtocolValidationError> {
        validate_identifier("workspace_id", workspace_id)?;
        validate_identifier("object_id", object_id)?;
        Ok(Self(format!(
            "rho://workspace/{workspace_id}/object/{object_id}"
        )))
    }

    pub fn artifact(
        workspace_id: &str,
        artifact_id: &str,
    ) -> Result<Self, ProtocolValidationError> {
        validate_identifier("workspace_id", workspace_id)?;
        validate_identifier("artifact_id", artifact_id)?;
        Ok(Self(format!(
            "rho://workspace/{workspace_id}/artifact/{artifact_id}"
        )))
    }

    pub fn problem(workspace_id: &str, problem_id: &str) -> Result<Self, ProtocolValidationError> {
        validate_identifier("workspace_id", workspace_id)?;
        validate_identifier("problem_id", problem_id)?;
        Ok(Self(format!(
            "rho://workspace/{workspace_id}/problem/{problem_id}"
        )))
    }

    pub fn approval(
        workspace_id: &str,
        approval_id: &str,
    ) -> Result<Self, ProtocolValidationError> {
        validate_identifier("workspace_id", workspace_id)?;
        validate_identifier("approval_id", approval_id)?;
        Ok(Self(format!(
            "rho://workspace/{workspace_id}/approval/{approval_id}"
        )))
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolValidationError {
    #[error("{field} must not be empty")]
    EmptyIdentifier { field: &'static str },
    #[error("{field} contains a character that is not safe in a deep link")]
    UnsafeIdentifier { field: &'static str },
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ProtocolValidationError> {
    if value.is_empty() {
        return Err(ProtocolValidationError::EmptyIdentifier { field });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ProtocolValidationError::UnsafeIdentifier { field });
    }
    Ok(())
}

/// Returns one stable JSON Schema document containing all Workbench entities.
pub fn workbench_protocol_schema() -> Value {
    json!({
        "protocol_version": WORKBENCH_PROTOCOL_VERSION,
        "entities": {
            "workspace": schema_for!(Workspace),
            "run": schema_for!(Run),
            "object": schema_for!(Object),
            "artifact": schema_for!(Artifact),
            "problem": schema_for!(Problem),
            "approval": schema_for!(Approval),
            "provenance": schema_for!(Provenance),
            "runtime_event": schema_for!(RuntimeEvent),
            "event_page": schema_for!(EventPage),
            "event_stream_message": schema_for!(EventStreamMessage),
            "runtime_health": schema_for!(RuntimeHealth),
            "dependency_report": schema_for!(DependencyReport),
            "dependency_summary": schema_for!(DependencySummary),
            "client_session": schema_for!(ClientSession),
            "api_error": schema_for!(ApiError),
            "execute_run_request": schema_for!(ExecuteRunRequest)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkspaceIdentity;

    #[test]
    fn exports_every_required_entity_schema() {
        let schema = workbench_protocol_schema();
        let entities = schema["entities"].as_object().unwrap();
        for name in [
            "workspace",
            "run",
            "object",
            "artifact",
            "problem",
            "approval",
            "provenance",
            "runtime_event",
            "runtime_health",
            "dependency_report",
            "dependency_summary",
        ] {
            assert!(entities.contains_key(name), "missing {name} schema");
        }
        assert_eq!(schema["protocol_version"], WORKBENCH_PROTOCOL_VERSION);
    }

    #[test]
    fn scientific_workspace_state_excludes_client_connection_state() {
        let workspace_id = "ws_test";
        let workspace = Workspace {
            workspace_id: workspace_id.to_string(),
            lifecycle: WorkspaceLifecycle::Ready,
            identity: WorkspaceIdentity::new(workspace_id),
            project_root: Some("/project".to_string()),
            created_at: "2026-07-20T00:00:00Z".to_string(),
            updated_at: "2026-07-20T00:00:00Z".to_string(),
            deep_link: DeepLink::workspace(workspace_id).unwrap(),
        };
        let value = serde_json::to_value(workspace).unwrap();
        assert_eq!(value["lifecycle"], "ready");
        assert!(value.get("connection_id").is_none());
        assert!(value.get("client_kind").is_none());
    }

    #[test]
    fn runtime_health_additions_accept_legacy_payloads() {
        let health: RuntimeHealth = serde_json::from_value(json!({
            "status": "ok",
            "workspace_id": "ws_test",
            "lifecycle": "disconnected",
            "connected_clients": 0
        }))
        .unwrap();
        assert!(health.control_plane_ready);
        assert_eq!(health.project_root, None);
        assert!(!health.executor_attached);
        assert!(!health.workspace_r_ready);
        assert!(!health.dependencies.ready);
        assert_eq!(health.dependencies.status, DependencyStatus::Checking);
    }

    #[test]
    fn deep_links_are_stable_and_reject_unsafe_identifiers() {
        assert_eq!(
            DeepLink::run("ws_123", "exec_456").unwrap().0,
            "rho://workspace/ws_123/run/exec_456"
        );
        assert!(matches!(
            DeepLink::object("ws_123", "x/y"),
            Err(ProtocolValidationError::UnsafeIdentifier { .. })
        ));
    }

    #[test]
    fn response_carries_an_independent_workbench_version() {
        let response = ApiResponse::new("req_1", json!({"status": "ready"}));
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["protocol_version"], WORKBENCH_PROTOCOL_VERSION);
        assert_eq!(value["request_id"], "req_1");
    }
}
