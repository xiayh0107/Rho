use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use clap::ValueEnum;
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum AgentClient {
    Codex,
    Claude,
}

#[derive(Debug, Serialize)]
pub struct BootstrapReport {
    pub client: AgentClient,
    pub project_root: String,
    pub r_project_detected: bool,
    pub r_project_markers: Vec<String>,
    pub files: Vec<String>,
}

struct ScaffoldFile {
    relative_path: &'static str,
    content: String,
}

pub fn bootstrap_project(
    project_root: &Path,
    client: AgentClient,
    server_url: &str,
    force: bool,
) -> Result<BootstrapReport> {
    let project_root = project_root
        .canonicalize()
        .with_context(|| format!("resolving project directory {}", project_root.display()))?;
    ensure!(project_root.is_dir(), "project root must be a directory");

    let files = scaffold_files(client, server_url)?;
    let mut collisions = Vec::new();
    for file in &files {
        let target = project_root.join(file.relative_path);
        if let Ok(metadata) = fs::symlink_metadata(&target) {
            ensure!(
                !metadata.file_type().is_symlink(),
                "refusing to replace symlink {}",
                target.display()
            );
            if !force {
                collisions.push(file.relative_path);
            }
        }
    }
    if !collisions.is_empty() {
        bail!(
            "bootstrap would overwrite existing files: {} (pass --force to replace regular files)",
            collisions.join(", ")
        );
    }

    for file in &files {
        let target = project_root.join(file.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&target, &file.content)
            .with_context(|| format!("writing {}", target.display()))?;
    }

    let markers = detect_r_project(&project_root)?;
    Ok(BootstrapReport {
        client,
        project_root: project_root.to_string_lossy().replace('\\', "/"),
        r_project_detected: !markers.is_empty(),
        r_project_markers: markers,
        files: files
            .iter()
            .map(|file| file.relative_path.to_string())
            .collect(),
    })
}

