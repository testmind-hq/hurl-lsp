# Helix Setup

## 1. Install `hurl-lsp`

**Homebrew (macOS / Linux):**

```sh
brew tap testmind-hq/tap
brew install hurl-lsp
```

**Cargo:**

```sh
cargo install hurl-lsp
```

Verify the installation:

```sh
hurl-lsp --version
```

## 2. Configure Helix

Add the following to `~/.config/helix/languages.toml`:

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

Reopen Helix — the language server starts automatically when you open a `.hurl` file.

## 3. Features

| Feature | Supported |
|---------|-----------|
| Syntax diagnostics | ✅ |
| Keyword completion (methods, sections, assertions) | ✅ |
| Hover documentation | ✅ |
| Document formatting (`hurlfmt`) | ✅ |
| Variable file tracking (`{{var}}` completion + Go to Definition) | ✅ |
| Code Lens (Run / Run with vars / Copy as curl) | ✅ |
| Document outline (`documentSymbol`) | ✅ |
| OpenAPI / Swagger path and field completion | ✅ |

## 4. Troubleshooting

Run `hx --health hurl` and verify `hurl-lsp` appears under language servers.

If Helix cannot find the binary, ensure it is on your `PATH`:

```sh
which hurl-lsp
```

For Homebrew installs, make sure `/opt/homebrew/bin` (Apple Silicon) or `/usr/local/bin` (Intel) is in your shell `PATH`.

## Upstream PR

To enable zero-config support for all Helix users, submit a PR to the official Helix repository adding `hurl` to `languages.toml`. See the checklist below once you're ready.

**Checklist:**

1. Fork [helix-editor/helix](https://github.com/helix-editor/helix).
2. Add the `hurl` language entry and `hurl-lsp` server block to `languages.toml`.
3. Include a short sample config and validation notes in the PR description.
4. Add the merged PR link to the main `README.md` once accepted.
