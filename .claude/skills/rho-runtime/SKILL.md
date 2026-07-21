---
description: Operate the authoritative Rho scientific runtime. Use for R execution, live object inspection, plots, problems, run history, artifacts, and reproducible scientific workflows in this project.
---

# Rho scientific runtime

Use Rho for scientific state and use Claude Code for reasoning.

1. Call `workspace_open` to discover the durable workspace and live objects.
2. Inspect relevant objects, runs, problems, and plots before changing state.
3. Explain the intended state change, then call `workspace_execute` only for requested R code and honor the approval prompt.
4. Verify the resulting run, problems, changed objects, plots, and provenance identifiers.

Do not start a separate `R`, `Rscript`, or Ark process as a substitute for a disconnected Rho runtime. Use `RHO_SERVER_URL` when configured; otherwise expect `http://127.0.0.1:8787`.
