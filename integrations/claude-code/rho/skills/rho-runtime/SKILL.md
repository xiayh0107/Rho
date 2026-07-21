---
description: Operate a Rho scientific runtime from Claude Code. Use when the user asks to inspect or execute R code, objects, runs, problems, plots, artifacts, or provenance in a Rho-backed project.
---

# Operate Rho

Treat Rho as the source of truth for scientific state.

1. Call `workspace_open` to discover the workspace and live objects.
2. Inspect the relevant state before changing it.
3. Explain the intended effect and call `workspace_execute` only for requested R code. Honor the client approval prompt.
4. Verify the run with `run_history` and `problem_list`; inspect changed objects and plots.
5. Include stable run, object, artifact, and provenance identifiers in the answer.

Do not bypass an available Rho runtime by launching a separate `R`, `Rscript`, or Ark process. If the runtime is disconnected, report the error and reconnect the intended runtime. Read [references/workbench-protocol.md](references/workbench-protocol.md) when tool selection or response fields are unclear.
