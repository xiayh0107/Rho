#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use rho_core::{BrokerState, ExecutionOrigin};
use rho_kernel::{ArkLaunchConfig, ArkSession};
use rho_server::coordinator::{bootstrap_bridge, dispatch_workspace_request, run_agent_turn};
use rho_store::Store;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::{Manager, State};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

const BRIDGE_STATE: &str = include_str!("../../../r/rho.bridge/R/state.R");
const BRIDGE_EXECUTE: &str = include_str!("../../../r/rho.bridge/R/execute.R");
const BRIDGE_WORKSPACE: &str = include_str!("../../../r/rho.bridge/R/workspace.R");
const AGENT_STATE: &str = include_str!("../../../r/rho.agent/R/aaa-state.R");
const AGENT_TRANSPORT: &str = include_str!("../../../r/rho.agent/R/transport.R");
const AGENT_ADAPTER: &str = include_str!("../../../r/rho.agent/R/aisdk_adapter.R");

#[derive(Clone)]
struct RuntimeConfig {
    kernelspec: PathBuf,
    rscript: PathBuf,
    bridge_package: PathBuf,
    agent_package: PathBuf,
    store_path: PathBuf,
}

struct RuntimeContext {
    broker: BrokerState,
    store: Store,
}

struct AppState {
    config: RuntimeConfig,
    session: RwLock<Option<Arc<ArkSession>>>,
    context: Mutex<Option<RuntimeContext>>,
}

#[derive(Serialize)]
struct WorkspaceStatus {
    status: &'static str,
    r_version: String,
    r_home: String,
    kernel_pid: Option<u32>,
    workspace: Option<Value>,
    python_required: bool,
}

#[tauri::command]
async fn workspace_start(state: State<'_, AppState>) -> Result<WorkspaceStatus, String> {
    start_workspace(&state).await.map_err(display_error)
}

#[tauri::command]
async fn workspace_status(state: State<'_, AppState>) -> Result<Value, String> {
    let session = state.session.read().await.clone();
    let context = state.context.lock().await;
    Ok(json!({
        "status": if session.is_some() { "idle" } else { "disconnected" },
        "kernel_pid": session.as_ref().and_then(|value| value.child_pid()),
        "workspace": context.as_ref().map(|value| value.broker.identity()),
        "python_required": false
    }))
}

#[tauri::command]
async fn execute_r(code: String, state: State<'_, AppState>) -> Result<Value, String> {
    if code.trim().is_empty() {
        return Err("R code is empty".to_string());
    }
    let session = active_session(&state).await.map_err(display_error)?;
    let mut context = state.context.lock().await;
    let context = context
        .as_mut()
        .context("Workspace context is not ready")
        .map_err(display_error)?;
    let payload = json!({
        "arguments": {"code": code},
        "expected_workspace": context.broker.identity()
    });
    dispatch_workspace_request(
        "workspace.execute",
        &payload,
        ExecutionOrigin::User,
        session.as_ref(),
        &mut context.broker,
        &mut context.store,
    )
    .await
    .map_err(display_error)
}

#[tauri::command]
async fn snapshot_workspace(state: State<'_, AppState>) -> Result<Value, String> {
    let session = active_session(&state).await.map_err(display_error)?;
    let mut context = state.context.lock().await;
    let context = context
        .as_mut()
        .context("Workspace context is not ready")
        .map_err(display_error)?;
    let payload = json!({
        "arguments": {},
        "expected_workspace": context.broker.identity()
    });
    dispatch_workspace_request(
        "workspace.snapshot",
        &payload,
        ExecutionOrigin::System,
        session.as_ref(),
        &mut context.broker,
        &mut context.store,
    )
    .await
    .map_err(display_error)
}

