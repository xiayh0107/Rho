# Agent workflows with Rho

Rho does not replace a general Agent client. Claude Code, Codex, Copilot, or another MCP client handles reasoning and planning; Rho owns the persistent R workspace, safe execution, scientific semantics, and provenance.

## One-prompt onboarding

Choose **Connect Agent** in the browser control plane and paste the generated sentence into the Agent. The sentence identifies the current project root, runtime URL, and the runtime-served setup contract at `/agent-setup.md`.

The contract tells the Agent to:

1. detect its own client integration format and startup directory;
2. install the official Rho skill and merge MCP configuration idempotently;
3. read the control plane, dependency summary, and Workspace R once with `rho doctor --json`;
4. load a new project-scoped Agent session when required; and
5. prove current-session MCP activation with one `workspace_open` call.

The runtime and remote gateway also publish the canonical skill files under `/agent/skills/operate-rho-runtime/`. These assets are compiled into the same Rho build that serves the protocol.

## Start the runtime

From the Rho source tree, start the durable server for the project:

```sh
cargo run -p rho-server -- serve --project-root /path/to/project
```

Managed dependency preparation is the default. Rho reuses a compatible local R, verifies its pinned Ark artifact in the global cache, materializes the embedded bridge, generates a controlled binding inside the project, and attaches Workspace R. The browser control plane is available at `http://127.0.0.1:8787` while this work is in progress. A client reconnect does not terminate Workspace R.

Use `--control-plane-only` explicitly when no execution backend is intended. An administrator may still pass `--kernelspec` as an advanced override.

Inspect or prepare dependencies independently of the server:

```sh
rho deps status --project /path/to/project --json
rho deps ensure --project /path/to/project --json
rho deps repair --project /path/to/project --json
rho deps cache-path --project /path/to/project --json
```

Rho discovers and reuses R; it does not silently elevate privileges. When R is missing, Rho reports an explicit installer or system-provider action. Agent onboarding reads and reports that state only—it must not install R, run `sudo`, launch a substitute R/Ark process, or start another server.

## Runtime health

Keep these layers distinct:

1. `control_plane_ready`: the Workbench API is reachable.
2. `dependencies`: R, Ark, the bridge, and project binding are ready or have a structured issue/action.
3. `workspace_r_ready`: the authoritative Workspace R executor is attached and available.

Use `rho --server URL doctor --json` for this combined view. A `disconnected` lifecycle may be expected during dependency preparation or `--control-plane-only`; it is unrelated to Agent registration.

## Codex

This repository checks in `.codex/config.toml` and `.agents/skills/operate-rho-runtime`. Trust the repository, restart Codex after changing MCP configuration, and invoke `$operate-rho-runtime` when you want the workflow explicitly.

For another R project, the setup Agent can run `rho --server URL init codex --project /path/to/project`. The command merges the Rho contract and MCP table, then writes the full official skill with its protocol reference. Existing unrelated instructions and MCP servers remain intact. The generated MCP entry points directly to `URL/mcp` over Streamable HTTP, so startup does not depend on the parent shell directory, Cargo, or `rho-mcp` on `PATH`.

Codex discovers project configuration by walking upward from its startup directory. If setup began elsewhere, open the configured project in a new session. This selects which project config Codex loads; it does not change or implicitly retarget Rho's authoritative workspace:

```bash
codex -C /path/to/project
```

For plugin distribution, `integrations/codex` is the `rho-official` marketplace root and `integrations/codex/plugins/rho` contains the official skill plus MCP adapter.

## Claude Code

The repository-level `.mcp.json` and `.claude/skills/rho-runtime` are project-scoped. Claude Code asks for approval before enabling a checked-in MCP server. The checked-in entry uses the runtime's HTTP MCP endpoint, so the plugin can be tested without installing a local adapter:

```sh
claude --plugin-dir ./integrations/claude-code/rho
```

Use `/mcp` to inspect the connection and `/rho:rho-runtime` to invoke the plugin workflow explicitly. Run `rho --server URL init claude --project /absolute/project` to generate a non-default endpoint.

## Example scientific turn

Request:

> Inspect `pbmc`, summarize its dimensions and metadata, run the requested QC transformation, show new problems and plots, and give me the provenance IDs.

Expected client flow:

1. `workspace_open` discovers the workspace and `pbmc`.
2. `object_inspect` reads bounded Seurat semantics before mutation.
3. The Agent explains the exact R code and `workspace_execute` goes through client approval.
4. `run_history` and `problem_list` verify execution.
5. `object_inspect` and `plot_view` verify the new state and artifact.
6. The answer includes stable workspace, run, object, artifact, and provenance identifiers.

This same flow works over the CLI with `rho status --json`, `rho objects --json`, `rho inspect pbmc --json`, `rho run qc.R --json`, `rho problems --json`, and `rho plots list --json`.
