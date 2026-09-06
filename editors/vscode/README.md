# Hurl LSP for VS Code

Language support for [Hurl](https://hurl.dev) `.hurl` files, powered by [hurl-lsp](https://github.com/testmind-hq/hurl-lsp).

![Hurl LSP](media/icon.png)

The extension registers the `hurl` language, starts a local language server, and auto-downloads a matching `hurl-lsp` binary when needed.

This project is under active development. Feedback and issues are welcome at [testmind-hq/hurl-lsp](https://github.com/testmind-hq/hurl-lsp/issues).

## Installation

### 1. Install this extension

- From VS Marketplace: search **Hurl LSP** (`testmind-hq.vscode-hurl`)
- Or install a `.vsix` from [GitHub Releases](https://github.com/testmind-hq/hurl-lsp/releases)

Open any `.hurl` file. The extension starts `hurl-lsp` automatically.

### 2. Install the `hurl-lsp` language server

Most users can skip this. On first activation the extension downloads a matching binary from GitHub Releases for macOS, Linux x64, and Windows x64.

To install the server yourself:

```sh
# Cargo
cargo install hurl-lsp

# Homebrew
brew tap testmind-hq/tap
brew install hurl-lsp
```

Pre-built archives: [Releases](https://github.com/testmind-hq/hurl-lsp/releases).

If the binary is not on `PATH`, set `hurl.server.path` to its absolute path. A `PATH` binary is reused only when its version is at least this extension's version.

### 3. Optional: install the `hurl` CLI

CodeLens **Run** / **Run chain** / **Run file** need [`hurl`](https://hurl.dev/docs/installation.html) on `PATH`. Completions, diagnostics, hover, and formatting work without it.

## Features

- **Syntax highlighting** and snippets for `.hurl` files
- **Diagnostics** from the official `hurl_core` parser, plus checks for methods, sections, status codes, and undefined `{{variables}}`
- **Completions** for HTTP methods, section names, assert functions, Content-Type values, captured variables, and OpenAPI paths
- **Hover** docs on methods, sections, and assert functions
- **Go to Definition** for `{{variables}}` (same-file `[Captures]` and workspace variable files)
- **Inlay hints** for resolved variable values, with secret masking
- **Format Document** via official `hurlfmt`
- **CodeLens** to Run, Run with vars, Run chain, Run file, and Copy as cURL
- **Hurl Requests** view in Explorer, with inline Run / Run Chain
- **Inspector** side panel for request, chain, result, and cURL preview
- **Export as Markdown** for the current `.hurl` file

## Language server lookup

When the extension starts, it picks a binary in this order:

1. `hurl.server.path` if set
2. `hurl-lsp` on `PATH` when its version is at least this extension's version
3. Auto-download from GitHub Releases

Auto-download targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`.

Use **Hurl: Show Log** if the server fails to start.

Workspace variable files: `.hurl-vars`, `vars.env`, `hurl.env`, `.env`.

OpenAPI path completions need `openapi.yaml` / `swagger.json` in the workspace.

## Commands

| Command | What it does |
|---|---|
| `Hurl: Open Inspector` | Open the request inspector |
| `Hurl: Format Document` | Format the current `.hurl` file |
| `Hurl: Export as Markdown` | Write a sibling `.md` file |
| `Hurl: Restart Language Server` | Restart `hurl-lsp` |
| `Hurl: Show Log` | Open the runtime log |
| `Hurl: Show Request Log` | Open the request log |
| `Hurl: Clear Run Alerts` | Clear inline run-failure diagnostics |

## Settings

| Setting | Default | What it does |
|---|---|---|
| `hurl.server.path` | `""` | Absolute path to a local `hurl-lsp` binary |
| `hurl.server.trace` | `off` | LSP trace: `off` / `messages` / `verbose` |
| `hurl.run.verbosity` | `verbose` | Run output verbosity |
| `hurl.run.inlineFailureDiagnostics` | `true` | Inline red diagnostics on failed runs |
| `hurl.variables.inlayHints.enabled` | `true` | Show resolved variable values |
| `hurl.variables.inlayHints.maxLength` | `60` | Max characters for a hint |
| `hurl.outline.groupMode` | `hierarchical` | Outline grouping: `hierarchical` / `flat` |
| `hurl.outline.sortMode` | `source` | Outline sort: `source` / `priority` |

## Links

- [Repository](https://github.com/testmind-hq/hurl-lsp)
- [Changelog](https://github.com/testmind-hq/hurl-lsp/blob/main/CHANGELOG.md)
- [Hurl documentation](https://hurl.dev)

License: [MIT](https://github.com/testmind-hq/hurl-lsp/blob/main/LICENSE)
