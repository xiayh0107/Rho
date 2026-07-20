# Agent-First Rho Implementation TODO

> GitHub Issues are currently disabled in this repository. This checklist mirrors the planned issue backlog and can be converted into Issues after enabling them.

## Epic 1 — Runtime-first architecture

- [ ] Define rho-server as the source of truth
- [ ] Extract protocol layer from UI implementation
- [ ] Define workspace identity model
- [ ] Define revision and event model
- [ ] Document runtime API contracts

## Epic 2 — CLI and machine interface

- [ ] Build rho CLI command structure
- [ ] Add JSON output mode for Agent consumption
- [ ] Support workspace status inspection
- [ ] Support execution and run retrieval
- [ ] Support object inspection commands

## Epic 3 — MCP integration

- [ ] Implement rho-mcp server
- [ ] Define minimal MCP tools
- [ ] Add Claude Code integration guide
- [ ] Add Codex integration guide
- [ ] Add generic Agent integration examples

## Epic 4 — Browser Web App

- [ ] Convert desktop UI logic into browser-compatible frontend
- [ ] Create WebSocket/API communication layer
- [ ] Build Run viewer
- [ ] Build Object viewer
- [ ] Build Plot viewer
- [ ] Build Problems viewer
- [ ] Build Approval viewer
- [ ] Build Provenance timeline

## Epic 5 — Scientific semantics

- [ ] R object inspection framework
- [ ] Bioconductor object support
- [ ] Seurat support
- [ ] ggplot provenance
- [ ] Artifact lineage tracking

## Epic 6 — Safety and governance

- [ ] Unified policy engine
- [ ] Approval rules shared by Web/MCP/CLI
- [ ] Local authentication token
- [ ] Origin validation
- [ ] Project filesystem isolation

## Epic 7 — Remote execution

- [ ] Remote rho-server support
- [ ] HPC integration design
- [ ] Long-running job management
- [ ] Shared workspace support

## Epic 8 — Optional Desktop wrapper

- [ ] Keep desktop as thin WebView wrapper
- [ ] Avoid desktop-only business logic
- [ ] Package installers

## Priority order

1. Runtime protocol
2. MCP + CLI
3. Browser scientific workspace
4. Agent integrations
5. Remote execution
6. Desktop packaging
