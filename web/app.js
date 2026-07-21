const views = {
  runs: ["Runs", "Every scientific execution, revision, and outcome."],
  objects: ["Objects", "Live R state as bounded scientific semantics."],
  plots: ["Plots", "Figures and their provenance-bearing source runs."],
  problems: ["Problems", "Structured failures, calls, and tracebacks."],
  approvals: ["Approvals", "Human decisions around consequential agent actions."],
  provenance: ["Provenance", "How code, objects, results, and artifacts relate."],
};

const state = { workspace: null, dependencies: null, active: "runs", socket: null, refreshTimer: null, runtimeTimer: null, controlPlane: "connecting", inspection: null };
const content = document.querySelector("#view-content");
const summary = document.querySelector("#runtime-summary");
const title = document.querySelector("#view-title");
const description = document.querySelector("#view-description");
const label = document.querySelector("#workspace-label");
const dot = document.querySelector("#connection-dot");
const toast = document.querySelector("#toast");
const setupDialog = document.querySelector("#agent-setup-dialog");
const setupPrompt = document.querySelector("#agent-setup-prompt");
const copySetupLabel = document.querySelector("#copy-agent-setup-label");
const setupControlPlane = document.querySelector("#setup-control-plane");
const setupRuntimeUrl = document.querySelector("#setup-runtime-url");
const setupWorkspaceStatus = document.querySelector("#setup-workspace-status");
const setupWorkspaceMeaning = document.querySelector("#setup-workspace-meaning");
const codexHandoff = document.querySelector("#codex-handoff");
const dependencyBanner = document.querySelector("#dependency-banner");

const escapeHtml = (value) => String(value ?? "")
  .replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
  .replaceAll('"', "&quot;").replaceAll("'", "&#039;");
const displayTime = (value) => value ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(value)) : "—";
const asList = (value) => Array.isArray(value) ? value : [];
const shellQuote = (value) => `'${String(value).replaceAll("'", "'\"'\"'")}'`;

