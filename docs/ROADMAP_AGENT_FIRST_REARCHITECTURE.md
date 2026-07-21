# Rho Agent-First Rearchitecture Plan

Status: architecture foundation implemented on 2026-07-20. See ADR-010 and
`docs/development-roadmap.md` for the active boundary and follow-on work.

## Vision

Rho should evolve from a standalone AI R IDE into a scientific runtime layer that can be used by any Agent (Codex, Claude Code, Copilot, future agents) while providing an optional rich browser-based scientific workspace.

The core thesis:

> Agents should own reasoning and task orchestration. Rho should own scientific execution state, R semantics, provenance, safety, and reproducibility.

## Current Risk

A standalone GUI client creates adoption friction:

- users already have preferred Agents;
- generic IDE/chat experiences are becoming commodity;
- rebuilding an Agent client duplicates ecosystem work.

The long-term differentiation is not another chat interface, but a trusted scientific runtime.

## Target Architecture

```
Codex / Claude Code / Copilot / Rho Web
              |
       MCP / CLI / HTTP API
              |
        Rho Runtime Server
              |
  policy | revision | provenance | events
              |
          R Workspace
              |
          R / Ark process
```

## Product Layers

### Rho Runtime

The core product:

- workspace lifecycle
- persistent R sessions
- revision tracking
- approval policy
- execution audit
- object inspection
- artifact management
- provenance graph

### Rho MCP

Expose scientific capabilities to external Agents:

- workspace_open
- workspace_status
- workspace_execute
- object_inspect
- run_history
- problem_list
- artifact_export
- plot_view

### Rho CLI

A universal fallback interface for humans and Agents.

### Rho Web

Browser-based scientific control plane.

The browser should provide:

- Run history
- Object viewers
- Plot viewers
- Problems
- Approval workflow
- Provenance timeline

The browser is a client, not the source of truth.

### Rho Desktop

Optional packaging layer:

```
Rho Desktop = Rho Runtime + WebView launcher
```

Desktop should not contain unique business logic.

## Migration Strategy

### Phase 1: Runtime-first foundation

- define stable protocol
- separate server from UI
- expose machine-readable state

### Phase 2: Agent ecosystem

- MCP server
- Claude Code integration
- Codex integration
- generic agent documentation

### Phase 3: Scientific UX

Build high-value scientific interfaces:

- object explorer
- provenance viewer
- execution timeline
- artifact browser

Avoid rebuilding generic IDE features.

### Phase 4: Remote and enterprise

Support:

- remote workspaces
- HPC execution
- shared projects
- long-running jobs

## Success Criteria

Rho succeeds when users can say:

"I use Claude Code/Codex for my work, but Rho gives my Agent a real scientific workspace that understands R objects and preserves every decision."
