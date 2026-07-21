# Rho

Rho is an Agent-native scientific runtime for R.

Use Claude Code, Codex, Copilot, or any MCP-capable Agent for reasoning and planning. Rho provides the trusted scientific workspace underneath: persistent R state, semantic object inspection, policy-controlled execution, artifacts, and provenance.

> Agent thinks. Rho executes and proves.

## Product boundary

Rho is not another general AI IDE and the browser is not the source of truth.

```text
Codex · Claude Code · MCP clients · rho CLI · Browser · Desktop wrapper
                              │
                  Rho Workbench Protocol 0.1
                    HTTP · WebSocket · JSON
                              │
                         rho-server
             lifecycle · policy · events · provenance
                              │
                    authoritative Workspace R
                              │
                             Ark
```

The runtime persists independently of browser refreshes and Agent reconnections. Every client projects the same `Workspace`, `Run`, `Object`, `Artifact`, `Problem`, `Approval`, and `Provenance` entities.

## Start a workspace

The control plane can start without a kernel, which is useful for protocol and client development:

```sh
cargo run -p rho-server -- serve --project-root .
```

For live R execution, attach the intended Ark kernelspec:

```sh
cargo run -p rho-server -- serve \
  --project-root . \
  --kernelspec /path/to/ark/kernel.json
```

Open <http://127.0.0.1:8787>. The browser exposes Runs, Objects, Plots, Problems, Approvals, and Provenance; it deliberately does not recreate a full IDE.

## Agent and CLI access

This repository includes project-scoped integrations for both Codex and Claude Code:

- Codex: `.codex/config.toml` and `.agents/skills/operate-rho-runtime`
- Claude Code: `.mcp.json`, `.claude/skills/rho-runtime`, and the distributable plugin in `integrations/claude-code/rho`
- Shared instructions: `integrations/templates/AGENTS.md`

Bootstrap another R project after installing the two client binaries:

```sh
cargo install --path crates/rho-cli
cargo install --path crates/rho-mcp

rho init codex --project /path/to/project
rho init claude --project /path/to/project
```

The bootstrap detects `.Rproj`, `DESCRIPTION`, `renv.lock`, `.Rprofile`, and `R/` markers. It refuses to overwrite existing Agent configuration unless `--force` is explicit.

The CLI has stable JSON envelopes for Agent use:

```sh
rho status --json
rho run analysis.R --json
rho objects --json
rho inspect OBJECT --json
rho problems --json
rho plots list --json
```

`rho-mcp` exposes scientific semantics rather than Ark or internal broker RPC: `workspace_open`, `workspace_status`, `workspace_execute`, `object_inspect`, `run_history`, `problem_list`, `artifact_export`, and `plot_view`.

## Scientific semantics and evidence

Rho provides bounded viewers for data frames, Seurat, SummarizedExperiment, SingleCellExperiment, GRanges, and ggplot objects. A provenance graph connects code, parameters, runtime environment, state revisions, objects, artifacts, and execution times.

Inspection, summarization, and visualization are read-only. Mutating R execution goes through the same policy model whether it originates in the browser, CLI, MCP host, or an internal Agent action. Package installation, file overwrite, shell execution, and data upload remain protected action classes.

## Remote and desktop projections

Project a remote runtime through a local same-origin browser/API gateway:

```sh
cargo run -p rho-server -- gateway \
  --upstream https://rho.example.org \
  --port 8787
```

The gateway forwards Workbench HTTP and WebSocket traffic while serving the same browser control plane locally. Runtime identity and scientific state remain upstream. Deployment profiles for a local machine, SSH Linux server, scheduler job, and cloud endpoint are documented in [docs/remote-runtime.md](docs/remote-runtime.md).

The optional Tauri desktop app is only a launcher: it starts `rho-server`, embeds the browser control plane, supplies a tray, startup notifications, and `.Rproj` file association. It contains no separate editor, project state, or scientific execution path.

## Verify the workspace

```sh
cargo test --workspace
Rscript -e 'testthat::test_local("r/rho.bridge", reporter = "summary")'
node --check web/app.js
```

Architecture decisions are in [docs/decisions](docs/decisions/README.md), the client workflow is in [docs/integrations/agent-workflows.md](docs/integrations/agent-workflows.md), and release changes are in [NEWS.md](NEWS.md).