async function api(path, options = {}) {
  const response = await fetch(path, {
    method: options.method || "GET",
    headers: { accept: "application/json", ...(options.body ? { "content-type": "application/json" } : {}) },
    body: options.body ? JSON.stringify(options.body) : undefined,
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(payload.message || `Request failed (${response.status})`);
  return payload.data;
}

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("show");
  window.setTimeout(() => toast.classList.remove("show"), 2600);
}

function buildAgentSetupPrompt() {
  const setupUrl = new URL("/agent-setup.md", window.location.href).href;
  const runtimeUrl = window.location.origin;
  const projectRoot = state.workspace?.project_root || "PROJECT_ROOT";
  return `Read ${setupUrl} and follow it exactly once to install Rho's official skill for this Agent in the project at ${JSON.stringify(projectRoot)} and connect it to the Rho runtime at ${runtimeUrl}.`;
}

function lifecycleMeaning(lifecycle) {
  const meanings = {
    starting: "Rho is preparing and starting the managed R/Ark runtime",
    ready: "Ark/R kernel is attached and ready",
    busy: "Ark/R kernel is executing work",
    restarting: "Ark/R kernel is restarting",
    disconnected: "Workspace R is not attached; inspect dependency status below",
    failed: "Ark/R kernel failed; the control plane may still be online",
  };
  return meanings[lifecycle] || "Kernel readiness, not Agent registration";
}

function dependencyStatusMeaning(report) {
  if (!report) return "Checking R, Ark, and the workspace binding";
  if (report.ready && !["ready", "busy"].includes(state.workspace?.lifecycle)) {
    return "R, Ark, the controlled binding, and rho.bridge are ready; Workspace R can now be started";
  }
  if (report.ready) return "R, Ark, the controlled binding, and rho.bridge are ready";
  if (report.issue?.message) return report.issue.message;
  const phases = {
    discovering: "Discovering compatible R and cached runtime components",
    downloading: "Downloading a checksum-pinned runtime component",
    verifying: "Verifying the downloaded artifact",
    installing: "Publishing runtime components atomically",
    generating_kernelspec: "Generating the controlled Workspace R binding",
    smoke_testing: "Verifying Workspace R",
  };
  return phases[report.phase] || "Runtime dependencies are not ready";
}

function renderDependencyBanner() {
  const report = state.dependencies;
  if (!report) return;
  dependencyBanner.className = `dependency-banner ${escapeHtml(report.status)}`;
  const components = asList(report.components).map((component) => `
    <span class="dependency-component ${escapeHtml(component.status)}">
      <b>${escapeHtml(component.name)}</b>
      <span>${escapeHtml(component.version || component.status)}</span>
    </span>`).join("");
  const availableActions = [...asList(report.available_actions)];
  if (report.ready && !["ready", "busy"].includes(state.workspace?.lifecycle)
      && !availableActions.some((action) => action.id === "ensure")) {
    availableActions.push({ id: "ensure", label: "Start Workspace R", requires_human: false });
  }
  const actions = availableActions.map((action) => `
    <button type="button" data-dependency-action="${escapeHtml(action.id)}" data-requires-human="${action.requires_human ? "true" : "false"}">
      ${escapeHtml(action.label)}
    </button>`).join("");
  dependencyBanner.innerHTML = `
    <div class="dependency-copy">
      <span class="dependency-kicker">Runtime dependencies · ${escapeHtml(report.phase)}</span>
      <strong>${report.ready && ["ready", "busy"].includes(state.workspace?.lifecycle) ? "Workspace runtime ready" : report.ready ? "Runtime dependencies ready" : escapeHtml(report.issue?.title || report.status)}</strong>
      <small>${escapeHtml(dependencyStatusMeaning(report))}</small>
      <div class="dependency-components">${components}</div>
    </div>
    ${actions ? `<div class="dependency-actions">${actions}</div>` : ""}`;
}

async function runDependencyAction(action, requiresHuman) {
  if (requiresHuman && !window.confirm("This action changes a Rho-managed runtime component. Continue?")) return;
  const button = [...dependencyBanner.querySelectorAll("[data-dependency-action]")]
    .find((candidate) => candidate.dataset.dependencyAction === action);
  if (button) { button.disabled = true; button.textContent = "Working…"; }
  try {
    state.dependencies = await api("/v1/runtime/dependencies", {
      method: "POST",
      body: { action, confirmed: requiresHuman },
    });
    renderDependencyBanner();
    await refreshRuntimeState();
    showToast(state.dependencies.ready ? "Workspace R is ready" : dependencyStatusMeaning(state.dependencies));
  } catch (error) {
    showToast(error.message);
    await refreshRuntimeState();
  }
}

function renderAgentSetupContext() {
  const runtimeUrl = window.location.origin;
  const projectRoot = state.workspace?.project_root || "PROJECT_ROOT";
  const lifecycle = state.workspace?.lifecycle;
  setupControlPlane.textContent = state.controlPlane;
  setupControlPlane.classList.toggle("warning", state.controlPlane !== "online");
  setupRuntimeUrl.textContent = runtimeUrl;
  setupWorkspaceStatus.textContent = lifecycle || "Unavailable";
  setupWorkspaceStatus.className = lifecycle ? `lifecycle-${lifecycle}` : "warning";
  setupWorkspaceMeaning.textContent = state.dependencies?.ready
    ? lifecycleMeaning(lifecycle)
    : dependencyStatusMeaning(state.dependencies);
  codexHandoff.textContent = `codex -C ${shellQuote(projectRoot)}`;
}

function openAgentSetup() {
  renderAgentSetupContext();
  setupPrompt.textContent = buildAgentSetupPrompt();
  copySetupLabel.textContent = "Copy one-sentence setup";
  setupDialog.showModal();
}

async function copyAgentSetup() {
  const prompt = buildAgentSetupPrompt();
  setupPrompt.textContent = prompt;
  try {
    await navigator.clipboard.writeText(prompt);
  } catch (_) {
    const field = document.createElement("textarea");
    field.value = prompt;
    field.setAttribute("readonly", "");
    field.style.position = "fixed";
    field.style.opacity = "0";
    document.body.append(field);
    field.select();
    document.execCommand("copy");
    field.remove();
  }
  copySetupLabel.textContent = "Copied — paste into your Agent";
  showToast("Agent setup prompt copied");
}

function renderSummary() {
  const ws = state.workspace;
  if (!ws) return;
  const metrics = [
    ["Lifecycle", ws.lifecycle, lifecycleMeaning(ws.lifecycle)],
    ["Dependencies", state.dependencies?.status || "checking", state.dependencies?.phase || "discovering"],
    ["State revision", ws.identity.state_revision, `Execution ${ws.identity.execution_seq}`],
    ["Project revision", ws.identity.project_revision, ws.project_root || "No project root"],
    ["Kernel identity", ws.identity.kernel_instance_id, ws.workspace_id],
  ];
  summary.innerHTML = metrics.map(([name, value, detail]) => `
    <article class="metric">
      <span class="metric-label">${escapeHtml(name)}</span>
      <strong class="metric-value">${escapeHtml(value)}</strong>
      <span class="metric-detail truncate">${escapeHtml(detail)}</span>
    </article>`).join("");
}

async function refreshRuntimeState() {
  const [workspace, dependencies] = await Promise.all([
    api("/v1/workspaces/current"),
    api("/v1/runtime/dependencies"),
  ]);
  state.workspace = workspace;
  state.dependencies = dependencies;
  label.textContent = `Control plane online · R ${workspace.lifecycle}`;
  renderSummary();
  renderDependencyBanner();
  if (setupDialog.open) renderAgentSetupContext();
  window.clearTimeout(state.runtimeTimer);
  if (!dependencies.ready || !["ready", "busy"].includes(workspace.lifecycle)) {
    state.runtimeTimer = window.setTimeout(() => refreshRuntimeState().catch(() => {}), 1000);
  }
}

function panelHead(titleText, detail, count) {
  return `<header class="panel-head"><div><h2>${escapeHtml(titleText)}</h2><p>${escapeHtml(detail)}</p></div><span class="count-badge">${count}</span></header>`;
}

function emptyState(noun, copy) {
  return `<div class="empty-state"><div><strong>No ${escapeHtml(noun)} yet</strong>${escapeHtml(copy)}</div></div>`;
}

async function renderRuns() {
  const runs = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/runs`);
  if (!runs.length) return emptyState("runs", "Executed code will appear here with revision and provenance details.");
  return `${panelHead("Run history", "Newest first · persisted independently of browser state", runs.length)}
    <div class="table-wrap"><table><thead><tr><th>Run</th><th>Request</th><th>Status</th><th>Revision</th><th>Started</th></tr></thead><tbody>
    ${runs.map((run) => `<tr>
      <td class="mono"><span class="truncate">${escapeHtml(run.run_id)}</span><small class="subtle">${escapeHtml(run.origin)}</small></td>
      <td><span class="truncate">${escapeHtml(run.code || run.request_type)}</span><small class="subtle">${escapeHtml(run.operation_class)}</small></td>
      <td><span class="status-badge ${escapeHtml(run.status)}">${escapeHtml(run.status)}</span></td>
      <td class="mono">S ${escapeHtml(run.state_revision_before ?? "—")} → ${escapeHtml(run.state_revision_after ?? "—")}<small class="subtle">P ${escapeHtml(run.project_revision_before ?? "—")} → ${escapeHtml(run.project_revision_after ?? "—")}</small></td>
      <td>${escapeHtml(displayTime(run.started_at))}<small class="subtle">${escapeHtml(displayTime(run.finished_at))}</small></td>
    </tr>`).join("")}</tbody></table></div>`;
}

async function inspectObject(objectId) {
  const initialTarget = document.querySelector("#inspection");
  if (!initialTarget) return;
  initialTarget.innerHTML = `<div class="loading-state"><span></span>Inspecting live object…</div>`;
  try {
    const object = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/objects/${encodeURIComponent(objectId)}`);
    state.inspection = object;
    const currentTarget = document.querySelector("#inspection");
    if (currentTarget) currentTarget.innerHTML = renderInspection(object);
  } catch (error) {
    const currentTarget = document.querySelector("#inspection");
    if (currentTarget) currentTarget.innerHTML = `<div class="error-state"><div><strong>Inspection failed</strong>${escapeHtml(error.message)}</div></div>`;
  }
}

