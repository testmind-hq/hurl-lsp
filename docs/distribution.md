# Distribution Guide

This document tracks how to publish `hurl-lsp` across package channels.

## 1. GitHub Release (binaries + checksums)

Workflow: `.github/workflows/release.yml`

- Triggered by tag push `v*`
- Publishes platform archives to GitHub Release
- Publishes `SHA256SUMS` asset for package managers

## 2. crates.io

Workflow: `.github/workflows/publish-crates-io.yml`

Required secret:

- `CARGO_REGISTRY_TOKEN`

Release steps:

1. Run workflow with `dry_run=true` and ensure it passes.
2. Run workflow with `dry_run=false` to publish `hurl-lsp`.

## 3. VS Code Marketplace and Open VSX

Workflow: `.github/workflows/publish-vscode.yml`

The vsix is built from `editors/vscode/`. Listing copy uses `editors/vscode/README.md`; keep that file in sync with extension features. `LICENSE` must sit next to `package.json` or `vsce` warns.

One vsix is published to both:

- Visual Studio Marketplace (VS Code)
- [Open VSX](https://open-vsx.org) (VSCodium and other Open VSX clients)

Required secrets:

- `VSCE_PAT`
- `OVSX_PAT`

Release steps:

1. Run workflow with `dry_run=true` to produce a `.vsix`.
2. Run workflow with `dry_run=false` to publish both registries.

### One-time Open VSX setup

Namespace must match `package.json` `publisher`: `testmind-hq`.

1. Sign in at [open-vsx.org](https://open-vsx.org) with GitHub.
2. Open [Access Tokens](https://open-vsx.org/user-settings/tokens) and create a token.
3. Create the namespace (once):

```sh
cd editors/vscode
npx ovsx create-namespace testmind-hq -p <token>
```

4. Add the token as repo secret `OVSX_PAT`.
5. Optional: [claim namespace ownership](https://github.com/EclipseFdn/open-vsx.org/wiki/Managing-Namespaces) so the listing shows as verified. GitHub account used for the claim needs at least one year of history.

The first `dry_run=false` run publishes `testmind-hq.vscode-hurl` to `https://open-vsx.org/extension/testmind-hq/vscode-hurl`. `--skip-duplicate` keeps re-runs from failing if that version is already there.

## 4. Zed Extensions

Current status: manual publish pending.

Recommended next step:

1. Fill/verify metadata in `editors/zed/extension.toml`.
2. Publish via Zed extension publishing process from a dedicated release branch.

## 5. Homebrew

Formula is maintained in [testmind-hq/homebrew-tap](https://github.com/testmind-hq/homebrew-tap) and updated automatically by the release workflow when `HOMEBREW_TAP_TOKEN` is configured.

To update the formula manually for a new tag:

1. Download `SHA256SUMS` from the release.
2. Run:

```sh
./scripts/update-homebrew-formula.sh <version> <path-to-SHA256SUMS> [output-path]
# output-path is optional; defaults to /tmp/hurl-lsp.rb if omitted.
```

3. Copy the generated file to `Formula/hurl-lsp.rb` in `testmind-hq/homebrew-tap` and commit.
