# Helix Setup

Install `hurl-lsp` first:

```sh
cargo install hurl-lsp
hurl-lsp --version
```

Add this to `~/.config/helix/languages.toml`:

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

## Troubleshooting

- Run `hx --health hurl` and verify `hurl-lsp` is detected.
- If Helix cannot find the server, ensure the binary is in your shell `PATH`.
- Reopen Helix after installation to refresh language server discovery.

## Upstream PR Checklist

1. Fork `helix-editor/helix`.
2. Add `hurl` language entry with `hurl-lsp` server in `languages.toml`.
3. Include a short sample configuration and validation notes in PR description.
4. Keep the PR link in this repo README once opened.