function renderInspection(object) {
  if (!object) return "";
  return `<div class="inspection"><span class="card-kicker">Bounded inspection</span><h3>${escapeHtml(object.name)}</h3><pre>${escapeHtml(JSON.stringify(object.metadata, null, 2))}</pre></div>`;
}

async function renderObjects() {
  const objects = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/objects`);
  if (!objects.length) return emptyState("objects", "Start Workspace R and create an object; values remain in R and only bounded metadata is projected here.");
  if (state.inspection && !objects.some((object) => object.object_id === state.inspection.object_id)) state.inspection = null;
  window.inspectRhoObject = inspectObject;
  return `${panelHead("Live environment", "Values stay in Workspace R · previews are bounded", objects.length)}<div class="card-grid">
    ${objects.map((object) => `<article class="object-card">
      <span class="card-kicker">${escapeHtml(object.r_type)}</span><h3>${escapeHtml(object.name)}</h3>
      <p class="object-meta">${object.dimensions.length ? `${object.dimensions.join(" × ")} dimensions` : "No dimensions"} · ${escapeHtml(object.metadata.preview_kind || "opaque")}</p>
      <div class="class-list">${asList(object.class).map((name) => `<span class="class-chip">${escapeHtml(name)}</span>`).join("")}</div>
      <button class="inspect-button" type="button" onclick="inspectRhoObject('${escapeHtml(object.object_id)}')">Inspect object</button>
    </article>`).join("")}<div id="inspection">${renderInspection(state.inspection)}</div></div>`;
}

function plotSource(plot) {
  const raw = plot.metadata?.payload?.data ?? plot.metadata?.payload;
  if (typeof raw !== "string") return null;
  if (plot.media_type === "image/png") return `data:image/png;base64,${raw}`;
  if (plot.media_type === "image/svg+xml") return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(raw)}`;
  return null;
}

