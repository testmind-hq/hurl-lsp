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
test("renders copy response action for text bodies", () => {
  const store = new InspectorStore(); store.pushRun({ uri:"file:///a",documentVersion:1,entryLine:0,target:"entry",success:true,startedAt:"x",exchanges:[{request:{method:"GET",url:"https://example.com",headers:[]},response:{status:200,headers:[],body:{text:"hello",encoding:"utf8",originalBytes:5,truncated:false}}}],failedAssertions:[],stdout:"",stderr:"" });
  const html = renderInspectorHtml({ cspSource:"vscode" }, undefined, store.snapshot());
  assert.ok(html.includes('data-type="copy-response"'));
  assert.ok(html.includes('data-exchange="0"'));
});
test("renders copy request action for text bodies", () => {
  const store = new InspectorStore(); store.pushRun({ uri:"file:///a",documentVersion:1,entryLine:0,target:"entry",success:true,startedAt:"x",exchanges:[{request:{method:"POST",url:"https://example.com",headers:[],body:{text:"hello",encoding:"utf8",originalBytes:5,truncated:false}}}],failedAssertions:[],stdout:"",stderr:"" });
  const html = renderInspectorHtml({ cspSource:"vscode" }, undefined, store.snapshot());
  assert.ok(html.includes('data-type="copy-request"'));
  assert.ok(html.includes('data-exchange="0"'));
});
test("labels run and HTTP timing breakdown explicitly", () => {
  const store = new InspectorStore(); store.pushRun({ uri:"file:///a",documentVersion:1,entryLine:0,target:"entry",success:true,startedAt:"x",durationMs:42,exchanges:[{durationMs:38,timings:{dnsMs:1,tcpMs:2,tlsMs:5,ttfbMs:4,downloadMs:3,totalMs:15},request:{method:"GET",url:"https://example.com",headers:[]}}],failedAssertions:[],stdout:"",stderr:"" });
  const html = renderInspectorHtml({ cspSource:"vscode" }, undefined, store.snapshot());
  assert.ok(html.includes("Run total: 42 ms"));
  assert.ok(html.includes("HTTP total: 38 ms"));
  assert.ok(html.includes("DNS 1 ms"));
  assert.ok(html.includes("TCP 2 ms"));
  assert.ok(html.includes("TLS 5 ms"));
  assert.ok(html.includes("TTFB 4 ms"));
  assert.ok(html.includes("Download 3 ms"));
  assert.ok(html.includes("Total 15 ms"));
});
test("renders an independent curl preview action", () => {
  const store = new InspectorStore();
  const model = { uri:"file:///a.hurl",fileName:"a.hurl",version:1,selectedIndex:0,entries:[{line:4,method:"GET",target:"/",body:"GET /"}],edges:[] };
  store.selectDocument(model.uri, model.version); store.select("curl");
  const html = renderInspectorHtml({ cspSource:"vscode" }, model, store.snapshot());
  assert.ok(html.includes('data-type="preview-curl"'));
  assert.ok(html.includes('data-line="4"'));
});