#[tauri::command]
async fn run_agent(
    prompt: String,
    mode: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    if prompt.trim().is_empty() {
        return Err("Agent prompt is empty".to_string());
    }
    let session = active_session(&state).await.map_err(display_error)?;
    let mut context = state.context.lock().await;
    let context = context
        .as_mut()
        .context("Workspace context is not ready")
        .map_err(display_error)?;
    run_agent_turn(
        session.as_ref(),
        &mut context.broker,
        &mut context.store,
        state.config.rscript.clone(),
        state.config.agent_package.clone(),
        model.unwrap_or_else(|| "deepseek:deepseek-v4-flash".to_string()),
        prompt,
        mode,
    )
    .await
    .map_err(display_error)
}

#[tauri::command]
async fn interrupt_r(state: State<'_, AppState>) -> Result<Value, String> {
    let session = active_session(&state).await.map_err(display_error)?;
    session.interrupt().await.map_err(display_error)?;
    Ok(json!({"status": "interrupt_requested"}))
}

#[tauri::command]
async fn restart_workspace(state: State<'_, AppState>) -> Result<WorkspaceStatus, String> {
    state.context.lock().await.take();
    state.session.write().await.take();
    start_workspace(&state).await.map_err(display_error)
}

async fn active_session(state: &AppState) -> Result<Arc<ArkSession>> {
    state
        .session
        .read()
        .await
        .clone()
        .context("Workspace R is not running")
}

async fn start_workspace(state: &AppState) -> Result<WorkspaceStatus> {
    if let Some(session) = state.session.read().await.clone() {
        let context = state.context.lock().await;
        return status_from(
            &state.config,
            &session,
            context.as_ref().map(|value| value.broker.identity()),
        );
    }

    let session = Arc::new(
        ArkSession::launch(&ArkLaunchConfig::new(&state.config.kernelspec))
            .await
            .context("starting Ark-backed Workspace R")?,
    );
    let mut store = Store::open(&state.config.store_path).context("opening Rho event store")?;
    let mut broker = BrokerState::new(format!("desktop_{}", Uuid::new_v4()));
    store.save_identity(broker.identity())?;
    bootstrap_bridge(
        session.as_ref(),
        &mut broker,
        &mut store,
        &state.config.bridge_package,
    )
    .await?;
    let status = status_from(&state.config, &session, Some(broker.identity()))?;
    *state.context.lock().await = Some(RuntimeContext { broker, store });
    *state.session.write().await = Some(session);
    Ok(status)
}

fn status_from(
    config: &RuntimeConfig,
    session: &ArkSession,
    identity: Option<&rho_protocol::WorkspaceIdentity>,
) -> Result<WorkspaceStatus> {
    let metadata = std::fs::read_to_string(config.kernelspec.with_extension("runtime.json"))?;
    let metadata: Value = serde_json::from_str(&metadata)?;
    Ok(WorkspaceStatus {
        status: "idle",
        r_version: metadata["r_version"].as_str().unwrap_or("R").to_string(),
        r_home: metadata["r_home"].as_str().unwrap_or_default().to_string(),
        kernel_pid: session.child_pid(),
        workspace: identity.map(|value| serde_json::to_value(value).unwrap_or(Value::Null)),
        python_required: false,
    })
}

fn prepare_runtime(app: &tauri::App) -> Result<RuntimeConfig> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .context("resolving Rho application data directory")?;
    prepare_runtime_files(data_dir, locate_ark(app)?)
}

