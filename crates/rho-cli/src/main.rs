use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rho_cli::RhoClient;
use rho_cli::bootstrap::{AgentClient, bootstrap_project};
use rho_protocol::{ClientKind, ExecuteRunRequest, WORKBENCH_PROTOCOL_VERSION};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Parser)]
#[command(name = "rho", about = "Agent-friendly client for the Rho runtime")]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:8787", global = true)]
    server: String,
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
    /// Execute an R script in the authoritative workspace.
    Run {
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

impl Command {
    fn json_output(&self) -> bool {
        match self {
            Self::Init { json, .. }
            | Self::Status { json }
            | Self::Run { json, .. }
            | Self::Objects { json }
            | Self::Inspect { json, .. }
            | Self::Problems { json } => *json,
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
    let server_url = cli.server.clone();
    let client = RhoClient::new(&cli.server)?;
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
                for file in report.files {
                    println!("created: {file}");
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
        Command::Run { script, json } => {
            let code = tokio::fs::read_to_string(&script)
                .await
                .with_context(|| format!("reading {}", script.display()))?;
            let run = client
                .execute(&ExecuteRunRequest {
                    code,
                    source_path: Some(script.to_string_lossy().replace('\\', "/")),
                    execution_mode: Some("source".to_string()),
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