async function renderPlots() {
  const plots = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/plots`);
  if (!plots.length) return emptyState("plots", "Figures emitted by Workspace R will appear with source and revision metadata.");
  return `${panelHead("Scientific figures", "Rendered output linked to the run that produced it", plots.length)}<div class="card-grid">
    ${plots.map((plot) => { const source = plotSource(plot); return `<article class="plot-card">
      <div class="plot-frame">${source ? `<img src="${source}" alt="Plot artifact ${escapeHtml(plot.artifact_id)}" />` : `<span class="object-meta">Preview unavailable for ${escapeHtml(plot.media_type)}</span>`}</div>
      <span class="card-kicker">${escapeHtml(plot.media_type)}</span><h3 class="mono">${escapeHtml(plot.artifact_id)}</h3>
      <p class="card-copy">Run ${escapeHtml(plot.run_id || "unknown")} · ${plot.metadata?.provenance_complete ? "complete provenance" : "partial provenance"}</p>
    </article>`; }).join("")}</div>`;
}

async function renderProblems() {
  const problems = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/problems`);
  if (!problems.length) return emptyState("problems", "The runtime has no recorded scientific execution failures.");
  return `${panelHead("Structured problems", "Errors retained with source, call, and traceback", problems.length)}<div class="card-grid">
    ${problems.map((problem) => `<article class="problem-card"><span class="card-kicker severity">${escapeHtml(problem.severity)}</span><h3>${escapeHtml(problem.message)}</h3><p class="card-copy mono">${escapeHtml(problem.call || problem.source_path || problem.run_id || "No call context")}</p>${problem.traceback?.length ? `<p class="card-copy">${escapeHtml(problem.traceback.join(" ← "))}</p>` : ""}</article>`).join("")}</div>`;
}

async function renderApprovals() {
  const approvals = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/approvals`);
  if (!approvals.length) return emptyState("approvals", "Consequential agent actions will wait here for a human decision.");
  return `${panelHead("Policy decisions", "One policy surface shared by every client", approvals.length)}<div class="card-grid">
    ${approvals.map((approval) => `<article class="approval-card"><span class="status-badge ${escapeHtml(approval.status)}">${escapeHtml(approval.status)}</span><h3>${escapeHtml(approval.action)}</h3><p class="card-copy">${escapeHtml(approval.policy)}</p><p class="card-copy mono">${escapeHtml(JSON.stringify(approval.arguments))}</p></article>`).join("")}</div>`;
}

async function renderProvenance() {
  const graph = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/provenance`);
  if (!graph.nodes.length) return emptyState("provenance", "Runs and artifacts will form a scientific lineage as work is executed.");
  return `${panelHead("Scientific lineage", `Graph revision ${graph.revision}`, `${graph.nodes.length} · ${graph.edges.length}`)}<div class="provenance-layout"><div class="node-list">
    ${graph.nodes.map((node) => `<article class="prov-node ${escapeHtml(node.kind)}"><span class="card-kicker">${escapeHtml(node.kind)}</span><h3>${escapeHtml(node.label)}</h3><code>${escapeHtml(node.node_id)}</code></article>`).join("")}
    </div><aside class="edge-list"><h2>Relationships</h2>${graph.edges.length ? graph.edges.map((edge) => `<div class="edge"><strong>${escapeHtml(edge.relation)}</strong>${escapeHtml(edge.from_node_id)} → ${escapeHtml(edge.to_node_id)}</div>`).join("") : `<p class="card-copy">No edges recorded yet.</p>`}</aside></div>`;
}

