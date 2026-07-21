# Rho Workbench Protocol quick reference

The public protocol version is independent from the Rho application version. Reject a mismatched `protocol_version` rather than guessing at fields.

| Scientific task | MCP tool | CLI fallback |
| --- | --- | --- |
| Discover workspace and objects | `workspace_open` | `rho status --json`; `rho objects --json` |
| Read lifecycle | `workspace_status` | `rho status --json` |
| Execute R | `workspace_execute` | `rho run script.R --json` |
| Inspect an object | `object_inspect` | `rho inspect OBJECT --json` |
| Review runs | `run_history` | Workbench HTTP API |
| Review problems | `problem_list` | `rho problems --json` |
| Read an artifact | `artifact_export` | Workbench HTTP API |
| Read a plot | `plot_view` | `rho plots list --json` |

Stable public entities are `Workspace`, `Run`, `Object`, `Artifact`, `Problem`, `Approval`, and `Provenance`. IDs are opaque strings. Object previews and metadata are deliberately bounded.

All CLI JSON responses have this envelope:

```json
{
  "ok": true,
  "protocol_version": "0.1",
  "data": {}
}
```

Errors use `ok: false`, the same protocol version, and a human-readable `error`. HTTP responses expose structured error codes and retryability. A WebSocket event stream provides replayable runtime events; clients should reconnect using the last observed sequence rather than treating browser state as authoritative.
