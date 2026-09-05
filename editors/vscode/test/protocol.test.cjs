const assert = require("node:assert/strict");
const test = require("node:test");
const { isRunResult, isCurlResult } = require("../out/protocol.js");

const run = { uri: "file:///a.hurl", documentVersion: 1, entryLine: 0, target: "entry", success: true, startedAt: "2026-09-05T00:00:00Z", exchanges: [], failedAssertions: [], stdout: "", stderr: "" };
test("validates run result payloads", () => { assert.equal(isRunResult(run), true); assert.equal(isRunResult({ ...run, uri: undefined }), false); });
test("requires a command for successful curl payloads", () => {
  const base = { uri: "file:///a.hurl", documentVersion: 1, entryLine: 0, ok: true, unresolvedVariables: [] };
  assert.equal(isCurlResult(base), false); assert.equal(isCurlResult({ ...base, command: "curl x" }), true);
});