fn prepare_runtime_files(data_dir: PathBuf, ark: PathBuf) -> Result<RuntimeConfig> {
    std::fs::create_dir_all(&data_dir)?;
    let source_dir = data_dir.join("sources");
    let bridge_package = source_dir.join("rho.bridge");
    let agent_package = source_dir.join("rho.agent");
    write_source(&bridge_package.join("R/state.R"), BRIDGE_STATE)?;
    write_source(&bridge_package.join("R/execute.R"), BRIDGE_EXECUTE)?;
    write_source(&bridge_package.join("R/workspace.R"), BRIDGE_WORKSPACE)?;
    write_source(&agent_package.join("R/aaa-state.R"), AGENT_STATE)?;
    write_source(&agent_package.join("R/transport.R"), AGENT_TRANSPORT)?;
    write_source(&agent_package.join("R/aisdk_adapter.R"), AGENT_ADAPTER)?;

    let rscript = locate_rscript()?;
    let r_home = r_output(&rscript, "cat(normalizePath(R.home(), winslash = '/'))")?;
    let r_version = r_output(&rscript, "cat(R.version.string)")?;
    let r_libs = r_output(
        &rscript,
        "cat(paste(normalizePath(.libPaths(), winslash = '/', mustWork = FALSE), collapse = ';'))",
    )?;
    let runtime_dir = data_dir.join("runtime");
    std::fs::create_dir_all(&runtime_dir)?;
    let empty_renviron = runtime_dir.join("empty.Renviron");
    write_source(&empty_renviron, "")?;
    let log_path = runtime_dir.join("ark.log");
    let kernelspec = runtime_dir.join("kernel.json");
    let r_bin = Path::new(&r_home).join("bin").join("x64");
    let path = format!(
        "{};{}",
        r_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let spec = json!({
        "argv": [ark, "--connection_file", "{connection_file}", "--session-mode", "console", "--log", log_path, "--", "--interactive", "--no-environ", "--no-init-file", "--no-site-file"],
        "display_name": "Ark R 0.1.252 (Rho Desktop)",
        "language": "R",
        "interrupt_mode": "message",
        "kernel_protocol_version": "5.4",
        "env": {
            "R_HOME": r_home,
            "R_LIBS": r_libs,
            "R_ENVIRON_USER": empty_renviron,
            "PATH": path
        }
    });
    std::fs::write(&kernelspec, serde_json::to_vec_pretty(&spec)?)?;
    std::fs::write(
        kernelspec.with_extension("runtime.json"),
        serde_json::to_vec_pretty(&json!({"r_version": r_version, "r_home": r_home}))?,
    )?;
    Ok(RuntimeConfig {
        kernelspec,
        rscript,
        bridge_package,
        agent_package,
        store_path: data_dir.join("rho-desktop.sqlite"),
    })
}

fn locate_ark(app: &tauri::App) -> Result<PathBuf> {
    let development = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/runtime/ark.exe");
    let installed = app
        .path()
        .resource_dir()
        .context("resolving Rho resource directory")?
        .join("resources/runtime/ark.exe");
    [installed, development]
        .into_iter()
        .find(|path| path.is_file())
        .context("bundled Ark executable was not found")
}

fn locate_rscript() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("RHO_RSCRIPT") {
        let path = PathBuf::from(path);
        ensure!(path.is_file(), "RHO_RSCRIPT does not point to a file");
        return Ok(path);
    }
    let output = Command::new("where.exe")
        .arg("Rscript.exe")
        .output()
        .context("searching for Rscript.exe")?;
    if output.status.success() {
        return String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(PathBuf::from)
            .context("where.exe returned no Rscript path");
    }
    let program_files = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    if let Ok(entries) = std::fs::read_dir(program_files.join("R")) {
        let mut candidates = entries
            .flatten()
            .map(|entry| entry.path().join("bin/Rscript.exe"))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        candidates.sort();
        if let Some(path) = candidates.pop() {
            return Ok(path);
        }
    }
    bail!("Rscript.exe was not found. Install R 4.4 or later, then restart Rho.")
}

