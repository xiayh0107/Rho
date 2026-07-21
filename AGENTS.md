<!-- BEGIN RHO SCIENTIFIC RUNTIME CONTRACT -->
# Rho scientific runtime contract

This project uses Rho as the source of truth for R state.

- The Agent reasons, converses, and plans. Rho executes and proves.
- Prefer the runtime's cwd-independent HTTP MCP tools. For diagnostics when MCP is unavailable, resolve the absolute packaged `rho` CLI—or the absolute-manifest Cargo prefix in a Rho source checkout—once; never assume a bare `rho` exists on `PATH`.
- Inspect workspace state before changing it.
- Execute scientific R code in the authoritative Workspace R; do not create an unrelated R session.
- Respect Rho policy decisions and client approval prompts.
- Never hide package installation, file overwrite, shell execution, or data upload inside a seemingly read-only step.
- After execution, check runs and problems, then inspect changed objects, plots, artifacts, and provenance.
- Report stable identifiers so another client can reconnect and verify the result.
- Treat `disconnected` as “control plane online, Workspace R not attached,” not failed Agent registration.
- During setup, do not start, restart, kill, or background `rho-server`; the Rho app or another durable owner manages it.
- During setup, read and report Rho's dependency status; do not install R, run `sudo`, invoke dependency mutations, or launch substitute R/Ark processes.

Common R project markers are `.Rproj`, `DESCRIPTION`, `renv.lock`, `.Rprofile`, and `R/`. Their presence helps discovery but does not replace the workspace identity returned by Rho.

The Agent shell's current directory is not Rho workspace identity. Use absolute project/script paths and the project root plus workspace ID reported by Rho.
<!-- END RHO SCIENTIFIC RUNTIME CONTRACT -->
