const assert = require("node:assert/strict");
const test = require("node:test");
const { escapeHtml, formatBody, renderInspectorHtml } = require("../out/inspectorRender.js");
const { InspectorStore } = require("../out/inspectorStore.js");

test("escapes untrusted content and formats json", () => {
  assert.equal(escapeHtml("<script>"), "&lt;script&gt;");
  assert.equal(formatBody({ text: '{\"x\":1}', mediaType: "application/json", encoding: "utf8", originalBytes: 7, truncated: false }), '{\n  "x": 1\n}');
});
test("renders sensitive headers masked", () => {
  const store = new InspectorStore(); store.pushRun({ uri:"file:///a",documentVersion:1,entryLine:0,target:"entry",success:true,startedAt:"x",exchanges:[{request:{method:"GET",url:"<script>alert(1)</script>",headers:[{name:"Authorization",value:"Bearer secret",sensitive:true}]}}],failedAssertions:[],stdout:"",stderr:"" });
  const html = renderInspectorHtml({ cspSource:"vscode" }, undefined, store.snapshot());
  assert.ok(html.includes("••••••")); assert.ok(!html.includes("Bearer secret")); assert.ok(!html.includes("<script>alert(1)</script>"));
});
