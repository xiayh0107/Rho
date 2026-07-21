# ADR-010: Agent-first product boundary

## Status

Accepted on 2026-07-20.

## Context

General Agent clients already own conversation, planning, model selection, tool orchestration, and coding workflows. Rebuilding those capabilities inside Rho would make the product compete with its strongest integration surface and would leave scientific state coupled to one UI process.

Rho's durable differentiation is the part a general Agent cannot safely infer from files or chat: the authoritative live R session, domain-aware object semantics, execution policy, artifacts, and evidence linking a result back to code and environment.

## Decision

Rho will not compete with general Agent clients.

Codex, Claude Code, Copilot, and other Agents handle reasoning, conversation, planning, general code editing, and orchestration.

Rho handles scientific runtime lifecycle, authoritative R state, object semantics, policy-controlled execution, approvals, artifacts, problems, and provenance.

The product boundary is:

> Agent thinks. Rho executes and proves.

`rho-server` is the source of truth. Browser, CLI, MCP, and the desktop wrapper are replaceable projections over an independently versioned Workbench Protocol.

```text
Browser · CLI · MCP Agents · Desktop WebView
                    │
          Rho Workbench Protocol
                    │
               rho-server
                    │
              Workspace R
                    │
                   Ark
```

Scientific state must survive browser refresh, client restart, and Agent reconnection. UI preferences may remain client-local, but workspace identity, runs, objects, problems, approvals, artifacts, events, and provenance belong to the runtime.

## Consequences

- The browser focuses on scientific state visualization instead of full IDE parity.
- The desktop package contains startup, tray, notifications, file association, and an embedded browser; it does not own a second execution implementation.
- External Agents receive semantic MCP tools rather than Jupyter frames or internal RPC.
- All entry points share a policy vocabulary and stable public entities.
- Remote runtimes can be projected locally without copying authoritative state into the gateway.
- New generic chat, editor, or planning features require evidence that they cannot live in the Agent client before entering Rho core.

## Non-goals

- Replacing Claude Code, Codex, Copilot, or other Agent clients.
- Treating browser state as a workspace database.
- Exposing Ark/Jupyter transport as Rho's public protocol.
- Launching a second authoritative R session for a different client.