const renderers = { runs: renderRuns, objects: renderObjects, plots: renderPlots, problems: renderProblems, approvals: renderApprovals, provenance: renderProvenance };

async function refreshView({ quiet = false } = {}) {
  if (!state.workspace) return;
  if (!quiet) content.innerHTML = `<div class="loading-state"><span></span>Reading ${escapeHtml(state.active)}…</div>`;
  try { content.innerHTML = await renderers[state.active](); }
  catch (error) { content.innerHTML = `<div class="error-state"><div><strong>Scientific state unavailable</strong>${escapeHtml(error.message)}</div></div>`; }
}

function selectView(view) {
  state.active = view;
  document.querySelectorAll(".nav-item").forEach((item) => item.classList.toggle("active", item.dataset.view === view));
  [title.textContent, description.textContent] = views[view];
  history.replaceState(null, "", `#${view}`);
  refreshView();
}

function connectEvents() {
  state.socket?.close();
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  const id = encodeURIComponent(state.workspace.workspace_id);
  const socket = new WebSocket(`${scheme}://${location.host}/v1/workspaces/${id}/events/ws?client=web`);
  state.socket = socket;
  socket.addEventListener("open", () => {
    state.controlPlane = "online";
    dot.classList.add("live");
    label.textContent = `Control plane online · R ${state.workspace.lifecycle}`;
    refreshRuntimeState().catch(() => {});
  });
  socket.addEventListener("message", () => {
    window.clearTimeout(state.refreshTimer);
    state.refreshTimer = window.setTimeout(async () => {
      await refreshRuntimeState().catch(() => {});
      await refreshView({ quiet: true });
    }, 180);
  });
  socket.addEventListener("close", () => {
    state.controlPlane = "reconnecting";
    dot.classList.remove("live");
    label.textContent = `Control plane reconnecting · R ${state.workspace.lifecycle}`;
    window.setTimeout(connectEvents, 2000);
  });
}

async function bootstrap() {
  document.querySelector("#connect-agent-button").addEventListener("click", openAgentSetup);
  document.querySelector("#copy-agent-setup").addEventListener("click", copyAgentSetup);
  document.querySelector("#close-agent-setup").addEventListener("click", () => setupDialog.close());
  setupDialog.addEventListener("click", (event) => {
    if (event.target === setupDialog) setupDialog.close();
  });
  dependencyBanner.addEventListener("click", (event) => {
    const button = event.target.closest("[data-dependency-action]");
    if (button) runDependencyAction(button.dataset.dependencyAction, button.dataset.requiresHuman === "true");
  });
  document.querySelector("#workspace-nav").addEventListener("click", (event) => {
    const button = event.target.closest("[data-view]");
    if (button) selectView(button.dataset.view);
  });
  document.querySelector("#refresh-button").addEventListener("click", async () => {
    await refreshRuntimeState();
    await refreshView();
    showToast("Runtime and scientific state refreshed");
  });
  try {
    await refreshRuntimeState();
    state.controlPlane = "online";
    label.textContent = `Control plane online · R ${state.workspace.lifecycle}`;
    dot.classList.add("live");
    dot.classList.remove("error");
    renderSummary();
    const initial = location.hash.slice(1);
    selectView(views[initial] ? initial : "runs");
    connectEvents();
  } catch (error) {
    state.controlPlane = "unavailable";
    dot.classList.add("error"); label.textContent = "Runtime unavailable";
    content.innerHTML = `<div class="error-state"><div><strong>Cannot reach Rho</strong>${escapeHtml(error.message)}</div></div>`;
  }
}

bootstrap();
