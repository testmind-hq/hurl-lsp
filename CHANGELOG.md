# Changelog

## 0.1.13 - 2026-05-08

### Fixes

- Upgrade Zed extension to full Rust LSP extension with Zed-compatible config: tree-sitter grammar via `[grammars.hurl]`, proper `[language_servers.hurl-lsp]` section, Zed-native `injections.scm` capture names, aligned highlight tokens (`@number.float`, `@string.regexp`)

### Documentation

- Expand Helix setup guide with Homebrew install option, feature support matrix, and troubleshooting instructions

### Dependencies

- Upgrade `hurl_core` and `hurlfmt` from 7.1.0 to 8.0.1 (requires Rust 1.95.0)
- Add `certificate` and `rawbytes` to assertion completion and hover documentation (new query types in Hurl 8.0.0)
- Add `charsetDecode`, `utf8Decode`, `utf8Encode` to Zed syntax highlights (new filters in Hurl 8.x)

---

## 0.1.12 - 2026-03-29

### Fixes

- Use top-level extension metadata in Zed `extension.toml`

### Maintenance

- Rename VSCode extension display name; bump extension version

---

## 0.1.11 - 2026-03-29

### Maintenance

- Bump crate version to 0.1.11