fn r_output(rscript: &Path, expression: &str) -> Result<String> {
    let output = Command::new(rscript)
        .args(["--vanilla", "-e", expression])
        .output()
        .with_context(|| format!("running {}", rscript.display()))?;
    ensure!(
        output.status.success(),
        "R runtime probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_source(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn startup_log_path() -> PathBuf {
    std::env::temp_dir().join("rho-desktop-startup.log")
}

fn write_startup_log(message: &str) {
    let _ = std::fs::write(startup_log_path(), message);
}

async fn smoke_test(include_agent: bool) -> Result<Value> {
    let data_dir = std::env::temp_dir().join("rho-desktop-smoke");
    let ark = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/runtime/ark.exe");
    let config = prepare_runtime_files(data_dir, ark)?;
    let session = ArkSession::launch(&ArkLaunchConfig::new(&config.kernelspec)).await?;
    let mut store = Store::open(&config.store_path)?;
    let mut broker = BrokerState::new("desktop_smoke");
    store.save_identity(broker.identity())?;
    bootstrap_bridge(&session, &mut broker, &mut store, &config.bridge_package).await?;
    let execute_payload = json!({
        "arguments": {
            "code": "rho_desktop_smoke <- data.frame(x = 1:5, y = (1:5)^2); plot(rho_desktop_smoke$x, rho_desktop_smoke$y, pch = 19)"
        },
        "expected_workspace": broker.identity()
    });
    let execution = dispatch_workspace_request(
        "workspace.execute",
        &execute_payload,
        ExecutionOrigin::User,
        &session,
        &mut broker,
        &mut store,
    )
    .await?;
    let snapshot_payload = json!({
        "arguments": {},
        "expected_workspace": broker.identity()
    });
    let snapshot = dispatch_workspace_request(
        "workspace.snapshot",
        &snapshot_payload,
        ExecutionOrigin::System,
        &session,
        &mut broker,
        &mut store,
    )
    .await?;
    let plot_count = execution["events"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|event| event["type"] == "display_data")
        .count();
    let object_found = snapshot["execution"]["objects"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|object| object["name"] == "rho_desktop_smoke");
    ensure!(plot_count > 0, "desktop smoke test did not receive a plot");
    ensure!(
        object_found,
        "desktop smoke object was absent from Environment"
    );
    let agent = if include_agent {
        let result = run_agent_turn(
            &session,
            &mut broker,
            &mut store,
            config.rscript.clone(),
            config.agent_package.clone(),
            "deepseek:deepseek-v4-flash".to_string(),
            "请检查 rho_desktop_smoke 对象，告诉我它有多少行和多少列。不要修改工作区。".to_string(),
            "ask".to_string(),
        )
        .await?;
        let completed = result["events"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|event| event["type"] == "chat.message_completed");
        ensure!(completed, "desktop Agent turn omitted its final message");
        Some(json!({"completed": true, "model": result["model"]}))
    } else {
        None
    };
    Ok(json!({
        "type": "rho_desktop_smoke",
        "workspace": broker.identity(),
        "plot_count": plot_count,
        "environment_object_found": object_found,
        "agent": agent,
        "event_count": store.event_count()?,
        "python_required": false
    }))
}

fn main() {
    let _ = std::fs::remove_file(startup_log_path());
    std::panic::set_hook(Box::new(|information| {
        write_startup_log(&format!("Rho desktop panic: {information}"));
    }));
    let arguments = std::env::args().collect::<Vec<_>>();
    let smoke_agent = arguments.iter().any(|argument| argument == "--smoke-agent");
    if smoke_agent || arguments.iter().any(|argument| argument == "--smoke-test") {
        let runtime = tokio::runtime::Runtime::new().expect("creating smoke-test runtime");
        match runtime.block_on(smoke_test(smoke_agent)) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
                return;
            }
            Err(error) => {
                eprintln!("Rho desktop smoke test failed: {error:#}");
                std::process::exit(1);
            }
        }
    }
    tauri::Builder::default()
        .setup(|app| {
            let config = prepare_runtime(app).map_err(|error| {
                write_startup_log(&format!("Rho desktop setup failed: {error:#}"));
                error
            })?;
            app.manage(AppState {
                config,
                session: RwLock::new(None),
                context: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            workspace_start,
            workspace_status,
            execute_r,
            snapshot_workspace,
            run_agent,
            interrupt_r,
            restart_workspace
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rho desktop");
}
