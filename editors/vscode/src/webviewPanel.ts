import * as vscode from "vscode";
import { Edge, Entry, inferEdges, parseEntries, pickSelectedEntry } from "./webviewModel";

type ViewModel = {
  uri: string;
  fileName: string;
  version: number;
  selectedIndex: number;
  entries: Entry[];
  edges: Edge[];
};

type ParsedCache = {
  uri: string;
  version: number;
  fileName: string;
  entries: Entry[];
  edges: Edge[];
};

export function registerWebviewPanel(context: vscode.ExtensionContext, log: (message: string) => void): void {
  let panel: vscode.WebviewPanel | undefined;
  let parsedCache: ParsedCache | undefined;
  let updatePanel = (force = false) => {
    if (!force) {
      return;
    }
  };
  let scheduledRefresh: ReturnType<typeof setTimeout> | undefined;
  let lastRenderKey: string | undefined;

  const clearScheduledRefresh = () => {
    if (!scheduledRefresh) {
      return;
    }
    clearTimeout(scheduledRefresh);
    scheduledRefresh = undefined;
  };

  const schedulePanelUpdate = (force = false) => {
    if (!panel) {
      return;
    }
    clearScheduledRefresh();
    scheduledRefresh = setTimeout(() => {
      scheduledRefresh = undefined;
      updatePanel(force);
    }, 50);
  };

  const openOrReveal = () => {
    if (panel) {
      panel.reveal(vscode.ViewColumn.Beside, true);
      schedulePanelUpdate(true);
      return;
    }
    panel = vscode.window.createWebviewPanel("hurlWebviewPanel", "Hurl Webview", vscode.ViewColumn.Beside, {
      enableScripts: true,
      retainContextWhenHidden: true,
    });
    const onDispose = panel.onDidDispose(() => {
      clearScheduledRefresh();
      panel = undefined;
      parsedCache = undefined;
      lastRenderKey = undefined;
      onDispose.dispose();
      onDidReceiveMessage.dispose();
    });
    const onDidReceiveMessage = panel.webview.onDidReceiveMessage(
      async (msg: { type?: string; line?: number; uri?: string }) => {
        if (!msg.uri) {
          return;
        }
      if (msg.type === "run-entry" && typeof msg.line === "number") {
        await vscode.commands.executeCommand("hurl.runEntry", msg.uri, msg.line);
        log(`webview run entry requested at line=${msg.line}`);
        return;
      }
      if (msg.type === "run-chain" && typeof msg.line === "number") {
        await vscode.commands.executeCommand("hurl.runChain", msg.uri, msg.line);
        log(`webview run chain requested at line=${msg.line}`);
        return;
      }
      if (msg.type === "reveal-line" && typeof msg.line === "number") {
        const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(msg.uri));
        const editor = await vscode.window.showTextDocument(document, { preview: false, preserveFocus: true });
        const position = new vscode.Position(msg.line, 0);
        editor.selection = new vscode.Selection(position, position);
        editor.revealRange(new vscode.Range(position, position), vscode.TextEditorRevealType.InCenter);
        return;
      }
      },
    );

    updatePanel = (force = false) => {
      if (!panel) {
        return;
      }
      const activeEditor = vscode.window.activeTextEditor;
      const { model, cache } = buildModel(activeEditor, parsedCache);
      parsedCache = cache;
      const renderKey = model ? `${model.uri}@${model.version}:${model.selectedIndex}` : "empty";
      if (!force && renderKey === lastRenderKey) {
        return;
      }
      lastRenderKey = renderKey;
      panel.title = model ? `Hurl Webview — ${model.fileName}` : "Hurl Webview";
      panel.webview.html = renderHtml(panel.webview, model);
    };
    updatePanel(true);
  };

  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.openWebviewPanel", () => {
      openOrReveal();
    }),
  );
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => {
      parsedCache = undefined;
      schedulePanelUpdate(true);
    }),
  );
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      const active = vscode.window.activeTextEditor?.document;
      if (active && event.document.uri.toString() === active.uri.toString()) {
        parsedCache = undefined;
        schedulePanelUpdate(true);
      }
    }),
  );
  context.subscriptions.push(
    vscode.window.onDidChangeTextEditorSelection((event) => {
      const active = vscode.window.activeTextEditor;
      if (!active || event.textEditor.document.uri.toString() !== active.document.uri.toString()) {
        return;
      }
      schedulePanelUpdate();
    }),
  );
}

