<div align="center">

# hurl-lsp

**[Hurl](https://hurl.dev) 的 Language Server Protocol 实现**

为 `.hurl` 文件提供更完整的编辑器智能能力：
诊断、补全、格式化、大纲等。

[![WIP](https://img.shields.io/badge/status-WIP-orange?style=flat-square)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![LSP](https://img.shields.io/badge/protocol-LSP%203.17-blueviolet?style=flat-square)]()

</div>

---

English README: [`README.md`](README.md)

---

> ⚠️ **开发中**：`hurl-lsp` 仍在快速迭代，当前不建议直接用于生产环境。欢迎反馈和贡献。

---

## 概览

[Hurl](https://hurl.dev) 是一个用纯文本 `.hurl` 文件定义 HTTP 请求并执行测试的工具，天然适合版本管理与 CI/CD。

`hurl-lsp` 目标是补齐 `.hurl` 文件在编辑器里的体验缺口：补全、诊断、悬停说明、格式化、结构大纲等。

当前已实现：

- Rust 语言服务器（诊断、补全、Hover、格式化、大纲、变量跳转）
- VSCode 扩展（语言注册、语法高亮、片段、LSP 客户端）
- VSCode 自动下载服务端二进制（macOS / Linux / Windows）
- Zed 扩展骨架与 Helix 配置文档

尚未实现：

- 更丰富的断言行内结果渲染（actual value）
- VSCode Webview 面板
- TestMind 集成

---

## 功能

### 语法诊断

- 基于 `hurl_core` 的解析错误
- 非法 HTTP Method 检测
- 非法 Section 名称检测
- 未定义变量 `{{variable}}` 警告
- 单请求块内重复 Section 警告
- 非法 `HTTP` 状态码格式检测

### 智能补全

- HTTP Methods：`GET`、`POST`、`PUT`、`DELETE`、`PATCH` 等
- Section：`[Query]`、`[Headers]`、`[Captures]`、`[Asserts]`、`[Options]` 等
- 断言函数：`jsonpath`、`xpath`、`regex`、`header`、`status`、`duration` 等
- 常见 Content-Type
- `{{` 上下文变量补全（来自同文件 `[Captures]`）

### Hover 文档

在方法、Section、断言函数上悬停可查看简短说明。

### 跳转定义

变量引用可跳转到同文件 `[Captures]` 定义；  
若工作区存在 `.hurl-vars`、`vars.env`、`hurl.env`、`.env`，也可跨文件解析定义与诊断。

### Code Lens

每个请求块支持：

- 摘要行（method/path + section 统计）
- `▶ Run`（执行当前请求块）
- `⚡ Run with vars`（使用就近变量文件执行）
- `⛓ Run chain`（执行当前请求及其依赖步骤）
- `📄 Run file`（执行当前文件全部请求）
- `📋 Copy as curl`（复制 curl）

运行告警行为：

- `hurl.run.inlineFailureDiagnostics`：默认 `true`，显示红色行内失败诊断；设为 `false` 可关闭
- `Hurl: Clear Run Alerts`：立即清除当前文件的运行告警诊断

### OpenAPI 联动补全

当工作区存在 `openapi.yaml` / `openapi.yml` / `swagger.yaml` / `swagger.yml` / `swagger.json` 时：

- 支持 `paths` URL 补全
- 支持 `requestBody` 字段补全

### 内置格式化

`Format Document` 通过 LSP 调用官方 `hurlfmt::format::format_text(..., false)`。
可使用命令 `Hurl: Format Document` 显式触发格式化，并在运行日志中查看记录。

### Markdown 导出（VSCode）

命令 `Hurl: Export as Markdown` 可将当前 `.hurl` 文件导出为同目录 `.md` 文件。
导出会遵循大纲配置：

- `hurl.outline.groupMode`: `hierarchical` | `flat`
- `hurl.outline.sortMode`: `source` | `priority`

### VSCode Webview 面板

命令 `Hurl: Open Webview Panel` 可打开独立标签页，包含：

- `Single Request`：单请求详情 + `Run` / `Run Chain` 动作
- `Chain Graph`：请求列表与推断/显式依赖边

### 文档大纲（Document Symbol）

支持优先展示元数据结构；若无元数据则回退到请求级符号。

可通过 VSCode 配置控制行为：

- `hurl.outline.groupMode`：`hierarchical` 保留 chain/priority 分组，`flat` 仅显示请求条目
- `hurl.outline.sortMode`：`source` 按文件顺序，`priority` 按 `P0 > P1 > P2`（步骤顺序为 `setup > test > teardown`）

在 VSCode 资源管理器中，`Hurl Requests` 视图提供可执行请求节点和内联动作：

- `Run`（执行该请求）
- `Run Chain`（执行该请求及其推断/声明依赖）

---

## 编辑器支持

| 编辑器 | 状态 | 说明 |
|--------|------|------|
| **VSCode** | 已实现 | `editors/vscode` 中包含扩展与二进制下载逻辑 |
| **Helix** | 已实现（手动配置） | 见 `editors/helix/README.md` |
| **Neovim** | 手动配置可用 | 可直接按 LSP 配置接入 |
| **Zed** | 基线已实现 | `editors/zed` 中提供扩展骨架（待发布） |

---

## 安装

### VSCode

在 VS Marketplace 搜索 `hurl-lsp` / `Hurl` 安装。  
也可使用 release 流水线产出的 `.vsix` 包安装。

二进制解析顺序：

1. 若配置了 `hurl.server.path`，优先使用该路径。
2. 否则检查本机 `PATH` 中的 `hurl-lsp`，若版本 >= 扩展版本则直接复用。
3. 否则自动下载与扩展版本匹配的 release 二进制。

可通过命令 `Hurl: Show Log` 打开扩展/运行日志。

### Zed

安装 Zed 扩展后，再安装二进制：

```sh
cargo install hurl-lsp
```

### Helix

先安装二进制，再在 `~/.config/helix/languages.toml` 中添加：

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

## 二进制安装

**Cargo：**

```sh
cargo install hurl-lsp
```

**Homebrew（tap 方式）：**

```sh
brew tap testmind-hq/tap
brew install hurl-lsp
```

Homebrew 公式位于：`packaging/homebrew/Formula/hurl-lsp.rb`  
发布产物位于：<https://github.com/testmind-hq/hurl-lsp/releases>

---

## 路线图（摘要）

### Phase 1（核心 LSP）

- [x] 诊断、补全、Hover、格式化、CI

### Phase 2（编辑器与分发）

- [x] 多平台 Release 自动化
- [x] VSCode 扩展基线
- [x] Zed 扩展基线
- [~] VS Marketplace 发布流程（workflow 已准备）
- [~] crates.io 发布流程（workflow 已准备）
- [ ] Zed Extensions 正式发布

### Phase 3（差异化能力）

- [x] 变量文件联动
- [x] Code Lens（run / run-with-vars / copy-as-curl）
- [x] OpenAPI 联动补全
- [x] 文档大纲与链路依赖标注

### Phase 4（生态）

- [~] VSCode Webview（单请求 + 依赖图基线）
- [x] Markdown 导出（已支持分组/排序联动）
- [~] Homebrew 分发（公式与校验和流程已就绪）
- [ ] Hurl 官方文档 PR
- [ ] TestMind CI 结果回流

---

## 架构

核心依赖：

- `tower-lsp`：LSP 服务框架
- `hurl_core`：官方 Hurl 解析与 AST
- `tokio`：异步运行时
- `dashmap`：并发文档状态存储

LSP 服务通过 stdin/stdout JSON-RPC 与编辑器通信。

---

## 贡献

欢迎贡献。若是较大功能，建议先提 issue 讨论。

```sh
git clone https://github.com/testmind-hq/hurl-lsp
cd hurl-lsp
cargo build
cargo test
cargo clippy
```

---

## 许可证

MIT（见 [LICENSE](LICENSE)）。
