use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use chrono::Utc;
use rho_core::ExecutionOrigin;
use rho_protocol::{
    ClientKind, ClientSession, DeepLink, DependencyIssue, DependencyPhase, DependencyReport,
    DependencyStatus, Envelope, ExecuteRunRequest, MessageKind, Object as ProtocolObject,
    OperationClass, RuntimeEvent, RuntimeEventKind, Workspace, WorkspaceIdentity,
    WorkspaceLifecycle,
};
use rho_runtime_deps::{DependencyManager, EnsureOptions};
use rho_store::{
    ApprovalRequestSummary, PlotArtifactSummary, ProblemSummary, RunDetail, RunSummary, Store,
    StoredEvent,
};
use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock, broadcast};
use uuid::Uuid;

use crate::execution::{ArkWorkspaceExecutor, WorkspaceExecution, WorkspaceExecutor};

const DEFAULT_EVENT_BUFFER: usize = 256;
const DEFAULT_REPLAY_LIMIT: usize = 1_000;

#[derive(Debug, Clone)]
pub struct RuntimeServiceConfig {
    pub store_path: PathBuf,
    pub workspace_id: Option<String>,
    pub project_root: Option<PathBuf>,
    pub event_buffer: usize,
}

impl RuntimeServiceConfig {
    pub fn new(store_path: impl Into<PathBuf>) -> Self {
        Self {
            store_path: store_path.into(),
            workspace_id: None,
            project_root: None,
            event_buffer: DEFAULT_EVENT_BUFFER,
        }
    }

    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    pub fn with_project_root(mut self, project_root: impl Into<PathBuf>) -> Self {
        self.project_root = Some(project_root.into());
        self
    }
}

#[derive(Clone)]
pub struct RuntimeService {
    inner: Arc<RuntimeServiceInner>,
}

struct RuntimeServiceInner {
    workspace: RwLock<Workspace>,
    clients: RwLock<HashMap<String, ClientSession>>,
    store_path: PathBuf,
    store: Mutex<Store>,
    mutations: Mutex<()>,
    execution_gate: Mutex<()>,
    dependency_start_gate: Mutex<()>,
    dependency_manager: RwLock<Option<DependencyManager>>,
    executor: RwLock<Option<Arc<dyn WorkspaceExecutor>>>,
    object_snapshot: RwLock<Option<ObjectSnapshot>>,
    events: broadcast::Sender<RuntimeEvent>,
}

#[derive(Clone)]
struct ObjectSnapshot {
    kernel_instance_id: String,
    state_revision: u64,
    objects: Vec<ProtocolObject>,
}

