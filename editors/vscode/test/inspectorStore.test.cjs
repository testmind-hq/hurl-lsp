const assert = require("node:assert/strict");
const test = require("node:test");
const { InspectorStore } = require("../out/inspectorStore.js");

const run = (entryLine) => ({ uri: "file:///a.hurl", documentVersion: 1, entryLine, target: "entry", success: true, startedAt: String(entryLine), exchanges: [], failedAssertions: [], stdout: "", stderr: "" });
test("keeps the newest ten in-memory results", () => {
  const store = new InspectorStore(); for (let i = 0; i < 12; i += 1) store.pushRun(run(i));
  const value = store.snapshot(); assert.equal(value.runs.length, 10); assert.equal(value.runs[0].entryLine, 2); assert.equal(value.selectedRun, 9); assert.equal(value.tab, "result");
});
test("selects curl tab", () => { const store = new InspectorStore(); store.setCurl({ uri: "file:///a", documentVersion: 1, entryLine: 0, ok: false, unresolvedVariables: ["x"] }); assert.equal(store.snapshot().tab, "curl"); });
test("partitions results by source uri and document version", () => {
  const store = new InspectorStore();
  store.pushRun(run(1));
  store.pushRun({ ...run(2), uri: "file:///b.hurl" });
  assert.deepEqual(store.snapshot().runs.map((value) => value.uri), ["file:///b.hurl"]);
  store.selectDocument("file:///a.hurl", 1);
  assert.deepEqual(store.snapshot().runs.map((value) => value.entryLine), [1]);
  store.selectDocument("file:///a.hurl", 2);
  assert.equal(store.snapshot().runs.length, 0);
});
test("limits run history across document versions", () => {
  const store = new InspectorStore();
  for (let version = 1; version <= 12; version += 1) store.pushRun({ ...run(version), documentVersion: version });
  store.selectDocument("file:///a.hurl", 3);
  assert.deepEqual(store.snapshot().runs.map((value) => value.documentVersion), [3]);
  store.selectDocument("file:///a.hurl", 1);
  assert.equal(store.snapshot().runs.length, 0);
  store.selectDocument("file:///a.hurl", 12);
  assert.deepEqual(store.snapshot().runs.map((value) => value.documentVersion), [12]);
});
