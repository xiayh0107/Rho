---
name: operate-rho-runtime
description: Operate an Agent-native Rho scientific runtime for R workspaces. Use when inspecting or executing R code, live objects, plots, problems, run history, artifacts, or provenance in a Rho project; prefer the Rho MCP tools and fall back to the machine-readable rho CLI.
---

# Operate Rho Runtime

Treat Rho as the source of truth for scientific state. Use the Agent client for reasoning and planning; use Rho for execution, object semantics, artifacts, approvals, and evidence.

## Workflow

1. Discover the runtime with `workspace_open`. It returns the durable workspace and the currently known R objects. If MCP is unavailable, run `rho status --json`, then `rho objects --json`.
2. Inspect before changing state. Use `object_inspect`, `run_history`, `problem_list`, or `plot_view` to establish the current scientific state.
3. Explain the intended state change and execute only the requested R code through `workspace_execute`. Respect the client approval prompt. Do not bypass Rho with a separate `R`, `Rscript`, or Ark process when the authoritative runtime is available.
4. Verify the result with run history and problems. Inspect changed objects and plots when relevant.
5. Cite run, object, artifact, and provenance identifiers in the handoff so the result is reproducible.

## Runtime discovery

- Use `RHO_SERVER_URL` when it is set; otherwise use `http://127.0.0.1:8787`.
- A Rho project commonly contains `.Rproj`, `DESCRIPTION`, `renv.lock`, `.Rprofile`, an `R/` directory, or an active workspace whose `project_root` points at the repository.
- A disconnected runtime is not permission to start an unrelated R process. Report the structured error and start or reconnect `rho-server` with the intended project and kernelspec.

## Safety boundary

- Read-only inspection, summarization, and visualization are automatic.
- R execution, object mutation, file overwrite, package installation, shell execution, and data upload require the policy path appropriate to the active client.
- Keep credentials out of R code and artifacts.
- `artifact_export` returns an artifact payload; it does not authorize writing it to an arbitrary path.

For the tool-to-CLI mapping and response contract, read [references/workbench-protocol.md](references/workbench-protocol.md).
