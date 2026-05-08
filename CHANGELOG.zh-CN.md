# 更新日志

## 0.1.13 - 2026-05-08

### 修复

- 将 Zed 扩展升级为完整的 Rust LSP 扩展：通过 `[grammars.hurl]` 声明 tree-sitter 语法，添加正确的 `[language_servers.hurl-lsp]` 配置，将 `injections.scm` 重写为 Zed 原生捕获名，修正高亮 token（`@number.float`、`@string.regexp`）

### 文档

- 完善 Helix 配置文档，新增 Homebrew 安装方式、功能支持矩阵和故障排查说明

### 依赖更新

- 升级 `hurl_core` 和 `hurlfmt` 从 7.1.0 至 8.0.1（需要 Rust 1.95.0）
- 新增 `certificate` 和 `rawbytes` 的断言补全和 Hover 文档（Hurl 8.0.0 新增 query 类型）
- 在 Zed 语法高亮中新增 `charsetDecode`、`utf8Decode`、`utf8Encode`（Hurl 8.x 新增 filter）

---

## 0.1.12 - 2026-03-29

### 修复

- 修正 Zed `extension.toml` 中顶层扩展元数据的写法

### 维护

- 重命名 VSCode 插件显示名称；升级扩展版本号

---

## 0.1.11 - 2026-03-29

### 维护

- 升级 crate 版本至 0.1.11
