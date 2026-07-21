# Rho for Claude Code

Install `rho-mcp` on `PATH`, start the intended Rho runtime, then test this plugin locally:

```sh
cargo install --path crates/rho-mcp
claude --plugin-dir ./integrations/claude-code/rho
```

Set `RHO_SERVER_URL` when the runtime is not at `http://127.0.0.1:8787`. Inside Claude Code, use `/mcp` to confirm the `rho` server and `/rho:rho-runtime` to invoke the packaged workflow explicitly.

The repository-level `.mcp.json` and `.claude/skills/rho-runtime/SKILL.md` provide the equivalent project-scoped setup without installing the plugin.
