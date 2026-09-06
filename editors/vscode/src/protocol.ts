export type HeaderField = { name: string; value: string; sensitive: boolean };
export type BodyContent = { text?: string; mediaType?: string; encoding: "utf8" | "binary"; originalBytes: number; truncated: boolean };
export type HttpExchange = {
  request: { method: string; url: string; headers: HeaderField[]; body?: BodyContent };
  response?: { version?: string; status?: number; headers: HeaderField[]; body?: BodyContent };
  durationMs?: number;
};
export type RunResult = {
  uri: string; documentVersion: number; entryLine: number; target: "entry" | "chain" | "file";
  success: boolean; exitCode?: number; startedAt: string; durationMs?: number;
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

export function isCurlResult(value: unknown): value is CurlResult {
  if (!record(value)) return false;
  const base = typeof value.uri === "string" && typeof value.documentVersion === "number" &&
    typeof value.entryLine === "number" && typeof value.ok === "boolean" && Array.isArray(value.unresolvedVariables);
  return base && (!value.ok || typeof value.command === "string");
}
