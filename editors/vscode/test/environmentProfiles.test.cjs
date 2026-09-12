const assert = require("node:assert/strict");
const test = require("node:test");
const {
  AUTO_PROFILE,
  discoverConventionProfiles,
  mergeEnvironmentProfiles,
  normalizeProfiles,
  profileWatchPaths,
  resolveActiveProfile,
} = require("../out/environmentProfileModel.js");

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

test("watches arbitrary configured profile files once per workspace", () => {
  assert.deepEqual(profileWatchPaths({
    Local:["vars.env", "config/local.env"],
    Staging:["staging.env", "vars.env"],
    Invalid:["../outside.env", "/tmp/absolute.env", "C:\\absolute.env"],
  }), ["vars.env", "config/local.env", "staging.env"]);
});

test("discovers node-style environment profiles in override order", () => {
  assert.deepEqual(discoverConventionProfiles([
    ".env.production.local",
    ".env",
    ".env.local",
    ".env.staging",
    ".env.staging.local",
    "vars.env",
  ]), {
    production:[".env", ".env.local", ".env.production.local"],
    staging:[".env", ".env.local", ".env.staging", ".env.staging.local"],
  });
});

test("configured profiles override convention profiles with the same name", () => {
  assert.deepEqual(mergeEnvironmentProfiles(
    { staging:[".env", ".env.staging"], production:[".env.production"] },
    { staging:["config/staging.env"] },
  ), {
    staging:["config/staging.env"],
    production:[".env.production"],
  });
});
