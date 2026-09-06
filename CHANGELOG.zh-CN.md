# 更新日志

## 未发布

### 新功能

- 使用原生 Inlay Hint 预览 Hurl 变量，支持来源 Hover、变量文件自动刷新和敏感值遮罩
- 新增 Hurl Inspector，结构化展示请求/响应 headers 与 body、JSON 格式化、原始输出、失败断言和内存运行历史
- 将解析变量后的 cURL 直接复制到剪贴板，全程不发送请求，并在 Inspector 中提供遮罩预览

### 修复

- 诊断、Hover、补全、Run with vars 与 cURL 生成统一使用同一套变量合并优先级

### 文档

- 增加 Marketplace 图标：几何 stroke `hurl` wordmark，官方粉

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