pub fn detect_r_project(project_root: &Path) -> Result<Vec<String>> {
    let mut markers = ["DESCRIPTION", "renv.lock", ".Rprofile"]
        .into_iter()
        .filter(|name| project_root.join(name).is_file())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if project_root.join("R").is_dir() {
        markers.push("R/".to_string());
    }
    for entry in fs::read_dir(project_root)
        .with_context(|| format!("reading project directory {}", project_root.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".rproj")
        {
            markers.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    markers.sort();
    markers.dedup();
    Ok(markers)
}

fn scaffold_files(client: AgentClient, server_url: &str) -> Result<Vec<ScaffoldFile>> {
    let quoted_url = serde_json::to_string(server_url)?;
    let mut files = vec![ScaffoldFile {
        relative_path: "AGENTS.md",
        content: AGENTS_TEMPLATE.to_string(),
    }];
    match client {
        AgentClient::Codex => {
            files.push(ScaffoldFile {
                relative_path: ".codex/config.toml",
                content: format!(
                    r#"[mcp_servers.rho]
command = "rho-mcp"
args = ["--server", {quoted_url}]
startup_timeout_sec = 30
tool_timeout_sec = 300
required = false
enabled = true
enabled_tools = ["workspace_open", "workspace_status", "workspace_execute", "object_inspect", "run_history", "problem_list", "artifact_export", "plot_view"]
default_tools_approval_mode = "writes"

[mcp_servers.rho.tools.workspace_execute]
approval_mode = "prompt"
"#
                ),
            });
            files.push(ScaffoldFile {
                relative_path: ".agents/skills/operate-rho-runtime/SKILL.md",
                content: CODEX_SKILL.to_string(),
            });
            files.push(ScaffoldFile {
                relative_path: ".agents/skills/operate-rho-runtime/agents/openai.yaml",
                content: CODEX_OPENAI_YAML.to_string(),
            });
        }
        AgentClient::Claude => {
            files.push(ScaffoldFile {
                relative_path: ".mcp.json",
                content: serde_json::to_string_pretty(&json!({
                    "mcpServers": {
                        "rho": {
                            "command": "rho-mcp",
                            "args": ["--server", server_url]
                        }
                    }
                }))? + "\n",
            });
            files.push(ScaffoldFile {
                relative_path: ".claude/skills/rho-runtime/SKILL.md",
                content: CLAUDE_SKILL.to_string(),
            });
        }
    }
    Ok(files)
}

const AGENTS_TEMPLATE: &str = r#"# Rho scientific runtime contract

This project uses Rho as the source of truth for R state.

- The Agent reasons, converses, and plans. Rho executes and proves.
- Prefer Rho MCP tools; fall back to `rho ... --json` when MCP is unavailable.
- Inspect before changing state and respect policy decisions and approval prompts.
- Execute scientific R code in the authoritative Workspace R, not an unrelated R session.
- Verify runs, problems, changed objects, plots, artifacts, and provenance.
- Report stable identifiers so another client can reconnect and verify the result.
"#;

const CODEX_SKILL: &str = r#"---
name: operate-rho-runtime
description: Operate an Agent-native Rho scientific runtime for R workspaces. Use for R execution, objects, plots, problems, runs, artifacts, or provenance; prefer Rho MCP and fall back to the machine-readable rho CLI.
---

# Operate Rho Runtime

1. Call `workspace_open` to discover the durable workspace and live objects.
2. Inspect relevant state before changing it.
3. Explain the intended effect and call `workspace_execute` only for requested R code. Honor approval prompts.
4. Verify with `run_history`, `problem_list`, object inspection, and plot inspection.
5. Report stable workspace, run, object, artifact, and provenance identifiers.

Do not bypass an available Rho runtime with a separate `R`, `Rscript`, or Ark process. Use `rho status --json` and the other JSON CLI commands only when MCP is unavailable.
"#;

const CODEX_OPENAI_YAML: &str = r#"interface:
  display_name: "Operate Rho Runtime"
  short_description: "Run trusted scientific R workflows through Rho"
  default_prompt: "Use $operate-rho-runtime to inspect this R workspace and execute a provenance-aware analysis."

policy:
  allow_implicit_invocation: true
"#;

const CLAUDE_SKILL: &str = r#"---
description: Operate the authoritative Rho scientific runtime. Use for R execution, live objects, plots, problems, run history, artifacts, and provenance in this project.
---

# Rho scientific runtime

1. Call `workspace_open` to discover the durable workspace and live objects.
2. Inspect relevant state before changing it.
3. Explain the intended effect and call `workspace_execute` only for requested R code. Honor approval prompts.
4. Verify runs, problems, changed objects, plots, artifacts, and provenance.

Do not start a separate `R`, `Rscript`, or Ark process as a substitute for a disconnected Rho runtime.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_common_r_project_markers() {
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join("analysis.Rproj"), "Version: 1.0\n").unwrap();
        fs::write(directory.path().join("renv.lock"), "{}\n").unwrap();
        fs::create_dir(directory.path().join("R")).unwrap();
        assert_eq!(
            detect_r_project(directory.path()).unwrap(),
            vec!["R/", "analysis.Rproj", "renv.lock"]
        );
    }

    #[test]
    fn bootstraps_codex_without_overwriting_by_default() {
        let directory = TempDir::new().unwrap();
        fs::write(directory.path().join("DESCRIPTION"), "Package: example\n").unwrap();
        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        assert!(report.r_project_detected);
        assert!(directory.path().join("AGENTS.md").is_file());
        assert!(directory.path().join(".codex/config.toml").is_file());
        assert!(
            directory
                .path()
                .join(".agents/skills/operate-rho-runtime/SKILL.md")
                .is_file()
        );
        let config = fs::read_to_string(directory.path().join(".codex/config.toml")).unwrap();
        assert!(config.contains("http://127.0.0.1:9999"));
        assert!(
            bootstrap_project(
                directory.path(),
                AgentClient::Codex,
                "http://127.0.0.1:9999",
                false
            )
            .unwrap_err()
            .to_string()
            .contains("would overwrite")
        );
    }

    #[test]
    fn bootstraps_valid_claude_mcp_json() {
        let directory = TempDir::new().unwrap();
        bootstrap_project(
            directory.path(),
            AgentClient::Claude,
            "https://rho.example.test/base",
            false,
        )
        .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.path().join(".mcp.json")).unwrap()).unwrap();
        assert_eq!(
            value["mcpServers"]["rho"]["args"][1],
            "https://rho.example.test/base"
        );
        assert!(
            directory
                .path()
                .join(".claude/skills/rho-runtime/SKILL.md")
                .is_file()
        );
    }
}