function buildModel(
  editor: vscode.TextEditor | undefined,
  cache: ParsedCache | undefined,
): { model: ViewModel | undefined; cache: ParsedCache | undefined } {
  if (!editor) {
    return { model: undefined, cache: undefined };
  }
  const doc = editor.document;
  if (doc.languageId !== "hurl" && !doc.fileName.endsWith(".hurl")) {
    return { model: undefined, cache: undefined };
  }
  const uri = doc.uri.toString();
  const fileName = doc.fileName.split(/[\\/]/).pop() ?? doc.fileName;
  const version = doc.version;
  let resolvedCache = cache;
  if (!resolvedCache || resolvedCache.uri !== uri || resolvedCache.version !== version) {
    const entries = parseEntries(doc.getText());
    resolvedCache = {
      uri,
      version,
      fileName,
      entries,
      edges: inferEdges(entries),
    };
  }
  const cursorLine = editor.selection.active.line;
  const selectedIndex = resolvedCache.entries.length > 0 ? pickSelectedEntry(resolvedCache.entries, cursorLine) : -1;
  return {
    model: {
      uri,
      fileName,
      version,
      selectedIndex,
      entries: resolvedCache.entries,
      edges: resolvedCache.edges,
    },
    cache: resolvedCache,
  };
}

function renderHtml(webview: vscode.Webview, model: ViewModel | undefined): string {
  const nonce = String(Date.now());
  const csp = `default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}';`;
  const payload = JSON.stringify(model ?? null).replace(/</g, "\\u003c");
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <meta http-equiv="Content-Security-Policy" content="${csp}" />
  <style>
    :root {
      --bg: var(--vscode-editor-background);
      --fg: var(--vscode-editor-foreground);
      --muted: var(--vscode-descriptionForeground);
      --accent: var(--vscode-button-background);
      --accent-fg: var(--vscode-button-foreground);
      --border: var(--vscode-panel-border);
      --chip: var(--vscode-badge-background);
    }
    body { margin: 0; padding: 16px; background: var(--bg); color: var(--fg); font-family: var(--vscode-font-family); }
    h1 { margin: 0 0 8px; font-size: 16px; }
    .tabs { display: flex; gap: 8px; margin: 12px 0; }
    .tab { border: 1px solid var(--border); background: transparent; color: var(--fg); padding: 6px 10px; cursor: pointer; border-radius: 6px; }
    .tab.active { background: var(--accent); color: var(--accent-fg); border-color: transparent; }
    .hidden { display: none; }
    .card { border: 1px solid var(--border); border-radius: 8px; padding: 12px; margin-bottom: 12px; }
    .title { font-weight: 700; margin-bottom: 6px; }
    .meta { color: var(--muted); font-size: 12px; margin: 3px 0; }
    .actions { display: flex; gap: 8px; margin-top: 12px; }
    .btn { background: var(--accent); color: var(--accent-fg); border: none; border-radius: 6px; padding: 6px 10px; cursor: pointer; }
    pre { background: color-mix(in srgb, var(--bg) 85%, var(--fg) 15%); padding: 10px; border-radius: 8px; overflow: auto; }
    .node { border: 1px solid var(--border); border-radius: 8px; padding: 8px; margin-bottom: 8px; }
    .edge { color: var(--muted); font-size: 12px; margin: 2px 0; }
    .chip { display: inline-block; background: var(--chip); color: var(--accent-fg); border-radius: 999px; padding: 2px 8px; font-size: 11px; margin-right: 6px; }
  </style>
</head>
<body>
  <h1>Hurl Webview</h1>
  <div id="empty" class="meta hidden">Open a .hurl file to view request details and chain graph.</div>
  <div id="content" class="hidden">
    <div class="tabs">
      <button class="tab active" id="tab-single">Single Request</button>
      <button class="tab" id="tab-chain">Chain Graph</button>
    </div>
    <section id="single"></section>
    <section id="chain" class="hidden"></section>
  </div>
  <script nonce="${nonce}">
    const vscode = acquireVsCodeApi();
    const data = ${payload};
    const empty = document.getElementById("empty");
    const content = document.getElementById("content");
    const single = document.getElementById("single");
    const chain = document.getElementById("chain");
    const tabSingle = document.getElementById("tab-single");
    const tabChain = document.getElementById("tab-chain");

    const escapeHtml = (v) => String(v)
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;");

    function button(label, type, line, uri) {
      return \`<button class="btn" data-type="\${type}" data-line="\${line}" data-uri="\${uri}">\${label}</button>\`;
    }

    function renderSingle(model) {
      if (!model || model.selectedIndex < 0 || !model.entries[model.selectedIndex]) {
        single.innerHTML = '<div class="meta">No request selected.</div>';
        return;
      }
      const e = model.entries[model.selectedIndex];
      const chips = [e.priority, e.caseId, e.stepType, e.stepId].filter(Boolean).map((c) => \`<span class="chip">\${escapeHtml(c)}</span>\`).join("");
      single.innerHTML = \`
        <div class="card">
          <div class="title">\${escapeHtml(e.method)} \${escapeHtml(e.target)}</div>
          <div class="meta">line: \${e.line + 1}</div>
          \${e.title ? \`<div class="meta">title: \${escapeHtml(e.title)}</div>\` : ""}
          <div style="margin:8px 0;">\${chips}</div>
          <div class="actions">
            \${button("Run", "run-entry", e.line, model.uri)}
            \${button("Run Chain", "run-chain", e.line, model.uri)}
            \${button("Reveal", "reveal-line", e.line, model.uri)}
          </div>
        </div>
        <pre>\${escapeHtml(e.body)}</pre>
      \`;
    }

    function renderChain(model) {
      if (!model || !model.entries.length) {
        chain.innerHTML = '<div class="meta">No request entries.</div>';
        return;
      }
      const nodes = model.entries.map((e, idx) =>
        \`<div class="node"><div><strong>\${idx + 1}. \${escapeHtml(e.method)} \${escapeHtml(e.target)}</strong></div><div class="meta">line \${e.line + 1}</div></div>\`
      ).join("");
      const edges = model.edges.length
        ? model.edges.map((edge) => {
            const from = model.entries[edge.from];
            const to = model.entries[edge.to];
            const vars = edge.variables.length ? \` (\${edge.variables.join(", ")})\` : "";
            const tag = edge.explicit ? "explicit" : "inferred";
            return \`<div class="edge">\${escapeHtml(from.method)} \${escapeHtml(from.target)} → \${escapeHtml(to.method)} \${escapeHtml(to.target)}\${escapeHtml(vars)} [\${tag}]</div>\`;
          }).join("")
        : '<div class="meta">No dependency edges inferred.</div>';
      chain.innerHTML = \`
        <div class="card">
          <div class="title">Entries</div>
          \${nodes}
        </div>
        <div class="card">
          <div class="title">Dependencies</div>
          \${edges}
        </div>
      \`;
    }

    function activate(tab) {
      const isSingle = tab === "single";
      tabSingle.classList.toggle("active", isSingle);
      tabChain.classList.toggle("active", !isSingle);
      single.classList.toggle("hidden", !isSingle);
      chain.classList.toggle("hidden", isSingle);
    }

    tabSingle.addEventListener("click", () => activate("single"));
    tabChain.addEventListener("click", () => activate("chain"));
    document.addEventListener("click", (event) => {
      const target = event.target;
      if (!(target instanceof HTMLElement)) return;
      const type = target.dataset.type;
      const line = Number(target.dataset.line);
      const uri = target.dataset.uri;
      if (!type || Number.isNaN(line)) return;
      if (!uri) return;
      vscode.postMessage({ type, line, uri });
    });

    if (!data) {
      empty.classList.remove("hidden");
    } else {
      content.classList.remove("hidden");
      renderSingle(data);
      renderChain(data);
    }
  </script>
</body>
</html>`;
}
