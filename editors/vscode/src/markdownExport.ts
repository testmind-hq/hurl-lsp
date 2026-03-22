import * as path from "node:path";
import * as vscode from "vscode";

const HTTP_METHOD_RE = /^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS|CONNECT|TRACE)\s+(.+)$/i;
const META_RE = /^\s*#\s*([a-z_]+)\s*=\s*(.+)\s*$/i;

type HurlEntry = {
  line: number;
  method: string;
  target: string;
  body: string;
  meta: EntryMeta;
};

type EntryMeta = {
  caseId?: string;
  caseKind?: "single" | "chain";
  priority?: "P0" | "P1" | "P2";
  stepId?: string;
  stepType?: "setup" | "test" | "teardown";
  title?: string;
  technique?: string;
};

type GroupMode = "hierarchical" | "flat";
type SortMode = "source" | "priority";

export async function exportActiveHurlAsMarkdown(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    void vscode.window.showWarningMessage("No active editor.");
    return;
  }
  if (editor.document.languageId !== "hurl" && !editor.document.fileName.endsWith(".hurl")) {
    void vscode.window.showWarningMessage("Active file is not a .hurl document.");
    return;
  }

  const source = editor.document.getText();
  const entries = parseEntries(source);
  if (entries.length === 0) {
    void vscode.window.showWarningMessage("No request entries found in current file.");
    return;
  }

  const config = vscode.workspace.getConfiguration("hurl");
  const groupMode = asGroupMode(config.get<string>("outline.groupMode", "hierarchical"));
  const sortMode = asSortMode(config.get<string>("outline.sortMode", "source"));
  const markdown = renderMarkdown(editor.document.fileName, entries, groupMode, sortMode);
  const sourcePath = editor.document.uri.fsPath;
  const ext = path.extname(sourcePath);
  const outputPath = sourcePath.slice(0, sourcePath.length - ext.length) + ".md";
  const outputUri = vscode.Uri.file(outputPath);
  await vscode.workspace.fs.writeFile(outputUri, Buffer.from(markdown, "utf8"));
  const doc = await vscode.workspace.openTextDocument(outputUri);
  await vscode.window.showTextDocument(doc, { preview: false });
  void vscode.window.showInformationMessage(`Exported Markdown: ${outputPath}`);
}

function parseEntries(source: string): HurlEntry[] {
  const lines = source.split(/\r?\n/);
  const methods: Array<{ line: number; method: string; target: string }> = [];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.trim().match(HTTP_METHOD_RE);
    if (match) {
      methods.push({ line: index, method: match[1].toUpperCase(), target: match[2].trim() });
    }
  }
  if (methods.length === 0) {
    return [];
  }

  const entries: HurlEntry[] = [];
  for (let i = 0; i < methods.length; i += 1) {
    const current = methods[i];
    const prevBoundary = i === 0 ? 0 : methods[i - 1].line + 1;
    const nextBoundary = i + 1 < methods.length ? methods[i + 1].line : lines.length;
    let meta: EntryMeta = {};
    for (let line = prevBoundary; line < current.line; line += 1) {
      const metaMatch = lines[line].match(META_RE);
      if (metaMatch) {
        meta = updateMeta(meta, metaMatch[1], metaMatch[2]);
      }
    }
    entries.push({
      line: current.line,
      method: current.method,
      target: current.target,
      body: lines.slice(current.line, bodyEndIndex(lines, current.line, nextBoundary)).join("\n").trim(),
      meta,
    });
  }
  return entries;
}

function bodyEndIndex(lines: string[], start: number, nextBoundary: number): number {
  let cursor = nextBoundary - 1;
  let sawMetaSuffix = false;
  while (cursor >= start) {
    const raw = lines[cursor];
    const trimmed = raw.trim();
    if (trimmed.length === 0) {
      cursor -= 1;
      continue;
    }
    if (META_RE.test(raw)) {
      sawMetaSuffix = true;
      cursor -= 1;
      continue;
    }
    break;
  }
  if (!sawMetaSuffix) {
    return nextBoundary;
  }
  return cursor + 1;
}

function updateMeta(meta: EntryMeta, rawKey: string, rawValue: string): EntryMeta {
  const key = rawKey.trim().toLowerCase();
  const value = rawValue.trim();
  const next = { ...meta };
  switch (key) {
    case "case_id":
      next.caseId = value;
      break;
    case "case_kind":
      if (value === "single" || value === "chain") {
        next.caseKind = value;
      }
      break;
    case "priority":
      if (value === "P0" || value === "P1" || value === "P2") {
        next.priority = value;
      }
      break;
    case "step_id":
      next.stepId = value;
      break;
    case "step_type": {
      const lowered = value.toLowerCase();
      if (lowered === "setup" || lowered === "test" || lowered === "teardown") {
        next.stepType = lowered;
      }
      break;
    }
    case "title":
      next.title = value;
      break;
    case "technique":
      next.technique = value;
      break;
    default:
      break;
  }
  return next;
}

