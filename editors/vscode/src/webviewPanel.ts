import * as vscode from "vscode";
import { InspectorStore, InspectorTab } from "./inspectorStore";
import { DocumentViewModel, renderInspectorHtml } from "./inspectorRender";
import { CurlResult, RunResult } from "./protocol";
import { Edge, Entry, inferEdges, parseEntries, pickSelectedEntry } from "./webviewModel";

type ParsedCache = { uri: string; version: number; fileName: string; entries: Entry[]; edges: Edge[] };
export type InspectorController = { open(tab?: InspectorTab): void; acceptRun(result: RunResult): void; acceptCurl(result: CurlResult): void; dispose(): void };

export function registerWebviewPanel(context: vscode.ExtensionContext, log: (message: string) => void): InspectorController {
  let panel: vscode.WebviewPanel | undefined;
  let cache: ParsedCache | undefined;
  let scheduled: ReturnType<typeof setTimeout> | undefined;
  let boundDocument: { uri: string; version: number; entryLine?: number } | undefined;
  const store = new InspectorStore();

  const model = (): DocumentViewModel | undefined => {
    const active = vscode.window.activeTextEditor;
    const doc = boundDocument
      ? vscode.workspace.textDocuments.find((item) => item.uri.toString() === boundDocument?.uri && item.version === boundDocument.version)
      : active?.document;
    if (!doc || (doc.languageId !== "hurl" && !doc.fileName.endsWith(".hurl"))) return undefined;
    if (!cache || cache.uri !== doc.uri.toString() || cache.version !== doc.version) {
      const entries = parseEntries(doc.getText());
      cache = { uri: doc.uri.toString(), version: doc.version, fileName: doc.fileName.split(/[\\/]/).pop() ?? doc.fileName, entries, edges: inferEdges(entries) };
    }
    const activeLine = active?.document.uri.toString() === doc.uri.toString() ? active.selection.active.line : boundDocument?.entryLine ?? 0;
    return { ...cache, selectedIndex: cache.entries.length ? pickSelectedEntry(cache.entries, activeLine) : -1 };
  };
  const render = () => { if (panel) { const value = model(); const fallbackName = boundDocument ? vscode.Uri.parse(boundDocument.uri).path.split("/").pop() : undefined; panel.title = `Hurl Inspector${value?.fileName || fallbackName ? ` — ${value?.fileName ?? fallbackName}` : ""}`; panel.webview.html = renderInspectorHtml(panel.webview, value, store.snapshot()); } };
  const schedule = () => { if (!panel) return; if (scheduled) clearTimeout(scheduled); scheduled = setTimeout(() => { scheduled = undefined; render(); }, 50); };

  const open = (tab?: InspectorTab, source?: { uri: string; version: number; entryLine?: number }) => {
    if (source) {
      boundDocument = source;
      store.selectDocument(source.uri, source.version);
      cache = undefined;
    } else if (!boundDocument) {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        boundDocument = { uri: editor.document.uri.toString(), version: editor.document.version, entryLine: editor.selection.active.line };
        store.selectDocument(boundDocument.uri, boundDocument.version);
      }
    }
    if (tab) store.select(tab);
    if (panel) { panel.reveal(vscode.ViewColumn.Beside, true); render(); return; }
    panel = vscode.window.createWebviewPanel("hurlInspector", "Hurl Inspector", vscode.ViewColumn.Beside, { enableScripts: true, retainContextWhenHidden: true });
    panel.onDidDispose(() => { panel = undefined; if (scheduled) clearTimeout(scheduled); }, null, context.subscriptions);
    panel.webview.onDidReceiveMessage(async (message: Record<string, string>) => {
      if (message.type === "select-tab" && ["request", "chain", "result", "curl"].includes(message.tab)) { store.select(message.tab as InspectorTab); render(); return; }
      if (message.type === "toggle-secrets") { store.toggleSecrets(); render(); return; }
      if (message.type === "select-run") { store.selectRun(Number(message.index)); render(); return; }
      if (message.type === "copy-curl") { const command = store.snapshot().curl?.command; if (command) { await vscode.env.clipboard.writeText(command); void vscode.window.showInformationMessage("cURL copied to clipboard"); } return; }
      const line = Number(message.line);
      if (!message.uri || Number.isNaN(line)) return;
      const commands: Record<string, string> = { "run-entry": "hurl.runEntry", "run-vars": "hurl.runEntryWithVars", "run-chain": "hurl.runChain", "copy-curl-command": "hurl.copyAsCurl" };
      const command = commands[message.type];
      if (command) { await vscode.commands.executeCommand(command, message.uri, line); log(`Inspector ${message.type} requested at line=${line}`); }
    }, null, context.subscriptions);
    render();
  };

  const controller: InspectorController = {
    open,
    acceptRun(result) { store.pushRun(result); open("result", { uri: result.uri, version: result.documentVersion, entryLine: result.entryLine }); },
    acceptCurl(result) { store.setCurl(result); open("curl", { uri: result.uri, version: result.documentVersion, entryLine: result.entryLine }); },
    dispose() { panel?.dispose(); panel = undefined; },
  };
  context.subscriptions.push(
    vscode.commands.registerCommand("hurl.openWebviewPanel", () => {
      const editor = vscode.window.activeTextEditor;
      open("request", editor ? { uri: editor.document.uri.toString(), version: editor.document.version, entryLine: editor.selection.active.line } : undefined);
    }),
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      const tab = store.snapshot().tab;
      if (editor && (tab === "request" || tab === "chain")) {
        boundDocument = { uri: editor.document.uri.toString(), version: editor.document.version, entryLine: editor.selection.active.line };
        store.selectDocument(boundDocument.uri, boundDocument.version);
      }
      cache = undefined;
      schedule();
    }),
    vscode.workspace.onDidChangeTextDocument(() => { cache = undefined; schedule(); }),
    vscode.window.onDidChangeTextEditorSelection(schedule),
    { dispose: () => controller.dispose() },
  );
  return controller;
}
