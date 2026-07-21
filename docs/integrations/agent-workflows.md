# Agent workflows with Rho

Rho does not replace a general Agent client. Claude Code, Codex, Copilot, or another MCP client handles reasoning and planning; Rho owns the persistent R workspace, safe execution, scientific semantics, and provenance.

## Start the runtime

From the Rho source tree, start the durable server and attach the intended Ark kernelspec:

```sh
cargo run -p rho-server -- serve \
  --project-root /path/to/project \
  --kernelspec /path/to/ark/kernel.json
```

The browser control plane is available at `http://127.0.0.1:8787`. A client reconnect does not terminate Workspace R.

## Codex

This repository checks in `.codex/config.toml` and `.agents/skills/operate-rho-runtime`. Trust the repository, restart Codex after changing MCP configuration, and invoke `$operate-rho-runtime` when you want the workflow explicitly.

For another R project, install `rho-mcp` on `PATH`, copy the AGENTS template, skill, and MCP table, then point the server argument at its runtime. The repository config uses `cargo run -p rho-mcp` because it is also the Rho source tree.

## Claude Code

The repository-level `.mcp.json` and `.claude/skills/rho-runtime` are project-scoped. Claude Code asks for approval before enabling a checked-in MCP server. Alternatively, load the distributable plugin:

```sh
cargo install --path crates/rho-mcp
claude --plugin-dir ./integrations/claude-code/rho
```

Use `/mcp` to inspect the connection and `/rho:rho-runtime` to invoke the plugin workflow explicitly. Set `RHO_SERVER_URL` for a non-default endpoint.

## Example scientific turn

Request:

> Inspect `pbmc`, summarize its dimensions and metadata, run the requested QC transformation, show new problems and plots, and give me the provenance IDs.

Expected client flow:

1. `workspace_open` discovers the workspace and `pbmc`.
2. `object_inspect` reads bounded Seurat semantics before mutation.
3. The Agent explains the exact R code and `workspace_execute` goes through client approval.
4. `run_history` and `problem_list` verify execution.
5. `object_inspect` and `plot_view` verify the new state and artifact.
6. The answer includes stable workspace, run, object, artifact, and provenance identifiers.

This same flow works over the CLI with `rho status --json`, `rho objects --json`, `rho inspect pbmc --json`, `rho run qc.R --json`, `rho problems --json`, and `rho plots list --json`.
