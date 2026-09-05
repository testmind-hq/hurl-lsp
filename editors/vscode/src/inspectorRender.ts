import { InspectorSnapshot } from "./inspectorStore";
import { BodyContent, HeaderField, HttpExchange, RunResult } from "./protocol";
import { Edge, Entry } from "./webviewModel";

export type DocumentViewModel = { uri: string; fileName: string; version: number; selectedIndex: number; entries: Entry[]; edges: Edge[] };
type WebviewLike = { cspSource: string };

export function escapeHtml(value: unknown): string {
  return String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

export function formatBody(body: BodyContent | undefined): string {
  if (!body) return "(empty)";
  if (body.encoding === "binary") return `[Binary body: ${body.originalBytes} bytes]`;
  let text = body.text ?? "";
  const isJson = body.mediaType?.toLowerCase().includes("json");
  if (isJson) { try { text = JSON.stringify(JSON.parse(text), null, 2); } catch { /* retain source */ } }
  if (body.truncated) text += `\n\n[Truncated; original size ${body.originalBytes} bytes]`;
  return text;
}

function headers(items: HeaderField[], reveal: boolean): string {
  if (!items.length) return '<div class="muted">No headers</div>';
  return `<table>${items.map((h) => `<tr><th>${escapeHtml(h.name)}</th><td>${escapeHtml(h.sensitive && !reveal ? "••••••" : h.value)}</td></tr>`).join("")}</table>`;
}

function exchange(value: HttpExchange, reveal: boolean): string {
  const response = value.response;
  return `<div class="grid"><section class="card"><h3>Request</h3><div class="headline">${escapeHtml(value.request.method)} ${escapeHtml(value.request.url)}</div>${headers(value.request.headers, reveal)}<pre>${escapeHtml(formatBody(value.request.body))}</pre></section><section class="card"><h3>Response</h3>${response ? `<div class="headline">${escapeHtml(response.version ?? "HTTP")} ${escapeHtml(response.status ?? "")}</div>${headers(response.headers, reveal)}<pre>${escapeHtml(formatBody(response.body))}</pre>` : '<div class="muted">No HTTP response</div>'}</section></div>`;
}

function resultView(result: RunResult | undefined, snapshot: InspectorSnapshot): string {
  if (!result) return '<div class="empty">Run a request to inspect its response.</div>';
  const reveal = snapshot.revealSecrets;
  const badge = result.success ? '<span class="ok">Passed</span>' : '<span class="fail">Failed</span>';
  const assertions = result.failedAssertions.length ? `<section class="card"><h3>Failed assertions</h3><ul>${result.failedAssertions.map((a) => `<li>${escapeHtml(a.message)}${a.line === undefined ? "" : ` — line ${a.line + 1}`}</li>`).join("")}</ul></section>` : "";
  const history = snapshot.runs.length > 1 ? `<select data-type="select-run">${snapshot.runs.map((run, index) => `<option value="${index}" ${index === snapshot.selectedRun ? "selected" : ""}>${escapeHtml(`${run.success ? "✓" : "✗"} line ${run.entryLine + 1} · ${run.durationMs ?? "?"}ms · ${run.startedAt}`)}</option>`).join("")}</select>` : "";
  return `<div class="toolbar">${badge}<span>${escapeHtml(result.durationMs ?? "?")} ms</span>${history}<button data-type="toggle-secrets">${reveal ? "Hide secrets" : "Reveal secrets"}</button></div>${result.parseWarning ? `<div class="warning">${escapeHtml(result.parseWarning)}</div>` : ""}${result.exchanges.map((item) => exchange(item, reveal)).join("")}${assertions}<section class="card"><h3>Raw stdout</h3><pre>${escapeHtml(result.stdout || "(empty)")}</pre><h3>Raw stderr</h3><pre>${escapeHtml(result.stderr || "(empty)")}</pre></section>`;
}

function requestView(model: DocumentViewModel | undefined): string {
  const entry = model && model.selectedIndex >= 0 ? model.entries[model.selectedIndex] : undefined;
  if (!entry || !model) return '<div class="empty">Open a .hurl request.</div>';
  return `<section class="card"><h3>${escapeHtml(entry.method)} ${escapeHtml(entry.target)}</h3><div class="muted">Line ${entry.line + 1}</div><div class="actions"><button data-type="run-entry" data-line="${entry.line}" data-uri="${escapeHtml(model.uri)}">Run</button><button data-type="run-vars" data-line="${entry.line}" data-uri="${escapeHtml(model.uri)}">Run with vars</button><button data-type="run-chain" data-line="${entry.line}" data-uri="${escapeHtml(model.uri)}">Run chain</button><button data-type="copy-curl-command" data-line="${entry.line}" data-uri="${escapeHtml(model.uri)}">Copy as cURL</button></div><pre>${escapeHtml(entry.body)}</pre></section>`;
}

function chainView(model: DocumentViewModel | undefined): string {
  if (!model?.entries.length) return '<div class="empty">No request chain.</div>';
  return `<section class="card"><h3>Entries</h3>${model.entries.map((e, i) => `<div class="node">${i + 1}. ${escapeHtml(e.method)} ${escapeHtml(e.target)}</div>`).join("")}<h3>Dependencies</h3>${model.edges.length ? model.edges.map((e) => `<div class="muted">${e.from + 1} → ${e.to + 1}${e.variables.length ? ` (${escapeHtml(e.variables.join(", "))})` : ""}</div>`).join("") : '<div class="muted">No dependencies inferred.</div>'}</section>`;
}

function curlView(snapshot: InspectorSnapshot): string {
  const value = snapshot.curl;
  if (!value) return '<div class="empty">Choose Copy as cURL for a request.</div>';
  if (!value.ok) return `<section class="card"><h3>Unable to build cURL</h3><div class="fail">${escapeHtml(value.error ?? "Unknown error")}</div>${value.unresolvedVariables.length ? `<p>Unresolved: ${escapeHtml(value.unresolvedVariables.join(", "))}</p>` : ""}</section>`;
  const shown = snapshot.revealSecrets ? value.command : (value.displayCommand ?? value.command);
  return `<div class="toolbar"><button data-type="toggle-secrets">${snapshot.revealSecrets ? "Hide secrets" : "Reveal secrets"}</button><button data-type="copy-curl">Copy again</button></div><pre>${escapeHtml(shown ?? "")}</pre>`;
}

export function renderInspectorHtml(webview: WebviewLike, model: DocumentViewModel | undefined, snapshot: InspectorSnapshot): string {
  const nonce = String(Date.now());
  const tabs = (["request", "chain", "result", "curl"] as const).map((tab) => `<button class="tab ${snapshot.tab === tab ? "active" : ""}" data-type="select-tab" data-tab="${tab}">${tab === "curl" ? "cURL" : tab[0].toUpperCase() + tab.slice(1)}</button>`).join("");
  const selected = snapshot.runs[snapshot.selectedRun];
  const content = snapshot.tab === "request" ? requestView(model) : snapshot.tab === "chain" ? chainView(model) : snapshot.tab === "result" ? resultView(selected, snapshot) : curlView(snapshot);
  return `<!doctype html><html><head><meta charset="UTF-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}';"><style>body{padding:16px;color:var(--vscode-editor-foreground);background:var(--vscode-editor-background);font-family:var(--vscode-font-family)}.tabs,.toolbar,.actions{display:flex;gap:8px;align-items:center;margin-bottom:12px;flex-wrap:wrap}button{color:var(--vscode-button-foreground);background:var(--vscode-button-background);border:0;border-radius:5px;padding:6px 10px;cursor:pointer}.tab:not(.active){color:var(--vscode-editor-foreground);background:transparent;border:1px solid var(--vscode-panel-border)}.card{border:1px solid var(--vscode-panel-border);border-radius:8px;padding:12px;margin-bottom:12px;min-width:0}.grid{display:grid;grid-template-columns:1fr 1fr;gap:12px}@media(max-width:900px){.grid{grid-template-columns:1fr}}pre{white-space:pre-wrap;overflow:auto;background:var(--vscode-textCodeBlock-background);padding:10px;border-radius:6px}table{width:100%;border-collapse:collapse;margin:8px 0}th,td{text-align:left;vertical-align:top;border-bottom:1px solid var(--vscode-panel-border);padding:5px}.muted,.empty{color:var(--vscode-descriptionForeground)}.ok{color:var(--vscode-testing-iconPassed)}.fail{color:var(--vscode-testing-iconFailed)}.warning{color:var(--vscode-editorWarning-foreground);margin:8px 0}.headline{font-weight:700;word-break:break-all}.node{padding:5px 0}</style></head><body><h2>Hurl Inspector${model ? ` — ${escapeHtml(model.fileName)}` : ""}</h2><div class="tabs">${tabs}</div>${content}<script nonce="${nonce}">const vscode=acquireVsCodeApi();document.addEventListener("click",e=>{const t=e.target;if(!(t instanceof HTMLElement)||!t.dataset.type)return;vscode.postMessage({...t.dataset});});document.addEventListener("change",e=>{const t=e.target;if(t instanceof HTMLSelectElement&&t.dataset.type==="select-run")vscode.postMessage({type:"select-run",index:t.value});});</script></body></html>`;
}
