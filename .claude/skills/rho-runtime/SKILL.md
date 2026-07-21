---
description: Connect to and operate the authoritative Rho scientific runtime. Use for Rho project setup, connection checks, R execution, live object inspection, plots, problems, run history, artifacts, and reproducible scientific workflows.
---

# Rho scientific runtime

Use Rho for scientific state and use Claude Code for reasoning.

1. Use the runtime URL supplied by the user, then `RHO_SERVER_URL`, then `http://127.0.0.1:8787`.
2. When the user asks to configure Rho, read `RHO_SERVER_URL/agent-setup.md`, preserve existing configuration, and never bootstrap with `--force` without explicit approval.
3. Call `workspace_open` once to discover the durable workspace and live objects. The primary transport is `URL/mcp` over Streamable HTTP, independent of shell cwd and local executables. If MCP is unavailable, resolve one CLI prefix: the absolute installed `rho`, or—only in the Rho source checkout—`cargo run --quiet --manifest-path ABSOLUTE_RHO_SOURCE/Cargo.toml -p rho-cli --`. Use it for `--server URL doctor --json`; if neither exists, make one read-only `GET URL/healthz` check. Never assume a bare `rho`, retry in a loop, or start a server.
4. Inspect relevant objects, runs, problems, and plots before changing state.
5. Explain the intended state change, then call `workspace_execute` only for requested R code and honor the approval prompt.
6. Verify the resulting run, problems, changed objects, plots, and provenance identifiers.

If MCP is unavailable but the resolved CLI reports Workspace R ready, continue through the JSON CLI with absolute project/script paths. Shell cwd is not workspace identity; use Rho's reported project root and workspace ID.

Let Rho's Dependency Manager own R discovery, the checksum-pinned Ark cache, the embedded bridge, and the project binding. During onboarding, use only `rho deps status --project PROJECT --json` when dependency details are needed; report its human action without running `ensure`, `repair`, `--install-r`, a package manager, or `sudo`. Outside onboarding, use dependency mutations only after an explicit user request and required approval.

Treat lifecycle `disconnected` as a healthy control plane without an attached Workspace R, not failed Agent registration. Do not start a separate `R`, `Rscript`, Ark, or `rho-server` process as a substitute, and never replace a listener merely because a sandboxed probe cannot reach it.
