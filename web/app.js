const views = {
  runs: ["Runs", "Every scientific execution, revision, and outcome."],
  objects: ["Objects", "Live R state as bounded scientific semantics."],
  plots: ["Plots", "Figures and their provenance-bearing source runs."],
  problems: ["Problems", "Structured failures, calls, and tracebacks."],
  approvals: ["Approvals", "Human decisions around consequential agent actions."],
  provenance: ["Provenance", "How code, objects, results, and artifacts relate."],
};

const state = { workspace: null, active: "runs", socket: null, refreshTimer: null };
const content = document.querySelector("#view-content");
const summary = document.querySelector("#runtime-summary");
const title = document.querySelector("#view-title");
const description = document.querySelector("#view-description");
const label = document.querySelector("#workspace-label");
const dot = document.querySelector("#connection-dot");
const toast = document.querySelector("#toast");

const escapeHtml = (value) => String(value ?? "")
  .replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
  .replaceAll('"', "&quot;").replaceAll("'", "&#039;");
const displayTime = (value) => value ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(value)) : "—";
const asList = (value) => Array.isArray(value) ? value : [];

async function api(path) {
  const response = await fetch(path, { headers: { accept: "application/json" } });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(payload.message || `Request failed (${response.status})`);
  return payload.data;
}

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("show");
  window.setTimeout(() => toast.classList.remove("show"), 2600);
}

function renderSummary() {
  const ws = state.workspace;
  if (!ws) return;
  const metrics = [
    ["Lifecycle", ws.lifecycle, "Workspace R"],
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
  const target = document.querySelector("#inspection");
  if (!target) return;
  target.innerHTML = `<div class="loading-state"><span></span>Inspecting live object…</div>`;
  try {
    const object = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/objects/${encodeURIComponent(objectId)}`);
    target.innerHTML = `<div class="inspection"><span class="card-kicker">Bounded inspection</span><h3>${escapeHtml(object.name)}</h3><pre>${escapeHtml(JSON.stringify(object.metadata, null, 2))}</pre></div>`;
  } catch (error) {
    target.innerHTML = `<div class="error-state"><div><strong>Inspection failed</strong>${escapeHtml(error.message)}</div></div>`;
  }
}

async function renderObjects() {
  const objects = await api(`/v1/workspaces/${encodeURIComponent(state.workspace.workspace_id)}/objects`);
  if (!objects.length) return emptyState("objects", "Start Workspace R and create an object; values remain in R and only bounded metadata is projected here.");
  window.inspectRhoObject = inspectObject;
  return `${panelHead("Live environment", "Values stay in Workspace R · previews are bounded", objects.length)}<div class="card-grid">
    ${objects.map((object) => `<article class="object-card">
      <span class="card-kicker">${escapeHtml(object.r_type)}</span><h3>${escapeHtml(object.name)}</h3>
      <p class="object-meta">${object.dimensions.length ? `${object.dimensions.join(" × ")} dimensions` : "No dimensions"} · ${escapeHtml(object.metadata.preview_kind || "opaque")}</p>
      <div class="class-list">${asList(object.class).map((name) => `<span class="class-chip">${escapeHtml(name)}</span>`).join("")}</div>
      <button class="inspect-button" type="button" onclick="inspectRhoObject('${escapeHtml(object.object_id)}')">Inspect object</button>
    </article>`).join("")}<div id="inspection"></div></div>`;
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
  socket.addEventListener("open", () => dot.classList.add("live"));
  socket.addEventListener("message", () => {
    window.clearTimeout(state.refreshTimer);
    state.refreshTimer = window.setTimeout(() => refreshView({ quiet: true }), 180);
  });
  socket.addEventListener("close", () => { dot.classList.remove("live"); window.setTimeout(connectEvents, 2000); });
}

async function bootstrap() {
  document.querySelector("#workspace-nav").addEventListener("click", (event) => {
    const button = event.target.closest("[data-view]");
    if (button) selectView(button.dataset.view);
  });
  document.querySelector("#refresh-button").addEventListener("click", async () => {
    await refreshView(); showToast("Scientific state refreshed");
  });
  try {
    state.workspace = await api("/v1/workspaces/current");
    label.textContent = `${state.workspace.workspace_id} · ${state.workspace.lifecycle}`;
    dot.classList.toggle("live", state.workspace.lifecycle !== "disconnected" && state.workspace.lifecycle !== "failed");
    dot.classList.toggle("error", state.workspace.lifecycle === "failed");
    renderSummary();
    const initial = location.hash.slice(1);
    selectView(views[initial] ? initial : "runs");
    connectEvents();
  } catch (error) {
    dot.classList.add("error"); label.textContent = "Runtime unavailable";
    content.innerHTML = `<div class="error-state"><div><strong>Cannot reach Rho</strong>${escapeHtml(error.message)}</div></div>`;
  }
}

bootstrap();
