# Agent-First Rho implementation inventory

This file records the architecture backlog implemented for the `Agent-First Rho Architecture v0.1` milestone. GitHub Issues are the tracking source; this inventory is kept for repository readers.

## Runtime and protocol

- [x] Make `rho-server` the source of truth.
- [x] Persist workspace identity independently of clients.
- [x] Preserve replayable runtime events and sessions across reconnects.
- [x] Define independently versioned HTTP, WebSocket, JSON schema, artifact, and deep-link contracts.
- [x] Define Workspace, Run, Object, Artifact, Problem, Approval, and Provenance.

## Agent interoperability

- [x] Add the machine-readable `rho` CLI.
- [x] Add the semantic `rho-mcp` server.
- [x] Add Claude Code project configuration and plugin packaging.
- [x] Add a repository-scoped Codex skill and MCP configuration.
- [x] Add R project detection, project bootstrap, AGENTS template, and example workflow.

## Scientific control plane

- [x] Add browser Runs, Objects, Plots, Problems, Approvals, and Provenance pages.
- [x] Add bounded semantic inspection for core R and common scientific object classes.
- [x] Link code, parameters, environment, revisions, objects, artifacts, and execution time in provenance.
- [x] Share policy decisions across browser/API, CLI, MCP, and internal Agent actions.

## Projections

- [x] Add a remote HTTP/WebSocket gateway that leaves state upstream.
- [x] Reduce desktop to `rho-server` plus an embedded browser, tray, notifications, startup, and file association.
- [x] Rewrite product positioning around the Agent-first boundary.

## Follow-on hardening

- [ ] Add authenticated non-loopback deployment profiles.
- [ ] Add concrete SSH and scheduler lifecycle adapters behind the remote gateway.
- [ ] Add protocol compatibility fixtures and multi-platform release matrices.
