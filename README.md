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

Built in Rust on top of [`tower-lsp`](https://github.com/ebkalderon/tower-lsp) and the official [`hurl_core`](https://crates.io/crates/hurl_core) parser, `hurl-lsp` delivers accurate, fast, and extensible editor support for `.hurl` files across all major editors.

`hurl-lsp` is a **standalone tool** — it works independently of any platform and requires no account or backend. Optionally, it integrates with [TestMind](https://github.com/testmind-hq) to surface CI execution results directly in your editor.

---

## Features

### Diagnostics

Real-time error detection as you type — invalid HTTP methods, malformed section names, unknown assert functions, undefined variable references, and more.

<!-- screenshot placeholder -->
> 📸 _Screenshot: Inline diagnostics in VSCode_
> `[assets/screenshots/diagnostics.png]`

---

### Completion

Context-aware completions triggered automatically:

- **HTTP Methods** — `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, ...
- **Section Keywords** — `[QueryStringParameters]`, `[Headers]`, `[Captures]`, `[Asserts]`, `[Options]`, ...
- **Assert Functions** — `jsonpath`, `xpath`, `regex`, `header`, `status`, `duration`, ...
- **Variable References** — `{{` triggers completions from captures and variable files

<!-- screenshot placeholder -->
> 📸 _Screenshot: Section keyword completion_
> `[assets/screenshots/completion.png]`

---

### Hover Documentation

Hover over any keyword to see inline documentation — section descriptions, assert function signatures, and usage examples.

<!-- screenshot placeholder -->
> 📸 _Screenshot: Hover docs on assert function_
> `[assets/screenshots/hover.png]`

---

### Built-in Formatter

Format `.hurl` files directly from your editor via `Format Document`. No need to install `hurlfmt` separately — formatting is built into the server using the official `hurl_core` AST pretty-printer.

---

### Variable File Integration

`hurl-lsp` automatically reads variable definitions from common file conventions:

```
.hurl-vars
vars.env
hurl.env
.env
```

This enables:
- `{{variable}}` completion from known variables
- Go to Definition — jump to where a variable is declared
- Undefined variable warnings

---

### Document Outline

When `.hurl` files include [metadata annotations](#metadata-annotation-format), `hurl-lsp` builds a rich structured outline of all test cases. Entries without annotations are still shown as plain `Method + Path` entries — they are never hidden.

```
📁 users.hurl
│
├── 🔗 TC-CHAIN-001  User lifecycle (create → query → delete)  [P0]
│   ├── 🔧 setup     Create test user          step-setup-user
│   ├── 🧪 test      Query the created user    step-test-get
│   └── 🧹 teardown  Cleanup: delete user      step-teardown
│
├── 🟥 P0
│   ├── TC-0001  POST /users - valid input, normal creation
│   └── TC-0002  POST /users - minimal required fields
│
├── 🟧 P1
│   └── TC-0042  POST /users - invalid email format   [equivalence_partitioning]
│
└── ○ POST /users                        ← no metadata, shows Method + Path only
```

<!-- screenshot placeholder -->
> 📸 _Screenshot: Document outline in VSCode_
> `[assets/screenshots/outline.png]`

---

### Code Lens

Inline action buttons and request summaries appear above every entry:

```
📋 TC-0042  P1  equivalence_partitioning  │ 0 headers  │ 2 asserts
▶ Run    ⚡ Run with vars    📋 Copy as curl
POST {{base_url}}/users
```

For chained entries, dependency annotations are shown with jump-to-source support:

```
🧪 test · TC-CHAIN-001  │  📥 depends on: user_id ← step-setup-user (line 12)
▶ Run    ⚡ Run with vars    📋 Copy as curl
GET {{base_url}}/users/{{user_id}}
```

---

### Inline Execution Results

After running a request, assertion results are displayed inline — no need to switch to a terminal or output panel:

```hurl
[Asserts]
jsonpath "$.code"  == "validation_error"   ✅  actual: "validation_error"
jsonpath "$.field" == "email"              ❌  actual: "username"
duration < 500                             ✅  actual: 230ms
```

<!-- screenshot placeholder -->
> 📸 _Screenshot: Inline execution results_
> `[assets/screenshots/inline-results.png]`

---

### VSCode Webview Panel _(VSCode only)_

A dedicated side panel provides two views:

**Single Entry View** — renders the current entry as a structured card with live execution results.

**Chain Flow View** — renders chained test cases as an interactive dependency graph. Node colors reflect execution state (gray = not run, green = passed, red = failed). Click any node to jump to the corresponding source line.

<!-- screenshot placeholder -->
> 📸 _Screenshot: VSCode chain flow panel_
> `[assets/screenshots/webview-chain.png]`

---

## Metadata Annotation Format

`hurl-lsp` reads structured metadata from `# key=value` comments above each entry. These annotations are produced by [CaseForge](https://github.com/testmind-hq/caseforge) and can also be written by hand.

**Supported keys:**

| Key | Values | Description |
|-----|--------|-------------|
| `case_id` | string | Unique case identifier |
| `case_kind` | `single` \| `chain` | Case type |
| `priority` | `P0` \| `P1` \| `P2` | Priority level |
| `step_id` | string | Step identifier (chain cases) |
| `step_type` | `setup` \| `test` \| `teardown` | Step role (chain cases) |
| `title` | string | Human-readable title for outline and hover display |
| `technique` | string | Testing technique tag (e.g. `equivalence_partitioning`) |
| `depends_on` | comma-separated step_ids | Explicit step dependency declaration |

Comments that do not match a whitelisted key (e.g. `# ═══` dividers, freeform descriptions) are ignored by the parser.

**Single case example:**

```hurl
# case_id=TC-0042
# case_kind=single
# priority=P1
# step_type=test
# technique=equivalence_partitioning
# title=POST /users - invalid email format
POST {{base_url}}/users
Content-Type: application/json
{ "name": "Test User", "email": "not-an-email" }
HTTP 422
[Asserts]
jsonpath "$.code"  == "validation_error"
jsonpath "$.field" == "email"
```

**Chain case example:**

```hurl
# case_id=TC-CHAIN-001
# case_kind=chain
# priority=P0

# step_id=step-setup-user
# step_type=setup
# title=Create test user
POST {{base_url}}/users
Authorization: Bearer {{auth_token}}
{ "name": "Markus Moen", "email": "markus@example.com" }
HTTP 201
[Captures]
user_id: jsonpath "$.id"

# step_id=step-test-get
# step_type=test
# title=Query the created user
# depends_on=step-setup-user
GET {{base_url}}/users/{{user_id}}
Authorization: Bearer {{auth_token}}
HTTP 200

# step_id=step-teardown
# step_type=teardown
# title=Cleanup: delete user
# depends_on=step-test-get
DELETE {{base_url}}/users/{{user_id}}
Authorization: Bearer {{auth_token}}
HTTP 200
```

Metadata annotations are **entirely optional**. All core LSP features work without them.

---

## Editor Support

| Editor | Installation | Syntax Highlight | LSP | Webview Panel |
|--------|-------------|-----------------|-----|---------------|
| **VSCode** | VS Marketplace extension (auto binary management) | ✅ | ✅ | ✅ |
| **Zed** | Zed Extensions + local binary | ✅ Tree-sitter | ✅ | ❌ |
| **Helix** | Manual config / built-in (pending PR) | ✅ Tree-sitter | ✅ | ❌ |
| **Neovim** | nvim-lspconfig | via plugin | ✅ | ❌ |

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

