# Rho tool map

| Tool | Use |
| --- | --- |
| `workspace_open` | Discover durable workspace identity and live objects |
| `workspace_status` | Read lifecycle, project root, and revision |
| `workspace_execute` | Execute approved R code in Workspace R |
| `object_inspect` | Read bounded semantic object metadata and preview |
| `run_history` | Review recorded state transitions |
| `problem_list` | Review structured errors, warnings, calls, and tracebacks |
| `artifact_export` | Read an artifact payload and provenance metadata |
| `plot_view` | Read a plot payload and provenance metadata |

The stable public entities are Workspace, Run, Object, Artifact, Problem, Approval, and Provenance. Treat all identifiers as opaque.

Resolve `RHO_CLI` once as the absolute installed `rho` executable or the absolute source-checkout Cargo prefix defined in the parent skill. Read runtime health with `RHO_CLI --server URL doctor --json`, or one `GET URL/healthz` request when no CLI exists: `control_plane_ready` covers the API, `dependencies` covers R/Ark/bridge/binding preparation, and `workspace_r_ready` plus `lifecycle` covers the authoritative executor. Use `RHO_CLI deps status --project ABSOLUTE_PROJECT --json` for read-only dependency details. During onboarding, report any required human action without invoking it or installing R.
