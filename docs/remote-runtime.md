# Remote Rho runtimes

The remote boundary preserves one rule: scientific state stays with the upstream `rho-server`. A browser, CLI, MCP host, or local gateway may disconnect and reconnect, but it does not copy or become authoritative for Workspace R.

## Local machine

```sh
rho-server serve \
  --host 127.0.0.1 \
  --port 8787 \
  --project-root /path/to/project \
  --kernelspec /path/to/kernel.json
```

Connect clients directly to `http://127.0.0.1:8787`.

## Linux server over SSH

Run the server on loopback at the remote host and forward the port through SSH:

```sh
ssh server.example.org \
  'cd /srv/project && rho-server serve --host 127.0.0.1 --port 8787 --project-root . --kernelspec /srv/rho/kernel.json'

ssh -N -L 8787:127.0.0.1:8787 server.example.org
```

The local browser and Agent clients can use the forwarded endpoint directly, or a local gateway can project it onto a different port.

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
  --kernelspec /shared/rho/kernel.json
```

Forward the allocated node's port through the login host according to site policy. Scheduler submission, authentication, quotas, and cancellation remain deployment responsibilities in protocol 0.1; they must not be disguised as scientific R execution.

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

## Reconnection contract

- Persist the SQLite store and project files on the remote host.
- Reconnect WebSocket clients with the last observed event sequence.
- Do not retry a mutation merely because the client lost its response; inspect run history first.
- Treat remote workspace and artifact IDs as opaque.
- Shut down Workspace R through the runtime lifecycle rather than deleting the state store.
