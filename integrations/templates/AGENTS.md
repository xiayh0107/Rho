# Rho scientific runtime contract

This project uses Rho as the source of truth for R state.

- The Agent reasons, converses, and plans. Rho executes and proves.
- Prefer the Rho MCP tools. Fall back to `rho ... --json` when MCP is unavailable.
- Inspect workspace state before changing it.
- Execute scientific R code in the authoritative Workspace R; do not create an unrelated R session.
- Respect Rho policy decisions and client approval prompts.
- Never hide package installation, file overwrite, shell execution, or data upload inside a seemingly read-only step.
- After execution, check runs and problems, then inspect changed objects, plots, artifacts, and provenance.
- Report stable identifiers so another client can reconnect and verify the result.

Common R project markers are `.Rproj`, `DESCRIPTION`, `renv.lock`, `.Rprofile`, and `R/`. Their presence helps discovery but does not replace the workspace identity returned by Rho.
