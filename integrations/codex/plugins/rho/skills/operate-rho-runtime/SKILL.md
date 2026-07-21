---
name: operate-rho-runtime
description: Connect to and operate Rho's authoritative scientific runtime for R workspaces. Use for Rho setup, dependency and connection checks, or when inspecting and executing R code, live objects, runs, plots, problems, artifacts, approvals, and provenance; prefer the bundled Rho MCP tools for scientific state and use the JSON CLI for diagnostics and dependency actions.
---

# Operate Rho Runtime

Treat Rho as the source of truth for scientific state. Let the Agent reason and plan; let Rho execute and prove.

## Connect

1. Use the runtime URL supplied by the user, then `RHO_SERVER_URL`, then `http://127.0.0.1:8787`.
2. Call `workspace_open` once. Rho's primary Agent transport is the runtime's Streamable HTTP MCP endpoint at `URL/mcp`; it does not depend on the Agent's shell directory, a project-local Cargo build, or `rho-mcp` on `PATH`. If the tool is unavailable, distinguish “project MCP configuration was not loaded in this Agent session” from “runtime unreachable”; do not retry or start duplicate servers in a loop.
3. Resolve a CLI prefix once before using a CLI fallback. Prefer the absolute path returned by `command -v rho`. In the Rho source checkout only, use `cargo run --quiet --manifest-path ABSOLUTE_RHO_SOURCE/Cargo.toml -p rho-cli --`. Otherwise mark the CLI unavailable; do not search build directories or assume a bare `rho` exists. Use the resolved prefix with `--server URL doctor --json`, or make one read-only `GET URL/healthz` check when no CLI is available. Read `control_plane_ready`, `dependencies`, and `workspace_r_ready`/`lifecycle` independently.
4. When the user asks to install or configure Rho, read `URL/agent-setup.md`. Preserve existing configuration, accept an idempotent `rho init`, and state clearly when a new Agent session in the project is required.

## Dependencies

Let Rho's Dependency Manager own R discovery, the checksum-pinned Ark cache, the embedded bridge, and the project-controlled binding. During Agent onboarding, only read the `dependencies` summary; when details are necessary, run `rho deps status --project PROJECT --json` once. Report `preparing`, `action_required`, or `failed` with the issue code and action URL.

When the user explicitly requests dependency work, inspect first. Use `rho deps ensure` to prepare non-privileged managed components, `rho deps repair` to replace an invalid managed Ark cache entry, and `rho deps cache-path` to locate the immutable cross-project cache. Explain the mutation and obtain any required approval. If R is missing, report the verified installer or system-provider action; never install R, run `sudo`, or complete an operating-system installer on the user's behalf.

## Scientific workflow

1. Inspect the workspace, relevant objects, runs, problems, and plots before changing state.
2. Explain the requested state change and call `workspace_execute`. Honor approval prompts.
3. Verify the run, problems, changed objects, plots, artifacts, and provenance.
4. Report stable workspace, run, object, artifact, and provenance identifiers.

If MCP is unavailable but the resolved CLI works and reports Workspace R ready, continue the requested workflow through the JSON CLI using absolute project/script paths; do not stop merely because the bare command `rho` is absent. The Agent's current shell directory is never workspace identity. Only the project root and workspace ID reported by Rho are authoritative.

Treat lifecycle `disconnected` as “control plane online, Workspace R not attached,” not failed Agent registration. Never substitute a separate `R`, `Rscript`, Ark, or `rho-server` process. Never kill or replace a listener merely because a sandboxed probe cannot reach it.

Read [references/workbench-protocol.md](references/workbench-protocol.md) when tool selection or response fields are unclear.
