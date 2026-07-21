# Rho for Claude Code

Start the intended Rho runtime in managed mode, then test this plugin locally. Its MCP entry connects directly to the runtime's Streamable HTTP endpoint; no local `rho-mcp` executable or working-directory convention is required:

```sh
rho-server serve --project-root /path/to/project
claude --plugin-dir ./integrations/claude-code/rho
```

Managed mode discovers the installed R, prepares verified Ark/bridge components and the project binding, and starts Workspace R by default. Set `RHO_SERVER_URL` when the runtime is not at `http://127.0.0.1:8787`. Inside Claude Code, use `/mcp` to confirm the `rho` server and `/rho:rho-runtime` to invoke the packaged workflow explicitly.

The repository-level `.mcp.json` and `.claude/skills/rho-runtime/SKILL.md` provide the equivalent project-scoped setup without installing the plugin.
