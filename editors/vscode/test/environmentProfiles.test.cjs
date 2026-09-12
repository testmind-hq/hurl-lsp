const assert = require("node:assert/strict");
const test = require("node:test");
const { AUTO_PROFILE, normalizeProfiles, resolveActiveProfile } = require("../out/environmentProfileModel.js");

test("resolves selections independently for multiple workspace folders", () => {
  const profiles = { Local:["vars.env", "vars.local.env"], Staging:["vars.staging.env"] };
  const selections = { "file:///a":"Local", "file:///b":"Staging" };
  assert.deepEqual(resolveActiveProfile("file:///a", selections, AUTO_PROFILE, profiles), { name:"Local", files:["vars.env", "vars.local.env"] });
  assert.deepEqual(resolveActiveProfile("file:///b", selections, AUTO_PROFILE, profiles), { name:"Staging", files:["vars.staging.env"] });
  assert.deepEqual(resolveActiveProfile("file:///c", selections, AUTO_PROFILE, profiles), { name:AUTO_PROFILE, files:[] });
});

test("falls back to Auto for invalid configuration and stale selection", () => {
  assert.deepEqual(normalizeProfiles({ Good:["vars.env"], Bad:[1], EmptyName:[""] }), { Good:["vars.env"] });
  assert.deepEqual(resolveActiveProfile("file:///a", { "file:///a":"Missing" }, "Missing", { Local:["local.env"] }), { name:AUTO_PROFILE, files:[] });
});
