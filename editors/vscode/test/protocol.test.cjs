const assert = require("node:assert/strict");
const test = require("node:test");
const { isRunResult, isRunTaskUpdate, isCurlResult } = require("../out/protocol.js");

const run = { uri: "file:///a.hurl", documentVersion: 1, entryLine: 0, target: "entry", success: true, startedAt: "2026-09-05T00:00:00Z", exchanges: [], failedAssertions: [], stdout: "", stderr: "" };
test("validates run result payloads", () => { assert.equal(isRunResult(run), true); assert.equal(isRunResult({ ...run, uri: undefined }), false); });
test("validates run task lifecycle payloads", () => {
  const task = { taskId:"task-1", uri:"file:///a.hurl", documentVersion:1, entryLine:4, target:"entry", state:"running", startedAt:"x", elapsedMs:25 };
  assert.equal(isRunTaskUpdate(task), true);
  assert.equal(isRunTaskUpdate({ ...task, state:"unknown" }), false);
  assert.equal(isRunTaskUpdate({ ...task, taskId:undefined }), false);
});
test("requires a command for successful curl payloads", () => {
  const base = { uri: "file:///a.hurl", documentVersion: 1, entryLine: 0, ok: true, unresolvedVariables: [] };
  assert.equal(isCurlResult(base), false); assert.equal(isCurlResult({ ...base, command: "curl x" }), true);
});
