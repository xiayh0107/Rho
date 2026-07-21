# Remote Rho runtimes

The remote boundary preserves one rule: scientific state stays with the upstream `rho-server`. A browser, CLI, MCP host, or local gateway may disconnect and reconnect, but it does not copy or become authoritative for Workspace R.

## Local machine

```sh
rho-server serve \
  --host 127.0.0.1 \
  --port 8787 \
  --project-root /path/to/project
```

Connect clients directly to `http://127.0.0.1:8787`. Managed mode discovers a compatible installed R, prepares verified Rho-owned components, generates the project binding, and starts Workspace R by default. Use `rho deps status --project /path/to/project --json` to inspect readiness without a server.

## Linux server over SSH

Run the server on loopback at the remote host and forward the port through SSH:

```sh
ssh server.example.org \
  'cd /srv/project && rho-server serve --host 127.0.0.1 --port 8787 --project-root .'

ssh -N -L 8787:127.0.0.1:8787 server.example.org
```

The local browser and Agent clients can use the forwarded endpoint directly, or a local gateway can project it onto a different port. Install a supported R through the server's operating-system provider before startup, or set `RHO_RSCRIPT` to the intended installation. Rho can prepare its checksum-pinned Ark component without privilege escalation; it never runs `sudo` or installs system R silently.

## HPC job

Submit `rho-server` and Workspace R together on the compute node. The scheduler owns process placement; Rho owns workspace identity and scientific lifecycle.

```sh
#!/bin/sh
#SBATCH --job-name=rho-runtime
#SBATCH --time=08:00:00
#SBATCH --cpus-per-task=4
#SBATCH --mem=32G

cd "$SLURM_SUBMIT_DIR"
exec rho-server serve \
  --host 127.0.0.1 \
  --port "${RHO_PORT:-8787}" \
  --project-root "$SLURM_SUBMIT_DIR" \
  --store "$SLURM_SUBMIT_DIR/.rho/state/runtime.sqlite" \
  --offline
```

Prepare the dependency cache and project binding before submitting an offline job:

```sh
rho deps ensure --project /path/to/project --json
rho deps cache-path --project /path/to/project --json
```

Make that reported cache location visible on the compute node and provide the site's supported R through `PATH` or `RHO_RSCRIPT`. Forward the allocated node's port through the login host according to site policy. Scheduler submission, authentication, quotas, and cancellation remain deployment responsibilities in protocol 0.1; they must not be disguised as scientific R execution.

## Dependency ownership and overrides

Ark is version-pinned and checksum-verified in Rho's immutable cross-project cache. The embedded `rho.bridge` ships with the Rho build, while the generated Ark/R binding is project-controlled under `.rho/runtime/`. `rho deps repair` re-verifies and replaces an invalid managed Ark entry; it does not modify system R.

Use `rho-server serve --control-plane-only` only when no Workspace R is intended. Use `--kernelspec /path/to/kernel.json` as an advanced override for a centrally administered binding. Normal local, SSH, and scheduler profiles do not require a hand-written kernelspec.

## Cloud deployment

Run `rho-server` behind an authenticated TLS reverse proxy or private network. Protocol 0.1 does not provide public-internet authentication, so do not bind an unauthenticated server to a public interface.

From a trusted client machine, project the authenticated upstream endpoint:

```sh
rho-server gateway \
  --host 127.0.0.1 \
  --port 8787 \
  --upstream https://rho.example.org/workspaces/team-a
```

The gateway serves the browser locally and forwards HTTP plus `ws`/`wss` event traffic. It preserves upstream response status, structured API errors, protocol versions, workspace IDs, and event sequence numbers.

Runtime health has three independent layers: `control_plane_ready`, the `dependencies` summary, and `workspace_r_ready`. Diagnose the upstream with `rho --server URL doctor --json`; Agent setup must not start a local substitute runtime when an upstream layer is not ready.

## Reconnection contract

- Persist the SQLite store and project files on the remote host.
- Reconnect WebSocket clients with the last observed event sequence.
- Do not retry a mutation merely because the client lost its response; inspect run history first.
- Treat remote workspace and artifact IDs as opaque.
- Shut down Workspace R through the runtime lifecycle rather than deleting the state store.
