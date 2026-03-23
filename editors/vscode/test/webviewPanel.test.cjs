const assert = require("node:assert/strict");
const test = require("node:test");

const { inferEdges, parseEntries, pickSelectedEntry } = require("../out/webviewModel.js");

test("parseEntries extracts metadata per request entry", () => {
  const source = [
    "# title=Get token",
    "# case_id=auth.login",
    "# case_kind=chain",
    "# step_id=step_get_token",
    "# step_type=setup",
    "# priority=P0",
    "GET https://example.org",
    "HTTP 200",
    "",
    "# title=Submit login",
    "# depends_on=step_get_token",
    "POST https://example.org/login",
    "HTTP 302",
  ].join("\n");

  const entries = parseEntries(source);
  assert.equal(entries.length, 2);
  assert.equal(entries[0].title, "Get token");
  assert.equal(entries[0].caseId, "auth.login");
  assert.equal(entries[0].caseKind, "chain");
  assert.equal(entries[0].stepId, "step_get_token");
  assert.equal(entries[0].stepType, "setup");
  assert.equal(entries[0].priority, "P0");
  assert.deepEqual(entries[1].dependsOn, ["step_get_token"]);
});

test("inferEdges merges explicit depends_on and inferred variable dependency", () => {
  const source = [
    "# step_id=step_get_token",
    "GET https://example.org",
    "HTTP 200",
    "[Captures]",
    "csrf_token: xpath \"string(//meta[@name='_csrf_token']/@content)\"",
    "",
    "# step_id=step_login",
    "# depends_on=step_get_token",
    "POST https://example.org/login",
    "[Form]",
    "token: {{csrf_token}}",
    "HTTP 302",
  ].join("\n");

  const entries = parseEntries(source);
  const edges = inferEdges(entries);
  assert.equal(edges.length, 1);
  assert.equal(edges[0].from, 0);
  assert.equal(edges[0].to, 1);
  assert.equal(edges[0].explicit, true);
  assert.deepEqual(edges[0].variables, ["csrf_token"]);
});

test("pickSelectedEntry returns nearest previous request by cursor line", () => {
  const source = [
    "GET https://example.org/1",
    "HTTP 200",
    "",
    "POST https://example.org/2",
    "HTTP 201",
  ].join("\n");
  const entries = parseEntries(source);
  assert.equal(pickSelectedEntry(entries, 0), 0);
  assert.equal(pickSelectedEntry(entries, 2), 0);
  assert.equal(pickSelectedEntry(entries, 3), 1);
  assert.equal(pickSelectedEntry(entries, 20), 1);
});
