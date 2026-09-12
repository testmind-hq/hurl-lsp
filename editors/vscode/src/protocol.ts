export type HeaderField = { name: string; value: string; sensitive: boolean };
export type BodyContent = { text?: string; mediaType?: string; encoding: "utf8" | "binary"; originalBytes: number; truncated: boolean };
export type HttpExchange = {
  request: { method: string; url: string; headers: HeaderField[]; body?: BodyContent };
  response?: { version?: string; status?: number; headers: HeaderField[]; body?: BodyContent };
  durationMs?: number;
  timings?: { dnsMs: number; tcpMs: number; tlsMs: number; ttfbMs: number; downloadMs: number; totalMs: number };
};
export type RunTaskState = "queued" | "running" | "cancelling" | "succeeded" | "failed" | "cancelled" | "timedOut";
export type RunTaskUpdate = {
  taskId: string; uri: string; documentVersion: number; entryLine: number; target: "entry" | "chain" | "file";
  state: RunTaskState; startedAt: string; elapsedMs: number; profileName?: string; message?: string;
  stdout?: string; stderr?: string;
};
export type RunPhaseTimings = { prepareMs: number; processMs: number; reportMs: number; totalMs: number };
export type RunResult = {
  taskId?: string;
  uri: string; documentVersion: number; entryLine: number; target: "entry" | "chain" | "file";
  success: boolean; exitCode?: number; startedAt: string; durationMs?: number;
  profileName?: string; profileSources?: string[]; phaseTimings?: RunPhaseTimings;
  exchanges: HttpExchange[]; failedAssertions: Array<{ message: string; line?: number }>;
  stdout: string; stderr: string; parseWarning?: string;
};
export type CurlResult = {
  uri: string; documentVersion: number; entryLine: number; ok: boolean;
  command?: string; displayCommand?: string; unresolvedVariables: string[]; error?: string; copyToClipboard?: boolean;
};

const record = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null;

export function isRunResult(value: unknown): value is RunResult {
  if (!record(value)) return false;
  return typeof value.uri === "string" && typeof value.documentVersion === "number" &&
    typeof value.entryLine === "number" && ["entry", "chain", "file"].includes(String(value.target)) &&
    typeof value.success === "boolean" && typeof value.startedAt === "string" &&
    Array.isArray(value.exchanges) && Array.isArray(value.failedAssertions) &&
    typeof value.stdout === "string" && typeof value.stderr === "string";
}

export function isRunTaskUpdate(value: unknown): value is RunTaskUpdate {
  if (!record(value)) return false;
  return typeof value.taskId === "string" && typeof value.uri === "string" &&
    typeof value.documentVersion === "number" && typeof value.entryLine === "number" &&
    ["entry", "chain", "file"].includes(String(value.target)) &&
    ["queued", "running", "cancelling", "succeeded", "failed", "cancelled", "timedOut"].includes(String(value.state)) &&
    typeof value.startedAt === "string" && typeof value.elapsedMs === "number";
}

export function isCurlResult(value: unknown): value is CurlResult {
  if (!record(value)) return false;
  const base = typeof value.uri === "string" && typeof value.documentVersion === "number" &&
    typeof value.entryLine === "number" && typeof value.ok === "boolean" && Array.isArray(value.unresolvedVariables);
  return base && (!value.ok || typeof value.command === "string");
}
