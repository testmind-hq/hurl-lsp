const assert = require("node:assert/strict");
const test = require("node:test");
const { InspectorStore } = require("../out/inspectorStore.js");

const run = (entryLine) => ({ uri: "file:///a.hurl", documentVersion: 1, entryLine, target: "entry", success: true, startedAt: String(entryLine), exchanges: [], failedAssertions: [], stdout: "", stderr: "" });
test("keeps the newest ten in-memory results", () => {
  const store = new InspectorStore(); for (let i = 0; i < 12; i += 1) store.pushRun(run(i));
  const value = store.snapshot(); assert.equal(value.runs.length, 10); assert.equal(value.runs[0].entryLine, 2); assert.equal(value.selectedRun, 9); assert.equal(value.tab, "result");
});
test("selects curl tab", () => { const store = new InspectorStore(); store.setCurl({ uri: "file:///a", documentVersion: 1, entryLine: 0, ok: false, unresolvedVariables: ["x"] }); assert.equal(store.snapshot().tab, "curl"); });
