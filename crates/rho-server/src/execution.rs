use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use rho_core::{BrokerState, ExecutionOrigin};
use rho_kernel::{ArkLaunchConfig, ArkSession};
use rho_protocol::{ExpectedWorkspace, WorkspaceIdentity};
use rho_store::{RunDetail, Store};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::coordinator::{CoordinatorRuntime, bootstrap_bridge, dispatch_workspace_request};

#[derive(Debug, Clone)]
pub struct WorkspaceExecution {
    pub run: RunDetail,
    pub result: Value,
    pub identity: WorkspaceIdentity,
}

#[async_trait]
pub trait WorkspaceExecutor: Send + Sync {
    async fn identity(&self) -> Result<WorkspaceIdentity>;

    async fn dispatch(
        &self,
        request_type: &str,
        arguments: Value,
        origin: ExecutionOrigin,
        expected: WorkspaceIdentity,
    ) -> Result<WorkspaceExecution>;

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub struct ArkWorkspaceExecutor {
    state: Mutex<ArkExecutorState>,
}

struct ArkExecutorState {
    session: Option<ArkSession>,
    context: CoordinatorRuntime,
}

impl ArkWorkspaceExecutor {
    pub async fn launch(
        kernelspec: impl Into<PathBuf>,
        bridge_package: impl AsRef<Path>,
        store_path: impl AsRef<Path>,
        identity: WorkspaceIdentity,
    ) -> Result<Self> {
        let kernelspec = kernelspec.into();
        let mut store = Store::open(store_path.as_ref()).with_context(|| {
            format!("opening execution store {}", store_path.as_ref().display())
        })?;
        store.recover_incomplete_runs()?;
        store.recover_incomplete_agent_turns()?;
        store.recover_incomplete_approvals()?;

        let mut broker = BrokerState::from_identity(identity);
        let mut session = ArkSession::launch(&ArkLaunchConfig::new(&kernelspec))
            .await
            .with_context(|| format!("starting Ark from {}", kernelspec.display()))?;
        if let Err(error) =
            bootstrap_bridge(&session, &mut broker, &mut store, bridge_package.as_ref()).await
        {
            let _ = session.shutdown().await;
            return Err(error);
        }

        Ok(Self {
            state: Mutex::new(ArkExecutorState {
                session: Some(session),
                context: CoordinatorRuntime { broker, store },
            }),
        })
    }
}

#[async_trait]
impl WorkspaceExecutor for ArkWorkspaceExecutor {
    async fn identity(&self) -> Result<WorkspaceIdentity> {
        Ok(self.state.lock().await.context.broker.identity().clone())
    }

    async fn dispatch(
        &self,
        request_type: &str,
        arguments: Value,
        origin: ExecutionOrigin,
        expected: WorkspaceIdentity,
    ) -> Result<WorkspaceExecution> {
        let mut state = self.state.lock().await;
        let ArkExecutorState { session, context } = &mut *state;
        let session = session.as_ref().context("Ark executor is shut down")?;
        let payload = json!({
            "arguments": arguments,
            "expected_workspace": ExpectedWorkspace {
                kernel_instance_id: Some(expected.kernel_instance_id),
                state_revision: Some(expected.state_revision),
                project_revision: Some(expected.project_revision),
            }
        });
        let result = dispatch_workspace_request(
            request_type,
            &payload,
            origin,
            session,
            &mut context.broker,
            &mut context.store,
        )
        .await?;
        let execution_id = result["execution_id"]
            .as_str()
            .context("workspace execution omitted execution_id")?;
        let run = context
            .store
            .get_run_detail(execution_id)?
            .with_context(|| format!("execution run `{execution_id}` was not persisted"))?;
        let identity = context.broker.identity().clone();
        ensure!(
            identity.workspace_id == expected.workspace_id,
            "executor changed workspace identity"
        );
        Ok(WorkspaceExecution {
            run,
            result,
            identity,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if let Some(mut session) = state.session.take() {
            session.shutdown().await?;
        }
        Ok(())
    }
}
