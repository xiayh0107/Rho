# Rho NEWS

This file records user-visible changes by release. It is intentionally
separate from the architecture plan: the plan describes intended work, while
this file records behavior that is already available in a released build.

## Unreleased

### Changed

- Repositioned Rho as an Agent-native scientific runtime for R: Agent clients
  reason and plan while Rho executes and proves.
- Made `rho-server` the durable source of workspace truth and reduced desktop
  to a browser wrapper with tray, notifications, startup, and `.Rproj`
  association.

### Added

- Independently versioned Workbench Protocol entities, HTTP API, JSON schemas,
  replayable WebSocket events, artifact references, and deep links.
- Machine-readable `rho` CLI and semantic `rho-mcp` server.
- Browser pages for Runs, Objects, Plots, Problems, Approvals, and Provenance.
- Bounded semantic inspection for data frames, Seurat,
  SummarizedExperiment, SingleCellExperiment, GRanges, and ggplot objects.
- Unified action policy, provenance graph, and remote HTTP/WebSocket gateway.
- Project-scoped Codex and Claude Code integrations, a Claude plugin, Agent
  skills, R project detection, and `rho init` bootstrap commands.
- One-prompt Agent onboarding in the browser, with a runtime-served setup
  contract and version-matched official `operate-rho-runtime` skill assets.
- Idempotent Agent bootstrap with non-destructive config merging, a packaged
  Codex plugin, one-call `rho doctor` diagnostics, and separate control-plane
  versus Workspace R readiness signals.
- A cwd-independent Streamable HTTP MCP endpoint built into `rho-server`, with
  automatic migration from legacy Cargo/stdio Agent configs and stdin-capable
  CLI execution fallback.
- Rho Dependency Manager with compatible-R discovery, pinned and verified Ark
  artifacts, embedded `rho.bridge`, project-controlled bindings, `rho deps`
  diagnostics/repair, and managed Workspace R startup by default.

## 0.2.0-dev.2 - 2026-07-16

### Added

- Native project selection with per-project restoration of open and closed
  document drafts, cursor positions and panel sizes.
- Monaco-based R editing with selection, current-line and complete-file
  execution in the authoritative Workspace R.
- Durable Runs, Problems, retry links, cancellation state and restart recovery.
- Broker-owned Agent turn history and explicit Act approval controls showing
  the exact tool, code, request id and workspace revision.
- Project environment diagnostics for R, library paths, `renv`, Bioconductor
  and attached packages.
- Bounded object previews, durable plot history with provenance, and optional
  Quarto/R Markdown render diagnostics.

### Fixed

- Agent mutations now require a single-use broker approval bound to the exact
  request arguments; Ask and Plan cannot bypass the mutation policy.
- Cancel and Interrupt no longer wait behind the active Workspace execution
  lock, and restart cancels Agent tasks and stale approvals before relaunch.
- Project file and render paths are rejected before any out-of-root filesystem
  side effect or document execution can occur.
- Closed dirty drafts have synchronous browser fallback persistence so recent
  edits survive normal application close and restart.
- Project file writes and project switches now advance `project_revision`.
- Object previews cap long strings and nested cells instead of bounding only
  row and column counts, including long list element names.
- Render and plot provenance now use the editor's actual document version and
  no longer mark Console-only plots as complete source provenance.

## 0.2.0-dev.1 - 2026-07-16

### Added

- First `0.2.x` development build for real project files.
- Broker-safe project root and source-file listing.
- File-tree and multiple document-tab state in the workbench.
- Read, save and create-file commands for supported source files.
- Workspace R working-directory synchronization when a project is opened.

### Not Yet Complete

- Native directory picker, durable document restoration, language-aware
  completion, approval dialogs, cancellation and crash recovery remain in the
  rest of the `0.2.x` milestone.

## 0.1.1 - 2026-07-16

### Added

- Draggable horizontal divider between the source editor and the Console,
  Plots and Problems dock.
- Draggable vertical dividers for the Files and Agent/Environment panels.
- Persistent panel sizes, keyboard arrow adjustment and double-click reset.
- Expand/restore control for the execution dock, useful for inspecting plots.
- Mouse and Pointer Event support for panel resizing.
- Windows NSIS installer rebuilt with the resizable workbench.

### Changed

- Prototype version advanced from `0.1.0` to `0.1.1`.
- Windows prototype documentation now describes panel layout behavior and
  the current development boundary.

## 0.1.0 - 2026-07-16

### Added

- First installable Windows Tauri prototype.
- Ark-backed persistent Workspace R session with no Python or Jupyter Server.
- Rust broker using direct Jupyter/ZeroMQ transport.
- R source editor, live Console, Environment object manifest, Plots and
  structured Problems surface.
- Ask, Plan and Act Agent modes backed by `YuLab-SMU/aisdk`.
- DeepSeek end-to-end Agent turn against the same Workspace R session.
- Broker-owned SQLite event store, workspace revisions and stale-context
  rejection.
- Windows installer carrying Ark, `WebView2Loader.dll` and runtime notices.

### Verification

- Rust workspace tests, `rho.agent` tests and `rho.bridge` tests pass.
- Installed release verified to launch Ark from the installation directory.
- Desktop smoke test verified R execution, plot output and Environment state.
