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

Start the durable server from the project root:

```sh
cargo run -p rho-server -- serve --project-root .
```

Managed mode is the default. Rho discovers and reuses a compatible local R, prepares the checksum-pinned Ark release in an immutable cross-project cache, materializes the embedded `rho.bridge`, writes a project-controlled binding under `.rho/runtime/`, and attaches the authoritative Workspace R. The control plane comes online first so the browser can show dependency progress and any required human action.

Use control-plane-only mode explicitly when developing protocol clients without Workspace R:

```sh
cargo run -p rho-server -- serve --project-root . --control-plane-only
```

`--kernelspec /path/to/kernel.json` remains an advanced override for an administrator-provided Ark binding; it is not required for normal startup.

Open <http://127.0.0.1:8787>. The browser exposes Runs, Objects, Plots, Problems, Approvals, and Provenance; it deliberately does not recreate a full IDE.

## Runtime dependencies

The `rho deps` commands work without a running server and return stable JSON for clients:

```sh
rho deps status --project . --json
rho deps ensure --project . --json
rho deps repair --project . --json
rho deps cache-path --project . --json
```

`status` is read-only. `ensure` prepares non-privileged Rho-owned components and the project binding; `repair` re-verifies and replaces an invalid managed Ark cache entry. Existing compatible R installations are reused, including an explicit `RHO_RSCRIPT`. Rho never silently runs `sudo` or lets an Agent install or replace R during onboarding. If R is absent, the dependency report returns `action_required`; after explicit user approval, `rho deps ensure --install-r` can prepare a checksum-verified official installer, but a human must open it, complete the operating-system prompts, and retry. Platforms without a supported installer report the system-provider action instead.

## One-prompt Agent setup

Open the browser control plane and choose **Connect Agent**. Rho generates one sentence containing the exact project root and runtime URL; paste it into Codex, Claude Code, or another Agent. The Agent reads the setup contract published by that runtime, installs Rho's official `operate-rho-runtime` skill, merges a direct `URL/mcp` HTTP entry without deleting existing project settings, and reports project configuration, current-session MCP, control-plane health, dependency readiness, and Workspace R readiness separately. Agent setup only configures the client and reads status; Rho's Dependency Manager owns R, Ark, the bridge, and the project binding. MCP startup therefore does not depend on Cargo, `rho-mcp` on `PATH`, or the terminal directory from which the Agent was launched.

The self-describing endpoints are available from every local runtime and remote gateway:

```text
/agent-setup.md
/agent/skills/operate-rho-runtime/SKILL.md
```

This keeps onboarding coupled to the running Rho version instead of a possibly stale external tutorial. If Codex was opened outside the project, setup finishes the files and hands off to `codex -C /absolute/project/path`; project skills and MCP configuration are discovered by a new project-scoped session.

## Agent and CLI access

This repository includes project-scoped integrations for both Codex and Claude Code:

- Codex: `.codex/config.toml` and `.agents/skills/operate-rho-runtime`
- Claude Code: `.mcp.json`, `.claude/skills/rho-runtime`, and the distributable plugin in `integrations/claude-code/rho`
- Shared instructions: `integrations/templates/AGENTS.md`

Official Rho packages include the `rho` bootstrap/diagnostic CLI. Bootstrap any R project with an absolute project path:

```sh
rho init codex --project /path/to/project
rho init claude --project /path/to/project
```

Source contributors can replace `rho` with `cargo run --quiet --manifest-path /absolute/path/to/Rho/Cargo.toml -p rho-cli --`. The absolute manifest keeps development commands independent of shell cwd; end users do not need Rust.

The bootstrap detects `.Rproj`, `DESCRIPTION`, `renv.lock`, `.Rprofile`, and `R/` markers. The official skill is embedded in the CLI, including its protocol reference and Codex metadata. Bootstrap is idempotent: it reports `created`, `updated`, `unchanged`, and `preserved` paths, merges Rho into existing Agent/MCP configuration, and leaves conflicting Rho-owned entries untouched unless `--force` is explicit.

Verify the three runtime health layers in one call:

```bash
rho --server http://127.0.0.1:8787 doctor --json
```

`control_plane_ready` reports the Workbench API, `dependencies` reports Rho's R/Ark/bridge/binding preparation, and `workspace_r_ready` reports whether the authoritative executor is attached and available. Lifecycle `disconnected` means the control plane is healthy but Workspace R is not attached; inspect `dependencies` to distinguish preparation or a required human action from intentional `--control-plane-only` mode. It does not mean Agent registration failed.

Rho also ships an official Codex plugin bundle containing the same skill and HTTP MCP configuration. From a Rho source checkout:

```bash
codex plugin marketplace add /absolute/path/to/Rho/integrations/codex
codex plugin add rho@rho-official
```

Start a new Codex session after installing or updating the plugin.

The CLI has stable JSON envelopes for Agent use:

```sh
rho status --json
rho run analysis.R --json
rho objects --json
rho inspect OBJECT --json
rho problems --json
rho plots list --json
# or pipe approved code without creating a cwd-relative script
printf 'summary(iris)\n' | rho run - --json
```

The built-in HTTP MCP endpoint and the optional `rho-mcp` stdio compatibility adapter expose scientific semantics rather than Ark or internal broker RPC: `workspace_open`, `workspace_status`, `workspace_execute`, `object_inspect`, `run_history`, `problem_list`, `artifact_export`, and `plot_view`.

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
