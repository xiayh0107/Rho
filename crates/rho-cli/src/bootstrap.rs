use std::fs;
use std::io::ErrorKind;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use reqwest::Url;
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
    /// Every file managed by this bootstrap invocation.
    pub files: Vec<String>,
    /// Files that did not exist and were created.
    pub created: Vec<String>,
    /// Existing files extended with Rho-owned configuration while preserving other content.
    pub updated: Vec<String>,
    /// Existing files whose contents already matched the scaffold.
    pub unchanged: Vec<String>,
    /// Differing regular files left untouched because `--force` was not used.
    pub preserved: Vec<String>,
    /// Differing regular files replaced because `--force` was used.
    pub overwritten: Vec<String>,
}

struct ScaffoldFile {
    relative_path: &'static str,
    content: String,
    merge_strategy: MergeStrategy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MergeStrategy {
    AgentsContract,
    CodexConfig,
    ClaudeMcp,
    Exact,
}

#[derive(Debug, Eq, PartialEq)]
enum ScaffoldAction {
    Create,
    Update(String),
    Unchanged,
    Preserve,
    Overwrite,
}

#[derive(Debug, Eq, PartialEq)]
enum MergeOutcome {
    Update(String),
    Unchanged,
    Conflict,
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

    // Validate the complete plan before writing anything. A symlink anywhere in
    // a managed path is a hard error so bootstrap never follows one into an
    // unexpected location, even when the final file does not exist yet.
    for file in &files {
        ensure_no_symlinks(&project_root, Path::new(file.relative_path))?;
    }

    let mut plan = Vec::with_capacity(files.len());
    for file in &files {
        let target = project_root.join(file.relative_path);
        let action = match fs::symlink_metadata(&target) {
            Ok(metadata) => {
                ensure!(
                    metadata.is_file(),
                    "refusing to replace non-regular path {}",
                    target.display()
                );
                let existing =
                    fs::read(&target).with_context(|| format!("reading {}", target.display()))?;
                plan_existing_file(file, &existing, server_url, force)?
            }
            Err(error) if error.kind() == ErrorKind::NotFound => ScaffoldAction::Create,
            Err(error) => {
                return Err(error).with_context(|| format!("inspecting {}", target.display()));
            }
        };
        plan.push(action);
    }

    let mut created = Vec::new();
    let mut updated = Vec::new();
    let mut unchanged = Vec::new();
    let mut preserved = Vec::new();
    let mut overwritten = Vec::new();
    for (file, action) in files.iter().zip(plan) {
        let target = project_root.join(file.relative_path);
        match action {
            ScaffoldAction::Create => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
                fs::write(&target, &file.content)
                    .with_context(|| format!("writing {}", target.display()))?;
                created.push(file.relative_path.to_string());
            }
            ScaffoldAction::Update(content) => {
                fs::write(&target, content)
                    .with_context(|| format!("updating {}", target.display()))?;
                updated.push(file.relative_path.to_string());
            }
            ScaffoldAction::Overwrite => {
                fs::write(&target, &file.content)
                    .with_context(|| format!("overwriting {}", target.display()))?;
                overwritten.push(file.relative_path.to_string());
            }
            ScaffoldAction::Unchanged => unchanged.push(file.relative_path.to_string()),
            ScaffoldAction::Preserve => preserved.push(file.relative_path.to_string()),
        }
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
        created,
        updated,
        unchanged,
        preserved,
        overwritten,
    })
}

fn plan_existing_file(
    file: &ScaffoldFile,
    existing: &[u8],
    server_url: &str,
    force: bool,
) -> Result<ScaffoldAction> {
    if existing == file.content.as_bytes() {
        return Ok(ScaffoldAction::Unchanged);
    }

    let merge = match std::str::from_utf8(existing) {
        Ok(existing) => match file.merge_strategy {
            MergeStrategy::AgentsContract => merge_agents_contract(existing),
            MergeStrategy::CodexConfig => merge_codex_config(existing, &file.content, server_url),
            MergeStrategy::ClaudeMcp => merge_claude_mcp(existing, server_url)?,
            MergeStrategy::Exact => MergeOutcome::Conflict,
        },
        Err(_) => MergeOutcome::Conflict,
    };

    Ok(match merge {
        MergeOutcome::Update(content) => ScaffoldAction::Update(content),
        MergeOutcome::Unchanged => ScaffoldAction::Unchanged,
        MergeOutcome::Conflict if force => ScaffoldAction::Overwrite,
        MergeOutcome::Conflict => ScaffoldAction::Preserve,
    })
}

