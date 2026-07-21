use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use rho_mcp::McpServer;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Parser)]
#[command(name = "rho-mcp", about = "MCP adapter for the Rho scientific runtime")]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:8787")]
    server: String,
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
    let mut server = McpServer::new(&cli.server)?;
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
