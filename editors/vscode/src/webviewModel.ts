const HTTP_METHOD_RE = /^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS|CONNECT|TRACE)\s+(.+)$/i;
const META_RE = /^\s*#\s*([a-z_]+)\s*=\s*(.+)\s*$/i;
const SECTION_RE = /^\s*\[([^\]]+)\]\s*$/;
const VAR_RE = /{{\s*([a-zA-Z0-9_.-]+)\s*}}/g;

export type StepType = "setup" | "test" | "teardown";
export type Priority = "P0" | "P1" | "P2";

export type Entry = {
  index: number;
  line: number;
  method: string;
  target: string;
  body: string;
  title?: string;
  caseId?: string;
  caseKind?: "single" | "chain";
  stepId?: string;
  stepType?: StepType;
  priority?: Priority;
  dependsOn: string[];
};

export type Edge = {
  from: number;
  to: number;
  variables: string[];
  explicit: boolean;
};

export function pickSelectedEntry(entries: Entry[], line: number): number {
  let selected = 0;
  for (let i = 0; i < entries.length; i += 1) {
    if (entries[i].line <= line) {
      selected = i;
    } else {
      break;
    }
  }
  return selected;
}

export function parseEntries(source: string): Entry[] {
  const lines = source.split(/\r?\n/);
  const methods: Array<{ line: number; method: string; target: string }> = [];
  for (let i = 0; i < lines.length; i += 1) {
    const match = lines[i].trim().match(HTTP_METHOD_RE);
    if (match) {
      methods.push({ line: i, method: match[1].toUpperCase(), target: match[2].trim() });
    }
  }
  const entries: Entry[] = [];
  for (let i = 0; i < methods.length; i += 1) {
    const current = methods[i];
    const next = methods[i + 1];
    const start = current.line;
    const end = next ? next.line : lines.length;
    let title: string | undefined;
    let caseId: string | undefined;
    let caseKind: "single" | "chain" | undefined;
    let stepId: string | undefined;
    let stepType: StepType | undefined;
    let priority: Priority | undefined;
    let dependsOn: string[] = [];
    const prevBoundary = i === 0 ? 0 : methods[i - 1].line + 1;
    for (let j = prevBoundary; j < start; j += 1) {
      const meta = lines[j].match(META_RE);
      if (!meta) {
        continue;
      }
      const key = meta[1].trim().toLowerCase();
      const value = meta[2].trim();
      if (key === "title") title = value;
      if (key === "case_id") caseId = value;
      if (key === "case_kind" && (value === "single" || value === "chain")) caseKind = value;
      if (key === "step_id") stepId = value;
      if (key === "step_type" && (value === "setup" || value === "test" || value === "teardown")) stepType = value;
      if (key === "priority" && (value === "P0" || value === "P1" || value === "P2")) priority = value;
      if (key === "depends_on") {
        dependsOn = value
          .split(",")
          .map((v) => v.trim())
          .filter((v) => v.length > 0);
      }
    }
    entries.push({
      index: i,
      line: start,
      method: current.method,
      target: current.target,
      body: lines.slice(start, end).join("\n").trim(),
      title,
      caseId,
      caseKind,
      stepId,
      stepType,
      priority,
      dependsOn,
    });
  }
  return entries;
}

export function inferEdges(entries: Entry[]): Edge[] {
  const byStepId = new Map<string, number>();
  entries.forEach((entry, idx) => {
    if (entry.stepId) {
      byStepId.set(entry.stepId, idx);
    }
  });
  const edges = new Map<string, Edge>();

  entries.forEach((entry, to) => {
    entry.dependsOn.forEach((dep) => {
      const from = byStepId.get(dep);
      if (from === undefined) {
        return;
      }
      const key = `${from}->${to}`;
      edges.set(key, { from, to, variables: [], explicit: true });
    });
  });

  const capturesByEntry: Array<Set<string>> = entries.map((entry) => captureVariables(entry.body));
  const usedByEntry: Array<Set<string>> = entries.map((entry) => usedVariables(entry.body));
  usedByEntry.forEach((used, to) => {
    used.forEach((variable) => {
      for (let from = to - 1; from >= 0; from -= 1) {
        if (!capturesByEntry[from].has(variable)) {
          continue;
        }
        const key = `${from}->${to}`;
        const existing = edges.get(key);
        if (existing) {
          if (!existing.variables.includes(variable)) {
            existing.variables.push(variable);
          }
        } else {
          edges.set(key, { from, to, variables: [variable], explicit: false });
        }
        break;
      }
    });
  });

  return [...edges.values()].sort((a, b) => (a.from - b.from) || (a.to - b.to));
}

function captureVariables(body: string): Set<string> {
  const lines = body.split(/\r?\n/);
  let inCaptures = false;
  const out = new Set<string>();
  for (const line of lines) {
    const trimmed = line.trim();
    const section = trimmed.match(SECTION_RE)?.[1];
    if (section) {
      inCaptures = section === "Captures";
      continue;
    }
    if (!inCaptures || !trimmed || trimmed.startsWith("#")) {
      continue;
    }
    const pos = trimmed.indexOf(":");
    if (pos <= 0) {
      continue;
    }
    out.add(trimmed.slice(0, pos).trim());
  }
  return out;
}

function usedVariables(body: string): Set<string> {
  const out = new Set<string>();
  for (const match of body.matchAll(VAR_RE)) {
    const variable = match[1].trim();
    if (variable) {
      out.add(variable);
    }
  }
  return out;
}