fn merge_agents_contract(existing: &str) -> MergeOutcome {
    let begin_count = existing.matches(AGENTS_RHO_BEGIN).count();
    let end_count = existing.matches(AGENTS_RHO_END).count();
    if begin_count == 0 && end_count == 0 {
        if let Some(range) = unmarked_agents_contract_range(existing) {
            return MergeOutcome::Update(replace_document_range(
                existing,
                range,
                Some(&marked_agents_contract()),
            ));
        }
        return MergeOutcome::Update(append_document(existing, &marked_agents_contract()));
    }
    if begin_count != 1 || end_count != 1 {
        return MergeOutcome::Conflict;
    }

    let Some(begin) = existing.find(AGENTS_RHO_BEGIN) else {
        return MergeOutcome::Conflict;
    };
    let content_start = begin + AGENTS_RHO_BEGIN.len();
    let Some(relative_end) = existing[content_start..].find(AGENTS_RHO_END) else {
        return MergeOutcome::Conflict;
    };
    let content_end = content_start + relative_end;
    if existing[content_start..content_end].trim() != AGENTS_TEMPLATE.trim() {
        return MergeOutcome::Conflict;
    }

    if let Some(range) = unmarked_agents_contract_range(&existing[..begin]) {
        return MergeOutcome::Update(replace_document_range(existing, range, None));
    }
    let marked_end = content_end + AGENTS_RHO_END.len();
    if let Some(relative_range) = unmarked_agents_contract_range(&existing[marked_end..]) {
        let range = marked_end + relative_range.start..marked_end + relative_range.end;
        return MergeOutcome::Update(replace_document_range(existing, range, None));
    }
    MergeOutcome::Unchanged
}

fn unmarked_agents_contract_range(document: &str) -> Option<Range<usize>> {
    let start = document.find(AGENTS_RHO_HEADING)?;
    let tail = &document[start..];
    let end_offset = tail
        .find(AGENTS_RHO_FINAL_SENTENCE)
        .map(|offset| offset + AGENTS_RHO_FINAL_SENTENCE.len())
        .or_else(|| {
            tail.find(AGENTS_RHO_LEGACY_FINAL_SENTENCE)
                .map(|offset| offset + AGENTS_RHO_LEGACY_FINAL_SENTENCE.len())
        })?;
    let candidate = &tail[..end_offset];
    if !candidate.contains(AGENTS_RHO_INTRO) || !candidate.contains(AGENTS_RHO_AGENT_RULE) {
        return None;
    }
    let mut end = start + end_offset;
    if document.as_bytes().get(end) == Some(&b'\r') {
        end += 1;
    }
    if document.as_bytes().get(end) == Some(&b'\n') {
        end += 1;
    }
    Some(start..end)
}

fn replace_document_range(
    document: &str,
    range: Range<usize>,
    replacement: Option<&str>,
) -> String {
    let mut sections = Vec::new();
    let prefix = document[..range.start].trim();
    if !prefix.is_empty() {
        sections.push(prefix);
    }
    if let Some(replacement) = replacement.map(str::trim).filter(|value| !value.is_empty()) {
        sections.push(replacement);
    }
    let suffix = document[range.end..].trim();
    if !suffix.is_empty() {
        sections.push(suffix);
    }
    format!("{}\n", sections.join("\n\n"))
}

fn marked_agents_contract() -> String {
    format!(
        "{AGENTS_RHO_BEGIN}\n{}\n{AGENTS_RHO_END}\n",
        AGENTS_TEMPLATE.trim_end()
    )
}

fn merge_codex_config(existing: &str, canonical: &str, server_url: &str) -> MergeOutcome {
    if existing.contains(canonical.trim_end()) {
        return MergeOutcome::Unchanged;
    }

    match toml_table_body(existing, "mcp_servers.rho") {
        Some(body)
            if codex_http_config_targets(body, server_url)
                || codex_legacy_stdio_config_targets(body, server_url) =>
        {
            MergeOutcome::Update(replace_toml_namespace(
                existing,
                "mcp_servers.rho",
                canonical,
            ))
        }
        Some(_) => MergeOutcome::Conflict,
        None if has_toml_namespace(existing, "mcp_servers.rho") => MergeOutcome::Conflict,
        None => MergeOutcome::Update(append_document(existing, canonical)),
    }
}

fn codex_http_config_targets(section_body: &str, server_url: &str) -> bool {
    toml_string_assignment(section_body, "url").as_deref()
        == mcp_endpoint(server_url).ok().as_deref()
        && toml_string_assignment(section_body, "command").is_none()
}

