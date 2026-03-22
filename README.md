<div align="center">

# hurl-lsp

**Language Server Protocol implementation for [Hurl](https://hurl.dev)**

Bringing first-class editor intelligence to `.hurl` files —
diagnostics, completion, formatting, outline, and more.

[![WIP](https://img.shields.io/badge/status-WIP-orange?style=flat-square)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![LSP](https://img.shields.io/badge/protocol-LSP%203.17-blueviolet?style=flat-square)]()

</div>

---

> ⚠️ **Work in Progress** — `hurl-lsp` is under active development. Core features are being implemented. Not yet recommended for production use. Feedback and contributions are welcome.

---

## Overview

[Hurl](https://hurl.dev) is a powerful tool for running HTTP requests defined in plain text `.hurl` files. These files are version-controlled alongside your code, making them ideal for API testing in CI/CD pipelines.

Until now, editing `.hurl` files in any editor meant writing in the dark — no completions, no diagnostics, no inline docs. **`hurl-lsp`** fills that gap.

Built in Rust on top of [`tower-lsp`](https://github.com/ebkalderon/tower-lsp) and the official [`hurl_core`](https://crates.io/crates/hurl_core) parser, `hurl-lsp` is currently focused on a solid v0 baseline: core LSP features plus a working VSCode extension.

Current implementation status:

- Rust language server with diagnostics, completions, hover, formatting, outline, and variable definition jump
- VSCode extension with `.hurl` language registration, TextMate grammar, snippets, and language client
- macOS binary download flow inside the VSCode extension

Not implemented yet:

- Rich per-assert actual-value inline rendering
- VSCode webview panels
- Zed / Helix extensions
- TestMind integration

---

## Features

### Diagnostics

Real-time diagnostics while editing `.hurl` files:

- Parser-backed syntax errors from `hurl_core`
- Invalid HTTP method detection
- Invalid section name detection
- Undefined variable warnings (`{{variable}}`)
- Duplicate section warnings in one request block
- Invalid `HTTP` status code format detection

### Completion

Context-aware completions triggered automatically:

- **HTTP Methods** — `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, ...
- **Section Keywords** — `[Query]`, `[Form]`, `[Multipart]`, `[Headers]`, `[Captures]`, `[Asserts]`, `[Options]`, ...
- **Assert Functions** — `jsonpath`, `xpath`, `regex`, `header`, `status`, `duration`, ...
- **Content-Type values** — common MIME types for request headers
- **Captured Variables** — `{{` context completion from same-file `[Captures]`

### Hover Documentation

Hover over methods, sections, and assert functions to see short inline docs.

### Go To Definition

Variable references can jump to their same-file `[Captures]` definition.
When workspace variable files exist (`.hurl-vars`, `vars.env`, `hurl.env`, `.env`), definition and diagnostics can also resolve against those files.

### Code Lens

Per-request Code Lens is available with:
- summary line (`method/path`, section counters)
- `▶ Run` action (executes selected request entry via temporary hurl file)
- `⚡ Run with vars` action (uses nearest `.hurl-vars` / `vars.env` / `hurl.env` / `.env` when found)
- `📋 Copy as curl` action (returns generated curl text from request line + headers)

### OpenAPI Path Completion

When `openapi.yaml` / `openapi.yml` / `swagger.yaml` / `swagger.yml` / `swagger.json` is present in the workspace hierarchy,
request lines can use OpenAPI `paths` keys for URL completion.
For mapped operations, request body field names from OpenAPI `requestBody.content.application/json.schema.properties`
are also suggested inside JSON request bodies.

### Built-in Formatter

`Format Document` is backed by official `hurlfmt::format::format_text(..., false)` through LSP.

### Document Outline

The server exposes metadata-first document symbols with request-level fallback:

```
📁 users.hurl
├── 🔗 TC-CHAIN-001 用户完整生命周期 [P0]
│   ├── 🔧 Create user step-setup
│   └── 🧪 Validate user step-test
├── 🟧 P1
│   └── TC-0042 Invalid email
└── ○ GET /health
```

## Editor Support

| Editor | Status | Notes |
|--------|--------|-------|
| **VSCode** | Implemented | Extension included in `editors/vscode`, with local binary config and macOS auto-download flow |
| **Helix** | Manual setup | Server is usable manually once binary is installed |
| **Neovim** | Manual setup | Server is usable manually via LSP config |
| **Zed** | Planned | No extension implementation yet |

## Platform Support

Current automatic binary management in the VSCode extension supports:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

Linux and Windows release artifacts are planned, but not part of the current v0 baseline.

---

## Installation

### VSCode

Install the **Hurl** extension from the VS Marketplace. The extension currently auto-downloads binaries on macOS only.

```
ext install yuchou87.vscode-hurl
```

### Zed

Install the **Hurl** extension from Zed Extensions, then install the binary:

```sh
cargo install hurl-lsp
```

### Helix

Install the binary, then add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "hurl"
scope = "source.hurl"
file-types = ["hurl"]
comment-token = "#"
language-servers = ["hurl-lsp"]

[language-server.hurl-lsp]
command = "hurl-lsp"
```

### Neovim

```lua
require('lspconfig').hurl_lsp.setup({
  cmd = { "hurl-lsp" },
  filetypes = { "hurl" },
})
```

---

## Binary Installation

**Via cargo:**

```sh
cargo install hurl-lsp
```

**Via Homebrew** _(coming soon)_:

```sh
brew install hurl-lsp
```

Pre-built binaries for macOS (Intel + Apple Silicon) are available on the [Releases](https://github.com/yuchou87/hurl-lsp/releases) page.

---

## TestMind Integration _(planned)_

CI result feedback integration from TestMind is a future phase and is not implemented in current builds.

---

## Roadmap

### Phase 1 — Core LSP _(baseline delivered)_

- [x] Project scaffolding (Cargo workspace + tower-lsp)
- [x] Document state management (`did_open` / `did_change`)
- [x] Syntax diagnostics via `hurl_core`
- [x] HTTP method + section keyword completion
- [x] Assert function completion
- [x] Hover documentation
- [x] Built-in formatter (`hurlfmt`)
- [x] CI (build + test + clippy)

### Phase 2 — Editor Extensions + Distribution

- [ ] Multi-platform cross-compilation + GitHub Releases
- [ ] VSCode extension (syntax highlight + snippets + LSP client + auto binary management)
- [ ] Publish to VS Marketplace
- [ ] Zed extension (syntax highlight + LSP client)
- [ ] Publish to Zed Extensions
- [ ] Helix configuration docs + upstream PR to `languages.toml`
- [ ] Publish to crates.io

### Phase 3 — Differentiating Features

- [x] Variable file integration (workspace env files + cross-file resolution)
- [x] Code Lens — run/run-with-vars/copy-as-curl + dependency annotation + last-run status summary
- [x] Inline execution result display (run failures map to assert-line diagnostics with persisted summary state)
- [x] OpenAPI / Swagger integration (path + request-body-field completion + response assert skeleton completion)
- [x] Document outline with metadata support (`documentSymbol`)
- [x] Chain case detection and dependency annotation

### Phase 4 — Ecosystem

- [ ] VSCode Webview panel (single entry view + chain flow graph)
- [ ] Markdown export command
- [ ] Homebrew distribution
- [ ] Upstream PR to Hurl official docs (editor support page)
- [ ] TestMind CI result feedback integration

---

## Architecture

`hurl-lsp` is built in **Rust** using:

| Crate | Role |
|-------|------|
| [`tower-lsp`](https://github.com/ebkalderon/tower-lsp) | Async LSP server framework |
| [`hurl_core`](https://crates.io/crates/hurl_core) | Official Hurl parser and AST |
| [`tokio`](https://tokio.rs/) | Async runtime |
| [`dashmap`](https://crates.io/crates/dashmap) | Concurrent document state store |

The server communicates via **stdin/stdout** JSON-RPC, launched as a subprocess by the editor. No daemon, no ports, no configuration required to get started.

---

## Repository Structure

```
hurl-lsp/
├── crates/
│   └── hurl-lsp/              # LSP server core (Rust)
│       └── src/
│           ├── main.rs
│           ├── backend.rs
│           ├── diagnostics.rs
│           ├── completion.rs
│           ├── hover.rs
│           ├── formatting.rs
│           ├── symbols.rs
│           ├── metadata.rs
│           └── definition.rs
├── editors/
│   └── vscode/                # VSCode extension (TypeScript)
└── .github/
    └── workflows/
        ├── ci.yml
        └── release.yml
```

---

## Contributing

Contributions are welcome. If you plan to work on a significant feature, please open an issue first to discuss the approach.

```sh
git clone https://github.com/yuchou87/hurl-lsp
cd hurl-lsp
cargo build
cargo test
cargo clippy
```

---

## Related Projects

| Project | Description |
|---------|-------------|
| [Hurl](https://hurl.dev) | The HTTP testing tool this LSP supports |
| [CaseForge](https://github.com/testmind-hq/caseforge) | AI-powered test case generation for Hurl |
| [TestMind](https://github.com/testmind-hq) | Test case management and CI integration platform |
| [taplo](https://taplo.tamasfe.dev/) | Inspiration — a well-crafted LSP for TOML in Rust |
| [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml) | Reference for VSCode extension + binary management pattern |

---

## License

MIT © [yuchou87](https://github.com/yuchou87)
