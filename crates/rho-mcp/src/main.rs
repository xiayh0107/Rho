use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use rho_mcp::McpServer;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";
const SERVER_URL_ENV: &str = "RHO_SERVER_URL";

#[derive(Debug, Parser)]
#[command(name = "rho-mcp", about = "MCP adapter for the Rho scientific runtime")]
struct Cli {
    #[arg(
        long,
        value_name = "URL",
        help = "Rho server URL (overrides RHO_SERVER_URL)"
    )]
    server: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rho-mcp: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let server_url = resolve_server_url(cli.server, std::env::var(SERVER_URL_ENV).ok());
    let mut server = McpServer::new(&server_url)?;
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = BufWriter::new(tokio::io::stdout());
    let mut line = String::new();
    loop {
        line.clear();
        let read = input
            .read_line(&mut line)
            .await
            .context("reading MCP stdin")?;
        if read == 0 {
            break;
        }
        let response = if read > MAX_MESSAGE_BYTES {
            Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {"code": -32600, "message": "MCP message exceeds 1 MiB"}
            }))
        } else {
            match serde_json::from_str::<Value>(&line) {
                Ok(message) => server.handle(message).await,
                Err(error) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {"code": -32700, "message": format!("Parse error: {error}")}
                })),
            }
        };
        if let Some(response) = response {
            output
                .write_all(serde_json::to_string(&response)?.as_bytes())
                .await?;
            output.write_all(b"\n").await?;
            output.flush().await?;
        }
    }
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
    fn server_argument_is_optional() {
        let cli = Cli::try_parse_from(["rho-mcp"]).unwrap();
        assert_eq!(cli.server, None);
    }

    #[test]
    fn explicit_server_precedes_environment() {
        assert_eq!(
            resolve_server_url(
                Some("https://explicit.example.test".to_string()),
                Some("https://environment.example.test".to_string()),
            ),
            "https://explicit.example.test"
        );
    }

    #[test]
    fn environment_server_precedes_default() {
        let resolved =
            resolve_server_url(None, Some("https://environment.example.test".to_string()));
        assert_eq!(resolved, "https://environment.example.test");
    }

    #[test]
    fn defaults_to_local_rho_server() {
        assert_eq!(resolve_server_url(None, None), DEFAULT_SERVER_URL);
    }
}
