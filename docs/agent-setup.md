# Connect an Agent to Rho

This bounded, one-pass setup contract tells the Agent to install Rho's official runtime skill in the project named by the user's prompt, configure that Agent's MCP client, read each health layer once, and stop. Do not turn setup into a server or dependency repair loop.

## Inputs are authoritative

The copied sentence supplies:

- `PROJECT_ROOT`: the project to configure
- `RHO_SERVER_URL`: the already-running Rho control plane

Use those literal values even when the Agent started in another directory. Record the Agent's startup directory before changing directories. Running `cd PROJECT_ROOT` in a shell tool does not retroactively load project-scoped instructions, skills, plugins, or MCP servers into the current Agent process.

## Keep setup and runtime health separate

Report these independently:

1. **Project configuration** — Rho's MCP entry and official skill are installed or already current in `PROJECT_ROOT`.
2. **Current-session MCP** — active only after a successful Rho MCP tool call in this Agent process.
3. **Control plane** — ready when `RHO_SERVER_URL` responds to Rho's diagnostic request.
4. **Runtime dependencies** — ready when Rho has a compatible R, verified Ark, embedded bridge, and project-controlled binding. `action_required` means a human or system administrator must complete the reported action.
5. **Workspace R** — ready only when Rho says its execution backend is attached and available. Lifecycle `disconnected` is not Agent registration state and does not mean project setup failed.

The three runtime health layers are the control plane, runtime dependencies, and Workspace R. A healthy control plane can remain online while dependencies are preparing or awaiting a human action.

Never say only “connected.” Name the layer.

## Guardrails

- Work only inside `PROJECT_ROOT`; preserve unrelated instructions, skills, plugins, and MCP servers.
- Never pass `--force` or replace a conflicting Rho-owned entry without explicit approval.
- Do not start, restart, kill, background, or daemonize `rho-server` during Agent setup. Rho Desktop, a persistent terminal, or a registered service owns the runtime—not the configuring Agent or its sandbox.
- Do not start a separate `R`, `Rscript`, or Ark process and do not execute scientific R code during setup. Rho's Dependency Manager owns R discovery, Ark, the bridge, and the generated project binding.
- Do not run `rho deps ensure`, `rho deps repair`, `rho deps ensure --install-r`, a package manager, or `sudo` during this onboarding contract. Read dependency status only. If R is absent, report the action URL or system-provider instruction and leave installation to the human.
- Run each bootstrap and MCP check at most once. Run one normal diagnostic attempt. If—and only if—the tool explicitly identifies a sandbox or loopback-network permission boundary, request approval once and rerun that same diagnostic in the approved context. This is the only diagnostic retry. Do not infer that the listener is stale and do not replace it.

## One-pass setup

1. Confirm that `PROJECT_ROOT` exists. Detect the active Agent from the current process, not from files found inside the target:
   - Codex uses `AGENTS.md`, `.codex/config.toml`, and `.agents/skills/`.
   - Claude Code uses `CLAUDE.md`, `.mcp.json`, and `.claude/skills/`.
   - Another MCP client should use its native Streamable HTTP configuration with URL `RHO_SERVER_URL/mcp`.
2. Resolve one `RHO_CLI` command prefix and reuse it for the rest of setup:
   - Prefer the absolute executable returned by `command -v rho`. Rho Desktop and official packages supply this CLI; users should not need Rust.
   - When setup is running inside the Rho source checkout, use `cargo run --quiet --manifest-path ABSOLUTE_RHO_SOURCE/Cargo.toml -p rho-cli --`. The manifest path must be absolute so the command works from any shell directory.
   - If neither exists, report that the Rho installation is incomplete. Do not ask an ordinary user to install Rust, search `target/`, or guess at a bare command. You may still make one read-only `GET RHO_SERVER_URL/healthz` request to report runtime health, but project bootstrap is blocked until the packaged CLI is available.

3. Run exactly one project bootstrap command, substituting the literal URL and absolute path:

   ```sh
   RHO_CLI --server RHO_SERVER_URL init codex --project PROJECT_ROOT
   RHO_CLI --server RHO_SERVER_URL init claude --project PROJECT_ROOT
   ```

   `RHO_CLI` means the prefix resolved in step 2, not a literal executable name. Bootstrap is idempotent. `created`, `updated`, and `unchanged` are successful outcomes. It merges the Rho block into existing Agent instructions and adds `RHO_SERVER_URL/mcp` as a cwd-independent HTTP MCP entry without deleting unrelated configuration. Known legacy `cargo`, `rho-mcp`, and relative-`cwd` Rho entries are migrated in place. A `preserved` Rho-owned entry is a real content conflict: inspect it once, leave it untouched, and report the path.
4. Verify the configured URL with one structured request:

   ```sh
   RHO_CLI --server RHO_SERVER_URL doctor --json
   ```

   This result reports control-plane health, the dependency summary, Workspace R readiness, workspace ID, project root, lifecycle, and whether execution is available. If it is denied by an identified sandbox permission—or this document was just fetched from the same loopback URL but the sandboxed CLI cannot reach it—request approval once and repeat this exact command in the approved context. Never substitute a server launch. For any other failure, report the control plane as unreachable and stop runtime checks. Do not fall back to repeated `curl`, `lsof`, process inspection, or server launch attempts.
5. If the diagnostic reports dependencies other than `ready`, read the detailed dependency report exactly once without changing it:

   ```sh
   RHO_CLI deps status --project PROJECT_ROOT --json
   ```

   Report the component, issue code, and human action. Do not invoke an available action during Agent setup.
6. Check whether Rho MCP was already loaded into this Agent process:
   - If a Rho MCP tool is available, call `workspace_open` exactly once. Only a successful call makes **current-session MCP** active.
   - If no Rho MCP tool is available, do not retry discovery. The CLI diagnostic proves the runtime record, not MCP activation. MCP transport itself is independent of the shell cwd; `codex -C` is needed only so a new Agent process discovers the selected project's config and skills.
   - For Codex started outside `PROJECT_ROOT`, or when bootstrap created/updated project MCP files, give the exact new-session command with the supplied absolute path shell-quoted:

     ```sh
     codex -C PROJECT_ROOT
     ```

     Do not leave `PROJECT_ROOT` as a placeholder in the final answer. New project skills and plugin tools are expected to become available in the new session.

The project-scoped `operate-rho-runtime` skill installed by `rho init codex` is Rho's canonical skill. Rho's official Codex plugin packages the same skill and MCP adapter for marketplace distribution; installing the plugin is optional and is not a reason to modify global Codex state during this project-scoped setup.

## Required report

Return a compact result in this shape:

```text
Project configuration: configured | already current | blocked
Agent client: Codex | Claude Code | other
Project root: <absolute path>
Current-session MCP: active | not loaded — new session required | unavailable
Control plane: ready | unreachable
Runtime dependencies: ready | preparing | action required | failed — <issue/action when present>
Workspace ID: <workspace_id or unavailable>
Workspace R: <lifecycle> — <plain-language readiness meaning>
Execution: available | unavailable
Verified with: MCP workspace_open | rho doctor --json | unavailable
Next command: codex -C '<absolute project path>'   # only when required
```

After a project-scoped Agent session loads the integration, use `operate-rho-runtime` for scientific work. The Agent reasons and plans; Rho executes and proves.