fn codex_legacy_stdio_config_targets(section_body: &str, server_url: &str) -> bool {
    let command = toml_string_assignment(section_body, "command");
    let values =
        toml_double_quoted_strings(toml_assignment_value(section_body, "args").unwrap_or_default());
    let launches_rho_mcp = match command.as_deref() {
        Some("rho-mcp") => true,
        Some("cargo") => values.windows(2).any(|pair| pair == ["-p", "rho-mcp"]),
        _ => false,
    };
    let environment_default = format!("${{RHO_SERVER_URL:-{server_url}}}");
    let targets_server = values.windows(2).any(|pair| {
        pair[0] == "--server" && (pair[1] == server_url || pair[1] == environment_default)
    });
    let has_explicit_server = values.iter().any(|value| value == "--server");
    let uses_implicit_default = server_url == DEFAULT_RHO_SERVER_URL && !has_explicit_server;
    launches_rho_mcp && (targets_server || uses_implicit_default)
}

fn merge_claude_mcp(existing: &str, server_url: &str) -> Result<MergeOutcome> {
    let Ok(mut document) = serde_json::from_str::<serde_json::Value>(existing) else {
        return Ok(MergeOutcome::Conflict);
    };
    let Some(root) = document.as_object_mut() else {
        return Ok(MergeOutcome::Conflict);
    };
    let expected = claude_rho_mcp_entry(server_url)?;

    if !root.contains_key("mcpServers") {
        root.insert("mcpServers".to_string(), json!({"rho": expected}));
    } else {
        let Some(servers) = root
            .get_mut("mcpServers")
            .and_then(|value| value.as_object_mut())
        else {
            return Ok(MergeOutcome::Conflict);
        };
        match servers.get("rho") {
            Some(rho) if rho == &expected => {
                return Ok(MergeOutcome::Unchanged);
            }
            Some(rho) if claude_legacy_stdio_config_targets(rho, server_url) => {
                servers.insert("rho".to_string(), expected);
            }
            Some(_) => return Ok(MergeOutcome::Conflict),
            None => {
                servers.insert("rho".to_string(), expected);
            }
        }
    }

    Ok(MergeOutcome::Update(
        serde_json::to_string_pretty(&document)? + "\n",
    ))
}

fn claude_rho_mcp_entry(server_url: &str) -> Result<serde_json::Value> {
    Ok(json!({
        "type": "http",
        "url": mcp_endpoint(server_url)?
    }))
}

fn claude_legacy_stdio_config_targets(rho: &serde_json::Value, server_url: &str) -> bool {
    let Some(object) = rho.as_object() else {
        return false;
    };
    let command_launches_rho =
        object.get("command").and_then(|value| value.as_str()) == Some("rho-mcp");
    let args = object
        .get("args")
        .and_then(|value| value.as_array())
        .map(|args| {
            args.iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let args_launch_rho = args.contains(&"rho-mcp");
    let environment_default = format!("${{RHO_SERVER_URL:-{server_url}}}");
    let args_target_server = args.windows(2).any(|pair| {
        pair[0] == "--server" && (pair[1] == server_url || pair[1] == environment_default)
    });
    let environment_targets_server = object
        .get("env")
        .and_then(|value| value.get("RHO_SERVER_URL"))
        .and_then(|value| value.as_str())
        == Some(server_url);
    let has_explicit_server = args.contains(&"--server");
    let has_environment_server = object
        .get("env")
        .and_then(|value| value.get("RHO_SERVER_URL"))
        .is_some();
    let uses_implicit_default =
        server_url == DEFAULT_RHO_SERVER_URL && !has_explicit_server && !has_environment_server;
    (command_launches_rho || args_launch_rho)
        && (args_target_server || environment_targets_server || uses_implicit_default)
}

fn mcp_endpoint(server_url: &str) -> Result<String> {
    let mut base = Url::parse(server_url).context("parsing Rho server URL")?;
    ensure!(
        matches!(base.scheme(), "http" | "https"),
        "Rho server URL must use http or https"
    );
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    Ok(base.join("mcp")?.to_string())
}

fn append_document(existing: &str, addition: &str) -> String {
    let mut merged = existing.to_string();
    if !merged.is_empty() {
        if !merged.ends_with('\n') {
            merged.push('\n');
        }
        if !merged.ends_with("\n\n") {
            merged.push('\n');
        }
    }
    merged.push_str(addition.trim_start_matches('\n'));
    if !merged.ends_with('\n') {
        merged.push('\n');
    }
    merged
}

fn toml_table_body<'a>(document: &'a str, target: &str) -> Option<&'a str> {
    let mut body_start = None;
    let mut offset = 0;
    for line in document.split_inclusive('\n') {
        if let Some(header) = toml_table_header(line) {
            if let Some(start) = body_start {
                return Some(&document[start..offset]);
            }
            if header == target {
                body_start = Some(offset + line.len());
            }
        }
        offset += line.len();
    }
    body_start.map(|start| &document[start..])
}

