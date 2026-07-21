# Rho Workbench Protocol quick reference

The public protocol version is independent from the Rho application version. Reject a mismatched `protocol_version` rather than guessing at fields.

`RHO_CLI` below means the absolute installed `rho` executable, or the one-time absolute source-checkout prefix defined in the parent skill. Never assume it is a bare command on `PATH`.

| Scientific task | MCP tool | CLI fallback |
| --- | --- | --- |
| Discover workspace and objects | `workspace_open` | `RHO_CLI --server URL doctor --json`; `RHO_CLI --server URL objects --json` |
| Read lifecycle | `workspace_status` | `RHO_CLI --server URL doctor --json` |
| Execute R | `workspace_execute` | `RHO_CLI --server URL run - --json` with the approved R code on stdin |
| Inspect an object | `object_inspect` | `RHO_CLI --server URL inspect OBJECT --json` |
| Review runs | `run_history` | Workbench HTTP API |
| Review problems | `problem_list` | `RHO_CLI --server URL problems --json` |
| Read an artifact | `artifact_export` | Workbench HTTP API |
| Read a plot | `plot_view` | `RHO_CLI --server URL plots list --json` |

Stable public entities are `Workspace`, `Run`, `Object`, `Artifact`, `Problem`, `Approval`, and `Provenance`. IDs are opaque strings. Object previews and metadata are deliberately bounded.

CLI JSON responses use an `ok`, `protocol_version`, and `data` envelope. Errors use `ok: false`, the same protocol version, and a human-readable `error`. A replayable WebSocket event stream is the live update channel; browser state is never authoritative.

## Runtime health and dependencies

Read the three health layers with `RHO_CLI --server URL doctor --json` (or one `GET URL/healthz` request when the CLI is unavailable):

| Layer | Field | Meaning |
| --- | --- | --- |
| Control plane | `control_plane_ready` | The Workbench HTTP API is reachable |
| Dependencies | `dependencies` | R, Ark, the bridge, and project binding are ready or have a structured issue/action |
| Workspace R | `workspace_r_ready`, `executor_attached`, `lifecycle` | The authoritative executor is attached and available |

Use `rho deps status --project PROJECT --json` for a read-only component report. Use `rho deps ensure` only for an explicitly requested preparation, `rho deps repair` to replace an invalid managed Ark entry, and `rho deps cache-path` to locate the immutable cross-project cache. The HTTP equivalent is `GET /v1/runtime/dependencies`; dependency actions use `POST /v1/runtime/dependencies` and require confirmation where reported.

Lifecycle `disconnected` only describes Workspace R. During Agent setup, report dependency status and its human action without invoking it, installing R, escalating privileges, or launching a substitute runtime.
