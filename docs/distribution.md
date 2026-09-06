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

## 3. VSCode Marketplace

Workflow: `.github/workflows/publish-vscode.yml`

The vsix is built from `editors/vscode/`. Marketplace Details uses `editors/vscode/README.md`; keep that file in sync with extension features. `LICENSE` must sit next to `package.json` or `vsce` warns.

Required secret:

- `VSCE_PAT`

Release steps:

1. Run workflow with `dry_run=true` to produce a `.vsix`.
2. Run workflow with `dry_run=false` to publish.

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