fn replace_toml_namespace(document: &str, target: &str, replacement: &str) -> String {
    let mut kept = String::new();
    let mut skipping = false;
    for line in document.split_inclusive('\n') {
        if let Some(header) = toml_table_header(line) {
            skipping = header == target
                || header
                    .strip_prefix(target)
                    .is_some_and(|suffix| suffix.starts_with('.'));
        }
        if !skipping {
            kept.push_str(line);
        }
    }
    append_document(kept.trim_end(), replacement)
}

fn toml_assignment_value<'a>(section_body: &'a str, key: &str) -> Option<&'a str> {
    let mut offset = 0;
    let mut value_start = None;
    let mut bracket_depth = 0_i32;
    for line in section_body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if value_start.is_none() {
            if trimmed.starts_with('#') {
                offset += line.len();
                continue;
            }
            let Some((candidate, value)) = trimmed.split_once('=') else {
                offset += line.len();
                continue;
            };
            if candidate.trim() != key {
                offset += line.len();
                continue;
            }
            let leading = line.len() - trimmed.len() + candidate.len() + 1;
            value_start = Some(offset + leading);
            bracket_depth = bracket_delta(value);
            if bracket_depth <= 0 {
                return Some(value.trim());
            }
        } else {
            bracket_depth += bracket_delta(line);
            if bracket_depth <= 0 {
                let start = value_start?;
                return Some(section_body[start..offset + line.len()].trim());
            }
        }
        offset += line.len();
    }
    value_start.map(|start| section_body[start..].trim())
}

fn bracket_delta(value: &str) -> i32 {
    value.bytes().fold(0_i32, |depth, byte| match byte {
        b'[' => depth + 1,
        b']' => depth - 1,
        _ => depth,
    })
}

fn toml_string_assignment(section_body: &str, key: &str) -> Option<String> {
    let value = toml_assignment_value(section_body, key)?;
    let mut strings = toml_double_quoted_strings(value);
    (strings.len() == 1).then(|| strings.remove(0))
}

fn has_toml_namespace(document: &str, target: &str) -> bool {
    document
        .lines()
        .filter_map(toml_table_header)
        .any(|header| {
            header == target
                || header
                    .strip_prefix(target)
                    .is_some_and(|suffix| suffix.starts_with('.'))
        })
}

fn toml_table_header(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') || trimmed.starts_with("[[") {
        return None;
    }
    let close = trimmed.find(']')?;
    let remainder = trimmed[close + 1..].trim();
    if !remainder.is_empty() && !remainder.starts_with('#') {
        return None;
    }
    Some(trimmed[1..close].trim())
}