function renderMarkdown(fileName: string, entries: HurlEntry[], groupMode: GroupMode, sortMode: SortMode): string {
  const displayName = path.basename(fileName);
  const lines: string[] = [];
  lines.push(`# ${displayName}`);
  lines.push("");
  lines.push(`Generated by hurl-lsp at ${new Date().toISOString()}`);
  lines.push("");
  lines.push(`Outline mode: group=\`${groupMode}\`, sort=\`${sortMode}\``);
  lines.push("");

  const sorted = [...entries].sort((a, b) => compareEntries(a, b, sortMode));
  if (groupMode === "flat") {
    lines.push("## Requests");
    lines.push("");
    appendEntries(lines, sorted);
  } else {
    lines.push("## Requests");
    lines.push("");
    const chainGroups = new Map<string, HurlEntry[]>();
    const singleGroups = new Map<"P0" | "P1" | "P2", HurlEntry[]>();
    const fallback: HurlEntry[] = [];
    for (const entry of sorted) {
      if (entry.meta.caseKind === "chain") {
        const caseId = entry.meta.caseId ?? "CHAIN";
        const bucket = chainGroups.get(caseId) ?? [];
        bucket.push(entry);
        chainGroups.set(caseId, bucket);
      } else if (entry.meta.priority) {
        const bucket = singleGroups.get(entry.meta.priority) ?? [];
        bucket.push(entry);
        singleGroups.set(entry.meta.priority, bucket);
      } else {
        fallback.push(entry);
      }
    }

    const groups: Array<{ title: string; entries: HurlEntry[]; startLine: number; rank: number }> = [];
    for (const [caseId, groupEntries] of chainGroups.entries()) {
      const ordered = [...groupEntries].sort((a, b) => compareEntries(a, b, sortMode));
      groups.push({
        title: `🔗 ${caseId}`,
        startLine: minSourceLine(ordered),
        rank: groupRank(ordered),
        entries: ordered,
      });
    }
    for (const priority of ["P0", "P1", "P2"] as const) {
      const groupEntries = singleGroups.get(priority) ?? [];
      if (groupEntries.length === 0) {
        continue;
      }
      const ordered = [...groupEntries].sort((a, b) => compareEntries(a, b, sortMode));
      groups.push({
        title: priority,
        startLine: minSourceLine(ordered),
        rank: groupRank(ordered),
        entries: ordered,
      });
    }

    if (fallback.length > 0) {
      const ordered = [...fallback].sort((a, b) => compareEntries(a, b, sortMode));
      groups.push({
        title: "Others",
        startLine: minSourceLine(ordered),
        rank: groupRank(ordered),
        entries: ordered,
      });
    }

    groups.sort((a, b) => {
      if (sortMode === "source") {
        return a.startLine - b.startLine;
      }
      if (a.rank !== b.rank) {
        return a.rank - b.rank;
      }
      return a.startLine - b.startLine;
    });

    for (const group of groups) {
      lines.push(`### ${group.title}`);
      lines.push("");
      appendEntries(lines, group.entries);
    }
  }
  return `${lines.join("\n")}\n`;
}

function groupRank(entries: HurlEntry[]): number {
  let rank = Number.POSITIVE_INFINITY;
  for (const entry of entries) {
    rank = Math.min(rank, priorityRank(entry.meta.priority));
  }
  return Number.isFinite(rank) ? rank : 3;
}

function minSourceLine(entries: HurlEntry[]): number {
  let line = Number.POSITIVE_INFINITY;
  for (const entry of entries) {
    line = Math.min(line, entry.line);
  }
  return Number.isFinite(line) ? line : 0;
}

function appendEntries(lines: string[], entries: HurlEntry[]): void {
  for (let index = 0; index < entries.length; index += 1) {
    const entry = entries[index];
    lines.push(`### ${index + 1}. ${entryHeading(entry)}`);
    const meta = entryMetaLine(entry.meta);
    if (meta) {
      lines.push("");
      lines.push(meta);
    }
    lines.push("");
    lines.push("```hurl");
    lines.push(entry.body);
    lines.push("```");
    lines.push("");
  }
}

function entryHeading(entry: HurlEntry): string {
  if (entry.meta.title) {
    return `${entry.method} ${entry.target} — ${entry.meta.title}`;
  }
  return `${entry.method} ${entry.target}`;
}

function entryMetaLine(meta: EntryMeta): string {
  const items: string[] = [];
  if (meta.caseId) items.push(`case_id=${meta.caseId}`);
  if (meta.caseKind) items.push(`case_kind=${meta.caseKind}`);
  if (meta.priority) items.push(`priority=${meta.priority}`);
  if (meta.stepId) items.push(`step_id=${meta.stepId}`);
  if (meta.stepType) items.push(`step_type=${meta.stepType}`);
  if (meta.technique) items.push(`technique=${meta.technique}`);
  return items.length > 0 ? `> ${items.join(", ")}` : "";
}

function compareEntries(a: HurlEntry, b: HurlEntry, sortMode: SortMode): number {
  if (sortMode === "source") {
    return a.line - b.line;
  }
  const priority = priorityRank(a.meta.priority) - priorityRank(b.meta.priority);
  if (priority !== 0) {
    return priority;
  }
  const step = stepRank(a.meta.stepType) - stepRank(b.meta.stepType);
  if (step !== 0) {
    return step;
  }
  return a.line - b.line;
}

function priorityRank(priority?: "P0" | "P1" | "P2"): number {
  switch (priority) {
    case "P0":
      return 0;
    case "P1":
      return 1;
    case "P2":
      return 2;
    default:
      return 3;
  }
}

function stepRank(step?: "setup" | "test" | "teardown"): number {
  switch (step) {
    case "setup":
      return 0;
    case "test":
      return 1;
    case "teardown":
      return 2;
    default:
      return 3;
  }
}

function asGroupMode(value: string): GroupMode {
  return value === "flat" ? "flat" : "hierarchical";
}

function asSortMode(value: string): SortMode {
  return value === "priority" ? "priority" : "source";
}
