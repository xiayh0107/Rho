---
description: Connect to and operate a Rho scientific runtime from Claude Code. Use for Rho project setup, connection checks, or requests to inspect and execute R code, objects, runs, problems, plots, artifacts, approvals, and provenance.
---

# Operate Rho

Treat Rho as the source of truth for scientific state.

1. Use the runtime URL supplied by the user, then `RHO_SERVER_URL`, then `http://127.0.0.1:8787`.
2. When the user asks to configure Rho, read `RHO_SERVER_URL/agent-setup.md`, preserve existing configuration, and never bootstrap with `--force` without explicit approval.
3. Call `workspace_open` once to discover the workspace and live objects. The primary transport is `URL/mcp` over Streamable HTTP, independent of shell cwd and local executables. If MCP is unavailable, resolve one CLI prefix: the absolute installed `rho`, or—only in the Rho source checkout—`cargo run --quiet --manifest-path ABSOLUTE_RHO_SOURCE/Cargo.toml -p rho-cli --`. Use it for `--server URL doctor --json`; if neither exists, make one read-only `GET URL/healthz` check. Never assume a bare `rho`, retry in a loop, or start a server.
4. Inspect the relevant state before changing it.
5. Explain the intended effect and call `workspace_execute` only for requested R code. Honor the client approval prompt.
6. Verify the run with `run_history` and `problem_list`; inspect changed objects and plots.
7. Include stable run, object, artifact, and provenance identifiers in the answer.

If MCP is unavailable but the resolved CLI reports Workspace R ready, continue through the JSON CLI with absolute project/script paths. Shell cwd is not workspace identity; use Rho's reported project root and workspace ID.

Let Rho's Dependency Manager own R discovery, the checksum-pinned Ark cache, the embedded bridge, and the project binding. During onboarding, use only `rho deps status --project PROJECT --json` when dependency details are needed; report its human action without running `ensure`, `repair`, `--install-r`, a package manager, or `sudo`. Outside onboarding, use dependency mutations only after an explicit user request and required approval.

Treat lifecycle `disconnected` as a healthy control plane without an attached Workspace R, not failed Agent registration. Do not launch a separate `R`, `Rscript`, Ark, or `rho-server` process as a substitute, and never replace a listener merely because a sandboxed probe cannot reach it. Read [references/workbench-protocol.md](references/workbench-protocol.md) when tool selection or response fields are unclear.