fn toml_double_quoted_strings(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut values = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'#' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'"' => {
                let start = index;
                index += 1;
                let mut escaped = false;
                while index < bytes.len() {
                    match bytes[index] {
                        b'"' if !escaped => {
                            index += 1;
                            if let Ok(value) = serde_json::from_str::<String>(&input[start..index])
                            {
                                values.push(value);
                            }
                            break;
                        }
                        b'\\' if !escaped => escaped = true,
                        _ => escaped = false,
                    }
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    values
}

fn ensure_no_symlinks(project_root: &Path, relative_path: &Path) -> Result<()> {
    let mut current = PathBuf::from(project_root);
    for component in relative_path.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink(),
                "refusing to use symlink {}",
                current.display()
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => break,
            Err(error) => {
                return Err(error).with_context(|| format!("inspecting {}", current.display()));
            }
        }
    }
    Ok(())
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
    let quoted_mcp_endpoint = serde_json::to_string(&mcp_endpoint(server_url)?)?;
    let mut files = vec![ScaffoldFile {
        relative_path: "AGENTS.md",
        content: AGENTS_TEMPLATE.to_string(),
        merge_strategy: MergeStrategy::AgentsContract,
    }];
    match client {
        AgentClient::Codex => {
            files.push(ScaffoldFile {
                relative_path: ".codex/config.toml",
                content: format!(
                    r#"[mcp_servers.rho]
url = {quoted_mcp_endpoint}
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
                merge_strategy: MergeStrategy::CodexConfig,
            });
            files.push(ScaffoldFile {
                relative_path: ".agents/skills/operate-rho-runtime/SKILL.md",
                content: CODEX_SKILL.to_string(),
                merge_strategy: MergeStrategy::Exact,
            });
            files.push(ScaffoldFile {
                relative_path: ".agents/skills/operate-rho-runtime/agents/openai.yaml",
                content: CODEX_OPENAI_YAML.to_string(),
                merge_strategy: MergeStrategy::Exact,
            });
            files.push(ScaffoldFile {
                relative_path:
                    ".agents/skills/operate-rho-runtime/references/workbench-protocol.md",
                content: CODEX_PROTOCOL_REFERENCE.to_string(),
                merge_strategy: MergeStrategy::Exact,
            });
        }
        AgentClient::Claude => {
            files.push(ScaffoldFile {
                relative_path: ".mcp.json",
                content: serde_json::to_string_pretty(&json!({
                    "mcpServers": {
                        "rho": claude_rho_mcp_entry(server_url)?
                    }
                }))? + "\n",
                merge_strategy: MergeStrategy::ClaudeMcp,
            });
            files.push(ScaffoldFile {
                relative_path: ".claude/skills/rho-runtime/SKILL.md",
                content: CLAUDE_SKILL.to_string(),
                merge_strategy: MergeStrategy::Exact,
            });
        }
    }
    Ok(files)
}

const AGENTS_TEMPLATE: &str = include_str!("../../../integrations/templates/AGENTS.md");
const DEFAULT_RHO_SERVER_URL: &str = "http://127.0.0.1:8787";
const AGENTS_RHO_BEGIN: &str = "<!-- BEGIN RHO SCIENTIFIC RUNTIME CONTRACT -->";
const AGENTS_RHO_END: &str = "<!-- END RHO SCIENTIFIC RUNTIME CONTRACT -->";
const AGENTS_RHO_HEADING: &str = "# Rho scientific runtime contract";
const AGENTS_RHO_INTRO: &str = "This project uses Rho as the source of truth for R state.";
const AGENTS_RHO_AGENT_RULE: &str =
    "- The Agent reasons, converses, and plans. Rho executes and proves.";
const AGENTS_RHO_FINAL_SENTENCE: &str =
    "Use absolute project/script paths and the project root plus workspace ID reported by Rho.";
const AGENTS_RHO_LEGACY_FINAL_SENTENCE: &str =
    "Their presence helps discovery but does not replace the workspace identity returned by Rho.";
const CODEX_SKILL: &str = include_str!("../../../.agents/skills/operate-rho-runtime/SKILL.md");
const CODEX_OPENAI_YAML: &str =
    include_str!("../../../.agents/skills/operate-rho-runtime/agents/openai.yaml");
const CODEX_PROTOCOL_REFERENCE: &str =
    include_str!("../../../.agents/skills/operate-rho-runtime/references/workbench-protocol.md");
const CLAUDE_SKILL: &str = include_str!("../../../.claude/skills/rho-runtime/SKILL.md");

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
    fn bootstraps_codex_idempotently() {
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
        assert_eq!(report.created, report.files);
        assert!(report.updated.is_empty());
        assert!(report.unchanged.is_empty());
        assert!(report.preserved.is_empty());
        assert!(report.overwritten.is_empty());
        assert!(directory.path().join("AGENTS.md").is_file());
        assert!(directory.path().join(".codex/config.toml").is_file());
        assert!(
            directory
                .path()
                .join(".agents/skills/operate-rho-runtime/SKILL.md")
                .is_file()
        );
        assert!(
            directory
                .path()
                .join(".agents/skills/operate-rho-runtime/references/workbench-protocol.md")
                .is_file()
        );
        let skill = fs::read_to_string(
            directory
                .path()
                .join(".agents/skills/operate-rho-runtime/SKILL.md"),
        )
        .unwrap();
        assert_eq!(skill, CODEX_SKILL);
        let config = fs::read_to_string(directory.path().join(".codex/config.toml")).unwrap();
        assert!(config.contains("url = \"http://127.0.0.1:9999/mcp\""));
        assert!(!config.contains("command ="));
        assert!(!config.contains("cwd ="));
        let second = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        assert!(second.created.is_empty());
        assert!(second.updated.is_empty());
        assert_eq!(second.unchanged, second.files);
        assert!(second.preserved.is_empty());
        assert!(second.overwritten.is_empty());
    }

    #[test]
    fn appends_one_marked_agents_contract_without_replacing_project_guidance() {
        let directory = TempDir::new().unwrap();
        let agents_path = directory.path().join("AGENTS.md");
        fs::write(&agents_path, "# Project guidance\n\nKeep this text.\n").unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();

        assert_eq!(report.updated, vec!["AGENTS.md"]);
        let merged = fs::read_to_string(&agents_path).unwrap();
        assert!(merged.starts_with("# Project guidance\n\nKeep this text.\n"));
        assert_eq!(merged.matches(AGENTS_RHO_BEGIN).count(), 1);
        assert_eq!(merged.matches(AGENTS_RHO_END).count(), 1);
        assert!(merged.contains(AGENTS_TEMPLATE.trim()));

        let second = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        assert!(second.updated.is_empty());
        assert_eq!(second.unchanged, second.files);
        assert_eq!(fs::read_to_string(agents_path).unwrap(), merged);
    }

    #[test]
    fn migrates_an_unmarked_agents_contract_without_duplication() {
        let directory = TempDir::new().unwrap();
        let agents_path = directory.path().join("AGENTS.md");
        let existing = format!("# Existing preface\n\n{AGENTS_TEMPLATE}\nKeep this footer.\n");
        fs::write(&agents_path, &existing).unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            DEFAULT_RHO_SERVER_URL,
            false,
        )
        .unwrap();

        assert_eq!(report.updated, vec!["AGENTS.md"]);
        let migrated = fs::read_to_string(&agents_path).unwrap();
        assert!(migrated.starts_with("# Existing preface"));
        assert!(migrated.ends_with("Keep this footer.\n"));
        assert_eq!(migrated.matches(AGENTS_RHO_BEGIN).count(), 1);
        assert_eq!(migrated.matches(AGENTS_RHO_END).count(), 1);
        assert_eq!(migrated.matches(AGENTS_RHO_HEADING).count(), 1);

        let second = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            DEFAULT_RHO_SERVER_URL,
            false,
        )
        .unwrap();
        assert!(second.updated.is_empty());
        assert_eq!(second.unchanged, second.files);
        assert_eq!(fs::read_to_string(agents_path).unwrap(), migrated);
    }

    #[test]
    fn removes_a_legacy_unmarked_contract_next_to_the_managed_contract() {
        let directory = TempDir::new().unwrap();
        let agents_path = directory.path().join("AGENTS.md");
        let legacy = AGENTS_TEMPLATE.replace(
            "- During setup, read and report Rho's dependency status; do not install R, run `sudo`, invoke dependency mutations, or launch substitute R/Ark processes.\n",
            "",
        );
        fs::write(
            &agents_path,
            format!("{legacy}\n{}", marked_agents_contract()),
        )
        .unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            DEFAULT_RHO_SERVER_URL,
            false,
        )
        .unwrap();

        assert_eq!(report.updated, vec!["AGENTS.md"]);
        let migrated = fs::read_to_string(&agents_path).unwrap();
        assert_eq!(migrated.matches(AGENTS_RHO_BEGIN).count(), 1);
        assert_eq!(migrated.matches(AGENTS_RHO_END).count(), 1);
        assert_eq!(migrated.matches(AGENTS_RHO_HEADING).count(), 1);
        assert!(migrated.contains(AGENTS_TEMPLATE.trim()));
    }

    #[test]
    fn merges_codex_config_idempotently_and_preserves_conflicting_rho_section() {
        let directory = TempDir::new().unwrap();
        let config_directory = directory.path().join(".codex");
        fs::create_dir(&config_directory).unwrap();
        let config_path = config_directory.join("config.toml");
        fs::write(&config_path, "[features]\nexperimental = true\n").unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        assert_eq!(report.updated, vec![".codex/config.toml"]);
        let merged = fs::read_to_string(&config_path).unwrap();
        assert!(merged.starts_with("[features]\nexperimental = true\n"));
        assert_eq!(merged.matches("[mcp_servers.rho]").count(), 1);

        let second = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        assert!(second.updated.is_empty());
        assert_eq!(second.unchanged, second.files);
        assert_eq!(fs::read_to_string(&config_path).unwrap(), merged);

        let conflicting = "[features]\nexperimental = true\n\n[mcp_servers.rho]\ncommand = \"rho-mcp\"\nargs = [\"--server\", \"https://different.example.test\"]\n";
        fs::write(&config_path, conflicting).unwrap();
        let conflict_report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        assert!(conflict_report.updated.is_empty());
        assert_eq!(conflict_report.preserved, vec![".codex/config.toml"]);
        assert_eq!(fs::read_to_string(config_path).unwrap(), conflicting);
    }

    #[test]
    fn migrates_source_checkout_codex_mcp_configuration_to_http() {
        let canonical = scaffold_files(AgentClient::Codex, "http://127.0.0.1:9999")
            .unwrap()
            .remove(1)
            .content;
        let source_checkout = r#"[mcp_servers.rho]
command = "cargo"
args = [
  "run",
  "--quiet",
  "-p",
  "rho-mcp",
  "--",
  "--server",
  "http://127.0.0.1:9999",
]
"#;

        assert_eq!(
            merge_codex_config(source_checkout, &canonical, "http://127.0.0.1:9999"),
            MergeOutcome::Update(canonical)
        );
    }

    #[test]
    fn migrates_only_the_owned_codex_namespace_and_rejects_bogus_launchers() {
        let canonical = scaffold_files(AgentClient::Codex, "http://127.0.0.1:9999")
            .unwrap()
            .remove(1)
            .content;
        let legacy = r#"# keep this comment
[features]
experimental = true

[mcp_servers.rho]
command = "cargo"
args = ["run", "--quiet", "-p", "rho-mcp", "--", "--server", "http://127.0.0.1:9999"]
cwd = "."

[mcp_servers.rho.tools.workspace_execute]
approval_mode = "prompt"

[mcp_servers.other]
command = "other-mcp"
"#;
        let MergeOutcome::Update(migrated) =
            merge_codex_config(legacy, &canonical, "http://127.0.0.1:9999")
        else {
            panic!("legacy Rho configuration was not migrated");
        };
        assert!(migrated.contains("# keep this comment"));
        assert!(migrated.contains("[features]"));
        assert!(migrated.contains("[mcp_servers.other]"));
        assert!(migrated.contains("url = \"http://127.0.0.1:9999/mcp\""));
        assert!(!migrated.contains("cwd = \".\""));
        assert!(!migrated.contains("command = \"cargo\""));
        assert_eq!(migrated.matches("[mcp_servers.rho]").count(), 1);

        let bogus = r#"[mcp_servers.rho]
command = "echo"
args = ["rho-mcp", "--server", "http://127.0.0.1:9999"]
"#;
        assert_eq!(
            merge_codex_config(bogus, &canonical, "http://127.0.0.1:9999"),
            MergeOutcome::Conflict
        );
    }

    #[test]
    fn creates_missing_files_while_merging_safe_codex_config() {
        let directory = TempDir::new().unwrap();
        bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();

        let config_path = directory.path().join(".codex/config.toml");
        fs::write(&config_path, "# user-owned config\n").unwrap();
        let skill_path = directory
            .path()
            .join(".agents/skills/operate-rho-runtime/SKILL.md");
        fs::remove_file(&skill_path).unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();

        assert_eq!(
            report.created,
            vec![".agents/skills/operate-rho-runtime/SKILL.md"]
        );
        assert_eq!(report.updated, vec![".codex/config.toml"]);
        assert!(report.preserved.is_empty());
        assert_eq!(
            report.unchanged,
            vec![
                "AGENTS.md",
                ".agents/skills/operate-rho-runtime/agents/openai.yaml",
                ".agents/skills/operate-rho-runtime/references/workbench-protocol.md",
            ]
        );
        assert!(report.overwritten.is_empty());
        let merged_config = fs::read_to_string(config_path).unwrap();
        assert!(merged_config.starts_with("# user-owned config\n\n"));
        assert!(merged_config.contains("[mcp_servers.rho]"));
        assert!(merged_config.contains("http://127.0.0.1:9999"));
        assert_eq!(fs::read_to_string(skill_path).unwrap(), CODEX_SKILL);
    }

    #[test]
    fn force_overwrites_only_differing_regular_files() {
        let directory = TempDir::new().unwrap();
        bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            false,
        )
        .unwrap();
        let config_path = directory.path().join(".codex/config.toml");
        fs::write(
            &config_path,
            "[mcp_servers.rho]\ncommand = \"rho-mcp\"\nargs = [\"--server\", \"https://different.example.test\"]\n",
        )
        .unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            true,
        )
        .unwrap();

        assert!(report.created.is_empty());
        assert!(report.updated.is_empty());
        assert!(report.preserved.is_empty());
        assert_eq!(report.overwritten, vec![".codex/config.toml"]);
        assert_eq!(report.unchanged.len(), report.files.len() - 1);
        assert!(
            fs::read_to_string(config_path)
                .unwrap()
                .contains("http://127.0.0.1:9999")
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinks_before_creating_any_files() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let agents_path = directory.path().join("AGENTS.md");
        fs::write(&agents_path, "# Existing project instructions\n").unwrap();
        symlink(outside.path(), directory.path().join(".codex")).unwrap();

        let error = bootstrap_project(
            directory.path(),
            AgentClient::Codex,
            "http://127.0.0.1:9999",
            true,
        )
        .unwrap_err();

        assert!(error.to_string().contains("symlink"));
        assert_eq!(
            fs::read_to_string(agents_path).unwrap(),
            "# Existing project instructions\n"
        );
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
    }

    #[test]
    fn bootstraps_valid_claude_mcp_json() {
        let directory = TempDir::new().unwrap();
        let report = bootstrap_project(
            directory.path(),
            AgentClient::Claude,
            "https://rho.example.test/base",
            false,
        )
        .unwrap();
        assert_eq!(report.created, report.files);
        assert!(report.updated.is_empty());
        assert!(report.unchanged.is_empty());
        assert!(report.preserved.is_empty());
        assert!(report.overwritten.is_empty());
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.path().join(".mcp.json")).unwrap()).unwrap();
        assert_eq!(
            value["mcpServers"]["rho"]["url"],
            "https://rho.example.test/base/mcp"
        );
        assert_eq!(value["mcpServers"]["rho"]["type"], "http");
        assert!(
            directory
                .path()
                .join(".claude/skills/rho-runtime/SKILL.md")
                .is_file()
        );
    }

    #[test]
    fn merges_claude_mcp_json_idempotently_and_preserves_conflicting_rho_server() {
        let directory = TempDir::new().unwrap();
        let mcp_path = directory.path().join(".mcp.json");
        fs::write(
            &mcp_path,
            r#"{
  "projectSetting": true,
  "mcpServers": {
    "existing": {"command": "existing-mcp"}
  }
}
"#,
        )
        .unwrap();

        let report = bootstrap_project(
            directory.path(),
            AgentClient::Claude,
            "https://rho.example.test/base",
            false,
        )
        .unwrap();
        assert_eq!(report.updated, vec![".mcp.json"]);
        let merged = fs::read_to_string(&mcp_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["projectSetting"], true);
        assert_eq!(value["mcpServers"]["existing"]["command"], "existing-mcp");
        assert_eq!(
            value["mcpServers"]["rho"]["url"],
            "https://rho.example.test/base/mcp"
        );

        let second = bootstrap_project(
            directory.path(),
            AgentClient::Claude,
            "https://rho.example.test/base",
            false,
        )
        .unwrap();
        assert!(second.updated.is_empty());
        assert_eq!(second.unchanged, second.files);
        assert_eq!(fs::read_to_string(&mcp_path).unwrap(), merged);

        let mut conflicting: serde_json::Value = serde_json::from_str(&merged).unwrap();
        conflicting["mcpServers"]["rho"]["url"] =
            serde_json::Value::String("https://different.example.test/mcp".to_string());
        let conflicting = serde_json::to_string_pretty(&conflicting).unwrap() + "\n";
        fs::write(&mcp_path, &conflicting).unwrap();
        let conflict_report = bootstrap_project(
            directory.path(),
            AgentClient::Claude,
            "https://rho.example.test/base",
            false,
        )
        .unwrap();
        assert!(conflict_report.updated.is_empty());
        assert_eq!(conflict_report.preserved, vec![".mcp.json"]);
        assert_eq!(fs::read_to_string(mcp_path).unwrap(), conflicting);
    }

    #[test]
    fn packaged_mcp_configs_use_the_cwd_independent_http_transport() {
        let configurations = [
            include_str!("../../../.mcp.json"),
            include_str!("../../../integrations/codex/plugins/rho/.mcp.json"),
        ];

        for configuration in configurations {
            let value: serde_json::Value = serde_json::from_str(configuration).unwrap();
            let rho = &value["mcpServers"]["rho"];
            assert_eq!(rho, &claude_rho_mcp_entry(DEFAULT_RHO_SERVER_URL).unwrap());
        }
    }

    #[test]
    fn codex_skill_assets_match_packaged_plugin() {
        assert_eq!(
            CODEX_SKILL,
            include_str!(
                "../../../integrations/codex/plugins/rho/skills/operate-rho-runtime/SKILL.md"
            )
        );
        assert_eq!(
            CODEX_OPENAI_YAML,
            include_str!(
                "../../../integrations/codex/plugins/rho/skills/operate-rho-runtime/agents/openai.yaml"
            )
        );
        assert_eq!(
            CODEX_PROTOCOL_REFERENCE,
            include_str!(
                "../../../integrations/codex/plugins/rho/skills/operate-rho-runtime/references/workbench-protocol.md"
            )
        );
    }
}
