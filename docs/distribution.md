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

Formula path:

- `packaging/homebrew/Formula/hurl-lsp.rb`

Update formula for a new tag:

1. Download `SHA256SUMS` from the release.
2. Run:

```sh
./scripts/update-homebrew-formula.sh <version> <path-to-SHA256SUMS>
```

3. Commit updated formula.
4. Sync formula into `testmind-hq/homebrew-tap` repository.
