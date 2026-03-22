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

- Rust language server with diagnostics, completions, hover, formatting, and request-level outline
- VSCode extension with `.hurl` language registration, TextMate grammar, snippets, and language client
- macOS binary download flow inside the VSCode extension

Not implemented yet:

- Variable file integration
- Code Lens actions and request execution
- Metadata-aware outline
- Inline execution results
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

### Completion

Context-aware completions triggered automatically:

- **HTTP Methods** — `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, ...
- **Section Keywords** — `[QueryStringParameters]`, `[Headers]`, `[Captures]`, `[Asserts]`, `[Options]`, ...
- **Assert Functions** — `jsonpath`, `xpath`, `regex`, `header`, `status`, `duration`, ...
- **Content-Type values** — common MIME types for request headers

### Hover Documentation

Hover over methods, sections, and assert functions to see short inline docs.

### Built-in Formatter

`Format Document` is supported through the language server. In the current v0 implementation this is a lightweight text normalization pass, not full `hurlfmt` parity yet.

### Document Outline

The server exposes request-level document symbols so editors can show a simple outline:

```
📁 users.hurl
├── GET https://example.com/health
└── POST /users
```

## Editor Support

| Editor | Status | Notes |
|--------|--------|-------|
| **VSCode** | Implemented | Extension included in `editors/vscode`, with local binary config and macOS auto-download flow |
| **Helix** | Planned | Server should be usable manually once binary is installed |
| **Neovim** | Planned | Server should be usable manually via LSP config |
| **Zed** | Planned | No extension implementation yet |

## Platform Support

Current automatic binary management in the VSCode extension supports:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

Linux and Windows release artifacts are planned, but not part of the current v0 baseline.

---

## Installation

### VSCode

Install the **Hurl** extension from the VS Marketplace. The extension automatically downloads and manages the correct binary for your platform — no manual steps required.

```
ext install testmind-hq.hurl
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

Pre-built binaries for Linux (`x86_64`, `musl`), macOS (Intel + Apple Silicon), and Windows are available on the [Releases](https://github.com/testmind-hq/hurl-lsp/releases) page.

---

## TestMind Integration _(optional)_

When configured, `hurl-lsp` pulls CI execution results from [TestMind](https://github.com/testmind-hq) and displays them as inline diagnostics — without opening the CI platform.

Add `.hurl-lsp.toml` to your project root:

```toml
[testmind]
endpoint      = "https://your-testmind-instance.com"
token         = "tm_xxxxxxxxxxxx"
branch        = "main"   # CI branch to pull results from
poll_interval = 60       # seconds; 0 = fetch once on file open
```

This integration is **entirely optional**. `hurl-lsp` is fully functional without it.

---

## Roadmap

### Phase 1 — Core LSP _(in progress)_

- [ ] Project scaffolding (Cargo workspace + tower-lsp)
- [ ] Document state management (`did_open` / `did_change`)
- [ ] Syntax diagnostics via `hurl_core`
- [ ] HTTP method + section keyword completion
- [ ] Assert function completion
- [ ] Hover documentation
- [ ] Built-in formatter
- [ ] CI (build + test + clippy)

### Phase 2 — Editor Extensions + Distribution

- [ ] Multi-platform cross-compilation + GitHub Releases
- [ ] VSCode extension (syntax highlight + snippets + LSP client + auto binary management)
- [ ] Publish to VS Marketplace
- [ ] Zed extension (syntax highlight + LSP client)
- [ ] Publish to Zed Extensions
- [ ] Helix configuration docs + upstream PR to `languages.toml`
- [ ] Publish to crates.io

### Phase 3 — Differentiating Features

- [ ] Variable file integration (completion + Go to Definition + undefined warnings)
- [ ] Code Lens — run, run with vars, copy as curl
- [ ] Inline execution result display
- [ ] Document outline with metadata support (`documentSymbol`)
- [ ] Chain case detection and dependency annotation
- [ ] OpenAPI / Swagger integration (URL + body completion, auto-generate asserts)

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
│           ├── backend.rs     # LanguageServer trait impl
│           ├── diagnostics.rs
│           ├── completion.rs
│           ├── hover.rs
│           ├── formatting.rs
│           ├── code_lens.rs
│           ├── definition.rs
│           ├── variables.rs
│           ├── outline.rs     # documentSymbol + metadata parser
│           └── openapi.rs
├── editors/
│   ├── vscode/                # VSCode extension (TypeScript)
│   ├── zed/                   # Zed extension (Rust → WASM)
│   └── helix/                 # Configuration docs
└── .github/
    └── workflows/
        ├── ci.yml
        ├── release.yml        # Multi-platform cross-compilation
        └── publish.yml        # VS Marketplace + Zed Extensions
```

---

## Contributing

Contributions are welcome. If you plan to work on a significant feature, please open an issue first to discuss the approach.

```sh
git clone https://github.com/testmind-hq/hurl-lsp
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

MIT © [TestMind HQ](https://github.com/testmind-hq)
