use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rho_cli::RhoClient;
use rho_cli::bootstrap::{AgentClient, bootstrap_project};
use rho_protocol::{ClientKind, ExecuteRunRequest, WORKBENCH_PROTOCOL_VERSION};
use rho_runtime_deps::{DependencyManager, EnsureOptions};
use serde::Serialize;
use serde_json::json;
use tokio::io::AsyncReadExt;

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";
const SERVER_URL_ENV: &str = "RHO_SERVER_URL";

#[derive(Debug, Parser)]
#[command(name = "rho", about = "Agent-friendly client for the Rho runtime")]
struct Cli {
    #[arg(
        long,
        value_name = "URL",
        global = true,
        help = "Rho server URL (overrides RHO_SERVER_URL)"
    )]
    server: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Bootstrap Agent integration files in an existing R project.
    Init {
        #[arg(value_enum)]
        client: AgentClient,
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show the current runtime and workspace state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Diagnose the control plane and Workspace R in one request.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Inspect, prepare, and repair the local R/Ark runtime dependency set.
    Deps {
        #[command(subcommand)]
        command: DependencyCommand,
    },
    /// Execute an R script in the authoritative workspace.
    Run {
        /// Absolute script path, or `-` to read R code from stdin.
        script: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// List R objects in the latest workspace snapshot.
    Objects {
        #[arg(long)]
        json: bool,
    },
    /// Inspect one R object by stable object ID or name.
    Inspect {
        object: String,
        #[arg(long)]
        json: bool,
    },
    /// List structured runtime problems.
    Problems {
        #[arg(long)]
        json: bool,
    },
    /// Work with scientific plot artifacts.
    Plots {
        #[command(subcommand)]
        command: PlotCommand,
    },
}

#[derive(Debug, Subcommand)]
enum PlotCommand {
    /// List plot artifacts and provenance references.
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum DependencyCommand {
    /// Detect R, Ark, the generated binding, and embedded bridge resources.
    Status {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Prepare all non-privileged dependencies and generate the kernelspec.
    Ensure {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        offline: bool,
        /// Download a verified official R installer when compatible R is absent.
        #[arg(long)]
        install_r: bool,
        #[arg(long)]
        json: bool,
    },
    /// Re-verify and replace an invalid Rho-managed Ark cache entry.
    Repair {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        json: bool,
    },
    /// Print the immutable cross-project dependency cache location.
    CachePath {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

impl Command {
    fn json_output(&self) -> bool {
        match self {
            Self::Init { json, .. }
            | Self::Status { json }
            | Self::Doctor { json }
            | Self::Run { json, .. }
            | Self::Objects { json }
            | Self::Inspect { json, .. }
            | Self::Problems { json } => *json,
            Self::Deps { command } => match command {
                DependencyCommand::Status { json, .. }
                | DependencyCommand::Ensure { json, .. }
                | DependencyCommand::Repair { json, .. }
                | DependencyCommand::CachePath { json, .. } => *json,
            },
            Self::Plots {
                command: PlotCommand::List { json },
            } => *json,
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_output = cli.command.json_output();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if json_output {
                println!(
                    "{}",
                    json!({
                        "ok": false,
                        "protocol_version": WORKBENCH_PROTOCOL_VERSION,
                        "error": error.to_string()
                    })
                );
            } else {
                eprintln!("rho: {error:#}");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    let server_url = resolve_server_url(cli.server, std::env::var(SERVER_URL_ENV).ok());
    let client = RhoClient::new(&server_url)?;
    match cli.command {
        Command::Init {
            client,
            project,
            force,
            json,
        } => {
            let report = bootstrap_project(&project, client, &server_url, force)?;
            if json {
                print_json(&report)?;
            } else {
                println!("bootstrapped {:?}: {}", report.client, report.project_root);
                if report.r_project_detected {
                    println!("R project markers: {}", report.r_project_markers.join(", "));
                } else {
                    println!("R project markers: none detected");
                }
                for file in report.created {
                    println!("created: {file}");
                }
                for file in report.updated {
                    println!("updated: {file}");
                }
                for file in report.unchanged {
                    println!("unchanged: {file}");
                }
                for file in report.preserved {
                    println!("preserved: {file}");
                }
                for file in report.overwritten {
                    println!("overwritten: {file}");
                }
            }
        }
        Command::Status { json } => {
            let workspace = client.status().await?;
            if json {
                print_json(&workspace)?;
            } else {
                println!("{} · {:?}", workspace.workspace_id, workspace.lifecycle);
                if let Some(project_root) = workspace.project_root {
                    println!("project: {project_root}");
                }
            }
        }
        Command::Doctor { json } => {
            let health = client.health().await?;
            if json {
                print_json(&health)?;
            } else {
                println!("control plane: ready");
                println!("workspace: {}", health.workspace_id);
                if let Some(project_root) = &health.project_root {
                    println!("project: {project_root}");
                }
                println!("Workspace R: {:?}", health.lifecycle);
                println!(
                    "execution: {}",
                    if health.executor_attached {
                        "available"
                    } else {
                        "unavailable"
                    }
                );
                println!(
                    "dependencies: {:?}{}",
                    health.dependencies.status,
                    if health.dependencies.ready {
                        " (ready)"
                    } else {
                        " (not ready)"
                    }
                );
                if let Some(issue_code) = &health.dependencies.issue_code {
                    println!("dependency issue: {issue_code}");
                }
                if let Some(action_url) = &health.dependencies.action_url {
                    println!("dependency action: {action_url}");
                }
            }
        }
        Command::Deps { command } => match command {
            DependencyCommand::Status { project, json } => {
                let manager = DependencyManager::new(project)?;
                let report = manager.inspect().await?;
                print_dependency_report(&report, json)?;
            }
            DependencyCommand::Ensure {
                project,
                offline,
                install_r,
                json,
            } => {
                let manager = DependencyManager::new(project)?;
                manager
                    .ensure(EnsureOptions {
                        offline,
                        install_r,
                        ..EnsureOptions::default()
                    })
                    .await?;
                let report = manager.current_report().await;
                print_dependency_report(&report, json)?;
            }
            DependencyCommand::Repair {
                project,
                offline,
                json,
            } => {
                let manager = DependencyManager::new(project)?;
                manager
                    .ensure(EnsureOptions {
                        offline,
                        repair: true,
                        ..EnsureOptions::default()
                    })
                    .await?;
                let report = manager.current_report().await;
                print_dependency_report(&report, json)?;
            }
            DependencyCommand::CachePath { project, json } => {
                let manager = DependencyManager::new(project)?;
                let path = manager.cache_root().to_string_lossy().to_string();
                if json {
                    print_json(&json!({"cache_path": path}))?;
                } else {
                    println!("{path}");
                }
            }
        },
        Command::Run { script, json } => {
            let (code, source_path, execution_mode) = if script.as_os_str() == "-" {
                let mut code = String::new();
                tokio::io::stdin()
                    .read_to_string(&mut code)
                    .await
                    .context("reading R code from stdin")?;
                (code, None, "stdin".to_string())
            } else {
                let absolute = script
                    .canonicalize()
                    .with_context(|| format!("resolving {}", script.display()))?;
                let code = tokio::fs::read_to_string(&absolute)
                    .await
                    .with_context(|| format!("reading {}", absolute.display()))?;
                (
                    code,
                    Some(absolute.to_string_lossy().replace('\\', "/")),
                    "source".to_string(),
                )
            };
            let run = client
                .execute(&ExecuteRunRequest {
                    code,
                    source_path,
                    execution_mode: Some(execution_mode),
                    document_version: None,
                    client_kind: Some(ClientKind::Cli),
                })
                .await?;
            if json {
                print_json(&run)?;
            } else {
                println!("{} · {}", run.run_id, run.status);
            }
        }
        Command::Objects { json } => {
            let objects = client.objects().await?;
            if json {
                print_json(&objects)?;
            } else {
                for object in objects {
                    println!("{} · {}", object.name, object.r_type);
                }
            }
        }
        Command::Inspect { object, json } => {
            let object = client.inspect_object(&object).await?;
            if json {
                print_json(&object)?;
            } else {
                println!("{} · {}", object.name, object.r_type);
                println!("dimensions: {:?}", object.dimensions);
            }
        }
        Command::Problems { json } => {
            let problems = client.problems().await?;
            if json {
                print_json(&problems)?;
            } else {
                for problem in problems {
                    println!("{:?} · {}", problem.severity, problem.message);
                }
            }
        }
        Command::Plots {
            command: PlotCommand::List { json },
        } => {
            let plots = client.plots().await?;
            if json {
                print_json(&plots)?;
            } else {
                for plot in plots {
                    println!("{} · {}", plot.artifact_id, plot.media_type);
                }
            }
        }
    }
    Ok(())
}

fn print_dependency_report(report: &rho_protocol::DependencyReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }
    println!("dependencies: {:?}", report.status);
    for component in &report.components {
        let version = component.version.as_deref().unwrap_or("—");
        let path = component.path.as_deref().unwrap_or("—");
        println!(
            "{}: {:?} · {} · {}",
            component.name, component.status, version, path
        );
    }
    if let Some(issue) = &report.issue {
        println!("action: {} · {}", issue.code, issue.message);
    }
    for action in &report.available_actions {
        println!(
            "available: {} · {}{}",
            action.id,
            action.label,
            if action.requires_human {
                " · human approval required"
            } else {
                ""
            }
        );
    }
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "protocol_version": WORKBENCH_PROTOCOL_VERSION,
            "data": value
        }))?
    );
    Ok(())
}

fn resolve_server_url(explicit: Option<String>, environment: Option<String>) -> String {
    explicit
        .or(environment)
        .unwrap_or_else(|| DEFAULT_SERVER_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_url_uses_explicit_then_environment_then_default() {
        assert_eq!(
            resolve_server_url(
                Some("https://explicit.example.test".to_string()),
                Some("https://environment.example.test".to_string()),
            ),
            "https://explicit.example.test"
        );
        assert_eq!(
            resolve_server_url(None, Some("https://environment.example.test".to_string())),
            "https://environment.example.test"
        );
        assert_eq!(resolve_server_url(None, None), DEFAULT_SERVER_URL);
    }

    #[test]
    fn dependency_commands_are_project_scoped_and_machine_readable() {
        let cli = Cli::try_parse_from([
            "rho",
            "deps",
            "ensure",
            "--project",
            "/tmp/rho-project",
            "--offline",
            "--json",
        ])
        .unwrap();
        assert!(cli.command.json_output());
        let Command::Deps {
            command:
                DependencyCommand::Ensure {
                    project,
                    offline,
                    install_r,
                    json,
                },
        } = cli.command
        else {
            panic!("expected deps ensure command");
        };
        assert_eq!(project, PathBuf::from("/tmp/rho-project"));
        assert!(offline);
        assert!(!install_r);
        assert!(json);
    }

    #[test]
    fn run_accepts_stdin_without_a_cwd_relative_script() {
        let cli = Cli::try_parse_from(["rho", "run", "-", "--json"]).unwrap();
        let Command::Run { script, json } = cli.command else {
            panic!("expected run command");
        };
        assert_eq!(script, PathBuf::from("-"));
        assert!(json);
    }
}
