# Rho development roadmap

Date: 2026-07-20

Current architecture milestone: Agent-First Rho Architecture v0.1

## Direction

Rho is an Agent-native scientific runtime for R, not a desktop-first AI IDE. General Agents own reasoning and planning. Rho owns the persistent workspace, semantic R inspection, safe execution, artifacts, and proof.

The dependency order is intentionally strict:

1. runtime identity and lifecycle;
2. public protocol and events;
3. CLI and MCP interoperability;
4. scientific browser projections;
5. remote and packaging projections.

Generic UI features do not move ahead of protocol stability.

## Implemented foundation

- `rho-server` owns durable workspace identity, lifecycle, replayable events, and execution attachment.
- Workbench Protocol 0.1 defines Workspace, Run, Object, Artifact, Problem, Approval, Provenance, schemas, and deep links.
- `rho` provides stable machine-readable status, execution, object, problem, and plot commands.
- `rho-mcp` provides eight semantic tools with MCP lifecycle and tool-result contracts.
- The browser control plane renders Runs, Objects, Plots, Problems, Approvals, and Provenance.
- Semantic object inspection covers data.frame, Seurat, SummarizedExperiment, SingleCellExperiment, GRanges, and ggplot.
- Policy decisions are shared across direct users, MCP hosts, and internal Agent actions.
- A remote gateway projects upstream HTTP and WebSocket state without taking ownership of it.
- Codex and Claude Code project integrations, skills, bootstrap, and example workflows are included.
- The desktop target is a tray/notification/file-association WebView wrapper over `rho-server`.

## Next milestones

### Protocol hardening

- Compatibility fixtures for each public entity and event.
- Authentication and deployment profiles for non-loopback servers.
- Explicit protocol deprecation and migration policy.
- Reconnect, replay, duplicate-execution, and crash-recovery stress tests.

### Remote compute adapters

- SSH-managed Linux runtime lifecycle.
- Scheduler adapters for Slurm and other HPC environments.
- Long-running job status, cancellation, and resource reporting.
- Authenticated cloud gateway deployment and workspace discovery.

All adapters must preserve the same workspace, approval, run, and provenance semantics as local execution.

### Scientific depth

- More Bioconductor and spatial/single-cell semantic adapters.
- Environment capture for renv, Bioconductor, system libraries, and containers.
- Reproducible report export from selected provenance subgraphs.
- Larger artifact stores with bounded previews and content-addressed transfer.

### Release engineering

- Cross-platform `rho`, `rho-mcp`, `rho-server`, and desktop packages.
- Signed installers and reproducible dependency/license manifests.
- Windows, macOS, and Linux end-to-end client matrices.
- Stable upgrade and rollback procedures for persisted workspace state.

## Decision checkpoint

Every milestone must answer:

- Does the runtime remain the only source of scientific truth?
- Can a browser or Agent disconnect without destroying workspace state?
- Do all clients observe the same identifiers, policy decisions, and provenance?
- Is a new surface scientific state visualization, or accidental reinvention of an Agent/IDE feature?
- Is the behavior covered by protocol-level tests rather than only one client UI?