impl RuntimeService {
    pub fn open(config: RuntimeServiceConfig) -> Result<Self> {
        ensure!(
            config.event_buffer > 0,
            "event buffer must be greater than zero"
        );
        if let Some(parent) = config.store_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("creating runtime state directory {}", parent.display())
            })?;
        }

        let mut store = Store::open(&config.store_path)
            .with_context(|| format!("opening runtime store {}", config.store_path.display()))?;
        let stored_workspace = store.load_workspace()?;
        let stored_identity = store.load_identity()?;
        let now = Utc::now().to_rfc3339();

        let mut workspace = match stored_workspace {
            Some(workspace) => workspace,
            None => {
                let identity = stored_identity.unwrap_or_else(|| {
                    WorkspaceIdentity::new(
                        config
                            .workspace_id
                            .clone()
                            .unwrap_or_else(|| format!("ws_{}", Uuid::new_v4().simple())),
                    )
                });
                Workspace {
                    workspace_id: identity.workspace_id.clone(),
                    lifecycle: WorkspaceLifecycle::Disconnected,
                    deep_link: DeepLink::workspace(&identity.workspace_id)?,
                    identity,
                    project_root: None,
                    created_at: now.clone(),
                    updated_at: now,
                }
            }
        };

        if let Some(requested_workspace_id) = config.workspace_id.as_deref() {
            ensure!(
                requested_workspace_id == workspace.workspace_id,
                "runtime store belongs to workspace {}, not {}",
                workspace.workspace_id,
                requested_workspace_id
            );
        }
        ensure!(
            workspace.workspace_id == workspace.identity.workspace_id,
            "stored workspace and identity disagree"
        );
        if let Some(project_root) = config.project_root.as_deref() {
            workspace.project_root = Some(normalized_path(project_root));
        }
        if matches!(
            workspace.lifecycle,
            WorkspaceLifecycle::Starting
                | WorkspaceLifecycle::Ready
                | WorkspaceLifecycle::Busy
                | WorkspaceLifecycle::Restarting
        ) {
            workspace.lifecycle = WorkspaceLifecycle::Disconnected;
            workspace.updated_at = Utc::now().to_rfc3339();
        }
        workspace.deep_link = DeepLink::workspace(&workspace.workspace_id)?;
        store.save_workspace(&workspace)?;

        let (events, _) = broadcast::channel(config.event_buffer);
        Ok(Self {
            inner: Arc::new(RuntimeServiceInner {
                workspace: RwLock::new(workspace),
                clients: RwLock::new(HashMap::new()),
                store_path: config.store_path,
                store: Mutex::new(store),
                mutations: Mutex::new(()),
                execution_gate: Mutex::new(()),
                dependency_start_gate: Mutex::new(()),
                dependency_manager: RwLock::new(None),
                executor: RwLock::new(None),
                object_snapshot: RwLock::new(None),
                events,
            }),
        })
    }

    pub async fn workspace(&self) -> Workspace {
        self.inner.workspace.read().await.clone()
    }

    pub async fn has_executor(&self) -> bool {
        self.inner.executor.read().await.is_some()
    }

    pub async fn set_dependency_manager(&self, manager: DependencyManager) {
        *self.inner.dependency_manager.write().await = Some(manager);
    }

    pub async fn dependency_manager(&self) -> Option<DependencyManager> {
        self.inner.dependency_manager.read().await.clone()
    }

    pub async fn dependency_report(&self) -> DependencyReport {
        if let Some(manager) = self.dependency_manager().await {
            return manager.current_report().await;
        }
        DependencyReport {
            schema_version: "1".to_string(),
            revision: 0,
            status: DependencyStatus::ActionRequired,
            ready: false,
            phase: DependencyPhase::Idle,
            managed_by: "rho".to_string(),
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            components: Vec::new(),
            issue: Some(DependencyIssue {
                code: "dependency.manager_not_configured".to_string(),
                title: "Runtime dependencies are not managed".to_string(),
                message: "This control plane was started without a project dependency manager."
                    .to_string(),
                retryable: false,
                requires_user_action: true,
                action_url: Some("rho://setup/dependencies".to_string()),
            }),
            available_actions: Vec::new(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    pub async fn ensure_managed_workspace(
        &self,
        options: EnsureOptions,
    ) -> Result<Option<Workspace>> {
        let _startup = self.inner.dependency_start_gate.lock().await;
        if self.has_executor().await {
            return Ok(Some(self.workspace().await));
        }
        let manager = self
            .dependency_manager()
            .await
            .context("Rho Dependency Manager is not configured")?;
        let prepared = match manager.ensure(options).await {
            Ok(prepared) => prepared,
            Err(error) => {
                self.set_lifecycle(WorkspaceLifecycle::Failed).await?;
                self.emit_dependency_status().await?;
                return Err(error);
            }
        };
        self.emit_dependency_status().await?;
        let Some(prepared) = prepared else {
            return Ok(None);
        };
        self.start_ark(prepared.kernelspec_path, prepared.bridge_package)
            .await
            .map(Some)
    }

    pub async fn emit_dependency_status(&self) -> Result<()> {
        let report = self.dependency_report().await;
        self.append_runtime_event(
            RuntimeEventKind::DependencyStatusChanged,
            serde_json::to_value(report)?,
            Some(self.workspace().await.deep_link),
        )
        .await?;
        Ok(())
    }

    pub async fn attach_executor(&self, executor: Arc<dyn WorkspaceExecutor>) -> Result<Workspace> {
        let identity = executor.identity().await?;
        let _mutation = self.inner.mutations.lock().await;
        let (workspace, previous) = {
            let mut workspace = self.inner.workspace.write().await;
            ensure!(
                workspace.workspace_id == identity.workspace_id,
                "executor belongs to workspace {}, not {}",
                identity.workspace_id,
                workspace.workspace_id
            );
            let previous = workspace.lifecycle;
            workspace.identity = identity;
            workspace.lifecycle = WorkspaceLifecycle::Ready;
            workspace.updated_at = Utc::now().to_rfc3339();
            (workspace.clone(), previous)
        };
        self.inner.store.lock().await.save_workspace(&workspace)?;
        *self.inner.executor.write().await = Some(executor);
        drop(_mutation);
        self.append_runtime_event(
            RuntimeEventKind::WorkspaceLifecycleChanged,
            json!({"previous": previous, "current": WorkspaceLifecycle::Ready}),
            Some(workspace.deep_link.clone()),
        )
        .await?;
        Ok(workspace)
    }

    pub async fn start_ark(
        &self,
        kernelspec: impl Into<PathBuf>,
        bridge_package: impl AsRef<Path>,
    ) -> Result<Workspace> {
        self.set_lifecycle(WorkspaceLifecycle::Starting).await?;
        let identity = self.workspace().await.identity;
        let executor = match ArkWorkspaceExecutor::launch(
            kernelspec,
            bridge_package,
            &self.inner.store_path,
            identity,
        )
        .await
        {
            Ok(executor) => executor,
            Err(error) => {
                self.set_lifecycle(WorkspaceLifecycle::Failed).await?;
                return Err(error);
            }
        };
        self.attach_executor(Arc::new(executor)).await
    }

    pub async fn execute(&self, request: ExecuteRunRequest) -> Result<WorkspaceExecution> {
        let client_kind = request.client_kind.unwrap_or(ClientKind::Cli);
        self.dispatch(
            "workspace.execute",
            json!({
                "code": request.code,
                "source_path": request.source_path,
                "execution_mode": request.execution_mode,
                "document_version": request.document_version,
                "client_kind": client_kind
            }),
            execution_origin(client_kind),
        )
        .await
    }

    pub async fn scientific_objects(&self) -> Result<Vec<ProtocolObject>> {
        let workspace = self.workspace().await;
        if let Some(snapshot) = self.inner.object_snapshot.read().await.as_ref()
            && snapshot.kernel_instance_id == workspace.identity.kernel_instance_id
            && snapshot.state_revision == workspace.identity.state_revision
        {
            return Ok(snapshot.objects.clone());
        }
        if !self.has_executor().await {
            return Ok(Vec::new());
        }

        let execution = self
            .dispatch("workspace.snapshot", json!({}), ExecutionOrigin::System)
            .await?;
        let workspace = self.workspace().await;
        let objects = bridge_object_list(&execution, &workspace)?;
        *self.inner.object_snapshot.write().await = Some(ObjectSnapshot {
            kernel_instance_id: workspace.identity.kernel_instance_id.clone(),
            state_revision: workspace.identity.state_revision,
            objects: objects.clone(),
        });
        self.emit_event(
            RuntimeEventKind::ObjectChanged,
            json!({"count": objects.len(), "snapshot_run_id": execution.run.run_id}),
            Some(workspace.deep_link),
        )
        .await?;
        Ok(objects)
    }

    pub async fn cached_scientific_objects(&self) -> Vec<ProtocolObject> {
        let workspace = self.workspace().await;
        self.inner
            .object_snapshot
            .read()
            .await
            .as_ref()
            .filter(|snapshot| {
                snapshot.kernel_instance_id == workspace.identity.kernel_instance_id
                    && snapshot.state_revision == workspace.identity.state_revision
            })
            .map(|snapshot| snapshot.objects.clone())
            .unwrap_or_default()
    }

    pub async fn inspect_object(&self, object_id_or_name: &str) -> Result<Option<ProtocolObject>> {
        let name =
            decode_object_name(object_id_or_name).unwrap_or_else(|| object_id_or_name.to_string());
        let objects = self.scientific_objects().await?;
        let Some(mut object) = objects.into_iter().find(|object| object.name == name) else {
            return Ok(None);
        };
        if !self.has_executor().await {
            return Ok(Some(object));
        }

        let execution = self
            .dispatch(
                "workspace.inspect_object",
                json!({"name": name}),
                ExecutionOrigin::System,
            )
            .await?;
        let inspection = execution.result["execution"].clone();
        ensure!(
            inspection["ok"].as_bool().unwrap_or(false),
            "Workspace R returned an invalid object inspection"
        );
        object.metadata = json!({
            "size_bytes": inspection["size_bytes"],
            "preview_kind": inspection["preview_kind"],
            "preview": inspection["preview"],
            "semantic": inspection["semantic"],
            "structure": inspection["structure"],
            "inspection_run_id": execution.run.run_id,
            "state_revision": execution.identity.state_revision,
            "project_revision": execution.identity.project_revision
        });
        object.lineage.push(execution.run.run_id);
        if let Some(snapshot) = self.inner.object_snapshot.write().await.as_mut()
            && let Some(cached) = snapshot
                .objects
                .iter_mut()
                .find(|cached| cached.object_id == object.object_id)
        {
            *cached = object.clone();
        }
        Ok(Some(object))
    }

    pub async fn dispatch(
        &self,
        request_type: &str,
        arguments: Value,
        origin: ExecutionOrigin,
    ) -> Result<WorkspaceExecution> {
        let _execution = self.inner.execution_gate.lock().await;
        let executor = self
            .inner
            .executor
            .read()
            .await
            .clone()
            .context("Workspace R execution backend is not attached")?;
        let expected = self.workspace().await.identity;
        self.set_lifecycle(WorkspaceLifecycle::Busy).await?;
        let execution = executor
            .dispatch(request_type, arguments, origin, expected)
            .await;
        match execution {
            Ok(execution) => {
                let workspace = self
                    .synchronize_execution_identity(execution.identity.clone())
                    .await?;
                self.append_runtime_event(
                    RuntimeEventKind::RunFinished,
                    json!({
                        "run_id": execution.run.run_id,
                        "status": execution.run.status,
                        "request_type": execution.run.request_type
                    }),
                    Some(DeepLink::run(
                        &workspace.workspace_id,
                        &execution.run.run_id,
                    )?),
                )
                .await?;
                Ok(execution)
            }
            Err(error) => {
                self.set_lifecycle(WorkspaceLifecycle::Ready).await?;
                self.append_runtime_event(
                    RuntimeEventKind::ProblemReported,
                    json!({"request_type": request_type, "message": error.to_string()}),
                    None,
                )
                .await?;
                Err(error)
            }
        }
    }

    pub async fn shutdown_executor(&self) -> Result<()> {
        let executor = self.inner.executor.write().await.take();
        if let Some(executor) = executor {
            executor.shutdown().await?;
        }
        self.set_lifecycle(WorkspaceLifecycle::Disconnected).await?;
        Ok(())
    }

    pub async fn connect_client(&self, client_kind: ClientKind) -> ClientSession {
        let now = Utc::now().to_rfc3339();
        let session = ClientSession {
            connection_id: format!("connection_{}", Uuid::new_v4().simple()),
            client_kind,
            connected_at: now.clone(),
            last_seen_at: now,
        };
        self.inner
            .clients
            .write()
            .await
            .insert(session.connection_id.clone(), session.clone());
        session
    }

    pub async fn disconnect_client(&self, connection_id: &str) -> bool {
        self.inner
            .clients
            .write()
            .await
            .remove(connection_id)
            .is_some()
    }

    pub async fn client_sessions(&self) -> Vec<ClientSession> {
        let mut sessions = self
            .inner
            .clients
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.connection_id.cmp(&right.connection_id));
        sessions
    }

    pub async fn set_lifecycle(&self, lifecycle: WorkspaceLifecycle) -> Result<Workspace> {
        let _mutation = self.inner.mutations.lock().await;
        let (workspace, previous) = {
            let mut workspace = self.inner.workspace.write().await;
            if workspace.lifecycle == lifecycle {
                return Ok(workspace.clone());
            }
            let previous = workspace.lifecycle;
            workspace.lifecycle = lifecycle;
            workspace.updated_at = Utc::now().to_rfc3339();
            (workspace.clone(), previous)
        };
        self.persist_workspace(&workspace).await?;
        self.append_runtime_event(
            RuntimeEventKind::WorkspaceLifecycleChanged,
            json!({"previous": previous, "current": lifecycle}),
            Some(workspace.deep_link.clone()),
        )
        .await?;
        Ok(workspace)
    }

    pub async fn set_project_root(&self, project_root: impl AsRef<Path>) -> Result<Workspace> {
        let project_root = normalized_path(project_root.as_ref());
        let _mutation = self.inner.mutations.lock().await;
        let workspace = {
            let mut workspace = self.inner.workspace.write().await;
            if workspace.project_root.as_deref() == Some(project_root.as_str()) {
                return Ok(workspace.clone());
            }
            workspace.project_root = Some(project_root.clone());
            workspace.identity.apply(OperationClass::ProjectMutation);
            workspace.updated_at = Utc::now().to_rfc3339();
            workspace.clone()
        };
        self.persist_workspace(&workspace).await?;
        self.append_runtime_event(
            RuntimeEventKind::WorkspaceUpdated,
            json!({"project_root": project_root}),
            Some(workspace.deep_link.clone()),
        )
        .await?;
        Ok(workspace)
    }

    pub async fn emit_event(
        &self,
        kind: RuntimeEventKind,
        payload: Value,
        deep_link: Option<DeepLink>,
    ) -> Result<RuntimeEvent> {
        let _mutation = self.inner.mutations.lock().await;
        self.append_runtime_event(kind, payload, deep_link).await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.inner.events.subscribe()
    }

    pub async fn replay_events(
        &self,
        after_sequence: u64,
        limit: Option<usize>,
    ) -> Result<Vec<RuntimeEvent>> {
        let store = self.inner.store.lock().await;
        let events = store.list_events(
            i64::try_from(after_sequence).context("event sequence exceeds SQLite range")?,
            Some(limit.unwrap_or(DEFAULT_REPLAY_LIMIT)),
        )?;
        events
            .into_iter()
            .filter(|stored| stored.envelope.payload["type"] == "workbench.event")
            .map(runtime_event_from_stored)
            .collect()
    }

    pub async fn list_runs(&self, limit: Option<usize>) -> Result<Vec<RunSummary>> {
        Ok(self.inner.store.lock().await.list_runs(limit)?)
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Option<RunDetail>> {
        Ok(self.inner.store.lock().await.get_run_detail(run_id)?)
    }

    pub async fn list_problems(&self, limit: Option<usize>) -> Result<Vec<ProblemSummary>> {
        Ok(self.inner.store.lock().await.list_problems(limit)?)
    }

    pub async fn list_plot_artifacts(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<PlotArtifactSummary>> {
        Ok(self.inner.store.lock().await.list_plot_artifacts(limit)?)
    }

    pub async fn list_approvals(
        &self,
        limit: Option<usize>,
        status: Option<&str>,
    ) -> Result<Vec<ApprovalRequestSummary>> {
        Ok(self
            .inner
            .store
            .lock()
            .await
            .list_approval_requests(limit, status)?)
    }

    async fn synchronize_execution_identity(
        &self,
        identity: WorkspaceIdentity,
    ) -> Result<Workspace> {
        let _mutation = self.inner.mutations.lock().await;
        let workspace = {
            let mut workspace = self.inner.workspace.write().await;
            ensure!(
                workspace.workspace_id == identity.workspace_id,
                "execution returned a different workspace identity"
            );
            workspace.identity = identity;
            workspace.lifecycle = WorkspaceLifecycle::Ready;
            workspace.updated_at = Utc::now().to_rfc3339();
            workspace.clone()
        };
        self.inner.store.lock().await.save_workspace(&workspace)?;
        Ok(workspace)
    }

    async fn persist_workspace(&self, workspace: &Workspace) -> Result<()> {
        self.inner.store.lock().await.save_workspace(workspace)?;
        Ok(())
    }

    async fn append_runtime_event(
        &self,
        kind: RuntimeEventKind,
        payload: Value,
        deep_link: Option<DeepLink>,
    ) -> Result<RuntimeEvent> {
        let workspace_id = self.inner.workspace.read().await.workspace_id.clone();
        let envelope = Envelope::new(
            MessageKind::Event,
            json!({
                "type": "workbench.event",
                "workspace_id": workspace_id,
                "kind": kind,
                "payload": payload,
                "deep_link": deep_link
            }),
        );
        let sequence = self.inner.store.lock().await.append_event(&envelope)?;
        let event = runtime_event_from_stored(StoredEvent { sequence, envelope })?;
        let _ = self.inner.events.send(event.clone());
        Ok(event)
    }
}

fn bridge_object_list(
    execution: &WorkspaceExecution,
    workspace: &Workspace,
) -> Result<Vec<ProtocolObject>> {
    let values = execution.result["execution"]["objects"]
        .as_array()
        .context("workspace snapshot omitted objects")?;
    values
        .iter()
        .map(|value| bridge_object(value, execution, workspace))
        .collect()
}

fn bridge_object(
    value: &Value,
    execution: &WorkspaceExecution,
    workspace: &Workspace,
) -> Result<ProtocolObject> {
    let name = value["name"]
        .as_str()
        .context("workspace object omitted name")?;
    let object_id = encode_object_name(name);
    Ok(ProtocolObject {
        deep_link: DeepLink::object(&workspace.workspace_id, &object_id)?,
        object_id,
        workspace_id: workspace.workspace_id.clone(),
        name: name.to_string(),
        r_type: value["typeof"].as_str().unwrap_or("unknown").to_string(),
        class: json_strings(&value["classes"]),
        dimensions: json_u64s(&value["dimensions"]),
        metadata: json!({
            "size_bytes": value["size_bytes"],
            "preview_kind": value["preview_kind"],
            "snapshot_run_id": execution.run.run_id,
            "state_revision": execution.identity.state_revision,
            "project_revision": execution.identity.project_revision
        }),
        lineage: vec![execution.run.run_id.clone()],
        related_artifacts: Vec::new(),
    })
}

fn json_strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn json_u64s(value: &Value) -> Vec<u64> {
    match value {
        Value::Number(value) => value.as_u64().into_iter().collect(),
        Value::Array(values) => values.iter().filter_map(Value::as_u64).collect(),
        _ => Vec::new(),
    }
}

fn encode_object_name(name: &str) -> String {
    let encoded = name
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("object_{encoded}")
}

fn decode_object_name(object_id: &str) -> Option<String> {
    let encoded = object_id.strip_prefix("object_")?;
    if encoded.is_empty() || encoded.len() % 2 != 0 {
        return None;
    }
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&encoded[index..index + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    String::from_utf8(bytes).ok()
}

fn runtime_event_from_stored(stored: StoredEvent) -> Result<RuntimeEvent> {
    let sequence = u64::try_from(stored.sequence).context("negative event sequence")?;
    let body = stored.envelope.payload;
    Ok(RuntimeEvent {
        sequence,
        event_id: stored.envelope.id,
        workspace_id: serde_json::from_value(body["workspace_id"].clone())?,
        timestamp: stored.envelope.timestamp,
        kind: serde_json::from_value(body["kind"].clone())?,
        payload: body["payload"].clone(),
        deep_link: serde_json::from_value(body["deep_link"].clone())?,
    })
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn execution_origin(client_kind: ClientKind) -> ExecutionOrigin {
    match client_kind {
        ClientKind::Mcp | ClientKind::Agent => ExecutionOrigin::Agent,
        ClientKind::Web | ClientKind::Cli | ClientKind::Desktop => ExecutionOrigin::User,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn execution_origin_preserves_agent_attribution() {
        assert_eq!(execution_origin(ClientKind::Mcp), ExecutionOrigin::Agent);
        assert_eq!(execution_origin(ClientKind::Agent), ExecutionOrigin::Agent);
        assert_eq!(execution_origin(ClientKind::Cli), ExecutionOrigin::User);
        assert_eq!(execution_origin(ClientKind::Web), ExecutionOrigin::User);
        assert_eq!(execution_origin(ClientKind::Desktop), ExecutionOrigin::User);
    }

    #[tokio::test]
    async fn workspace_identity_survives_service_reopen() {
        let directory = TempDir::new().unwrap();
        let store_path = directory.path().join("runtime.sqlite");
        let service = RuntimeService::open(
            RuntimeServiceConfig::new(&store_path)
                .with_workspace_id("ws_persisted")
                .with_project_root(directory.path()),
        )
        .unwrap();
        let first = service.workspace().await;
        drop(service);

        let reopened = RuntimeService::open(
            RuntimeServiceConfig::new(&store_path).with_workspace_id("ws_persisted"),
        )
        .unwrap();
        let second = reopened.workspace().await;
        assert_eq!(second.workspace_id, first.workspace_id);
        assert_eq!(second.identity, first.identity);
        assert_eq!(second.created_at, first.created_at);
        assert_eq!(second.project_root, first.project_root);
    }

    #[tokio::test]
    async fn reopening_without_an_executor_normalizes_active_lifecycles() {
        let directory = TempDir::new().unwrap();
        for (index, lifecycle) in [
            WorkspaceLifecycle::Starting,
            WorkspaceLifecycle::Ready,
            WorkspaceLifecycle::Busy,
            WorkspaceLifecycle::Restarting,
        ]
        .into_iter()
        .enumerate()
        {
            let store_path = directory.path().join(format!("runtime-{index}.sqlite"));
            let service = RuntimeService::open(
                RuntimeServiceConfig::new(&store_path)
                    .with_workspace_id(format!("ws_recovery_{index}")),
            )
            .unwrap();
            service.set_lifecycle(lifecycle).await.unwrap();
            drop(service);

            let reopened = RuntimeService::open(RuntimeServiceConfig::new(&store_path)).unwrap();
            assert_eq!(
                reopened.workspace().await.lifecycle,
                WorkspaceLifecycle::Disconnected
            );
            assert!(!reopened.has_executor().await);
        }
    }

    #[tokio::test]
    async fn client_disconnect_does_not_change_scientific_state() {
        let directory = TempDir::new().unwrap();
        let service = RuntimeService::open(
            RuntimeServiceConfig::new(directory.path().join("runtime.sqlite"))
                .with_workspace_id("ws_clients"),
        )
        .unwrap();
        let before = service.workspace().await;
        let client = service.connect_client(ClientKind::Web).await;
        assert_eq!(service.client_sessions().await, vec![client.clone()]);
        assert!(service.disconnect_client(&client.connection_id).await);
        assert!(service.client_sessions().await.is_empty());
        assert_eq!(service.workspace().await, before);
    }

    #[tokio::test]
    async fn events_can_be_replayed_after_service_reopen() {
        let directory = TempDir::new().unwrap();
        let store_path = directory.path().join("runtime.sqlite");
        let service = RuntimeService::open(
            RuntimeServiceConfig::new(&store_path).with_workspace_id("ws_events"),
        )
        .unwrap();
        let mut subscriber = service.subscribe();
        let workspace = service
            .set_lifecycle(WorkspaceLifecycle::Ready)
            .await
            .unwrap();
        let live = subscriber.recv().await.unwrap();
        assert_eq!(live.kind, RuntimeEventKind::WorkspaceLifecycleChanged);
        assert_eq!(live.workspace_id, workspace.workspace_id);
        drop(service);

        let reopened = RuntimeService::open(
            RuntimeServiceConfig::new(&store_path).with_workspace_id("ws_events"),
        )
        .unwrap();
        let replayed = reopened.replay_events(0, None).await.unwrap();
        assert_eq!(replayed, vec![live]);
    }

    #[test]
    fn rejects_a_workspace_id_that_does_not_match_the_store() {
        let directory = TempDir::new().unwrap();
        let store_path = directory.path().join("runtime.sqlite");
        RuntimeService::open(
            RuntimeServiceConfig::new(&store_path).with_workspace_id("ws_original"),
        )
        .unwrap();
        let error = RuntimeService::open(
            RuntimeServiceConfig::new(&store_path).with_workspace_id("ws_other"),
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("ws_original"));
    }
}
