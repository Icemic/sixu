# Sixu Language Server Protocol (LSP) 开发技术文档

本文档记录了 Sixu 语言 VS Code 插件及 LSP 服务器的技术设计方案。

## 1. 概述

目标是为 Sixu 脚本语言提供现代化的编辑体验，包括代码补全、实时错误检查、格式化等功能。
核心策略是采用 **Rust** 编写 LSP 服务器，以直接复用现有的 `sixu` 解析器（Parser）逻辑，确保解析行为的一致性并获得高性能。

## 2. 架构设计

采用标准的 LSP (Language Server Protocol) 架构：

- **Client (VS Code Extension)**: 负责启动 Server，转发编辑器事件（打开文件、修改、补全请求等），并渲染 Server 返回的结果（诊断红线、补全列表）。
- **Server (`sixu-lsp`)**: 一个独立的 Rust 二进制程序。负责解析代码、分析语义、计算补全项、执行格式化，并通过标准输入/输出 (stdio) 与 Client 通信。

### 技术栈

- **Server**:
  - Language: Rust
  - LSP Framework: `tower-lsp-server` (处理 JSON-RPC 和 LSP 协议细节)
  - Parser: 复用 `sixu` crate (基于 `nom`)
  - Schema Parsing: `serde_json` (解析 `commands.schema.json`)
  - Async Runtime: `tokio`
- **Client**:
  - Language: TypeScript
  - Library: `vscode-languageclient`

## 3. 核心功能实现细节

### 3.1. 命令补全 (Completion)

- **数据源**: 项目根目录下的 `commands.schema.json`。
- **触发字符**: `@` (触发命令提示), ` ` (空格，触发参数提示)，`#` (触发系统调用提示)
- **逻辑**:
  1.  **加载 Schema**: Server 启动时读取并缓存 Schema。监听文件变动事件以更新缓存。
  2.  **上下文分析**:
      - **命令补全**: 当光标位于 `@` 后，返回 Schema 中所有定义的 `command`。补全后自动插入空格。
      - **参数补全**: 当光标位于命令内部（支持空格分隔 `@cmd arg` 或括号分隔 `@cmd(arg)`），解析当前行已有的参数，过滤掉已存在的参数，返回剩余可选参数。
      - **Snippet 支持**: 参数补全会自动插入 `key="value"` 格式。
        - 如果 Schema 定义了 `default` 值，则插入 `key=default`。
        - 如果类型为 `string`，则插入 `key="$1"` 并将光标置于引号内。
      - **系统调用补全**: 当光标位于 `#` 后，提示 `goto`, `call`, `replace`, `break`, `finish`。
      - **系统调用参数补全**: 提示 `paragraph`, `story` 以及当前文件内的段落名。
  3.  **容错处理**: 实现了独立的 `scanner.rs`，使用 `nom` 进行容错解析。即使代码不完整（如正在输入时），也能识别出当前所在的命令和参数上下文。支持两种命令调用风格：
      - 空格分隔: `@bg file="test.jpg"`
      - 括号分隔: `@bg(file="test.jpg")`

### 3.2. 诊断与校验 (Diagnostics)

- **触发时机**: `textDocument/didOpen` 和 `textDocument/didChange`。
- **两层校验**:
  1.  **语法校验 (Syntax)**: 调用 `sixu::parser::parse`。如果解析失败，将 `nom` 返回的错误位置映射为 LSP `Diagnostic`，标记为 Error。
  2.  **语义校验 (Schema)**: 如果语法解析成功，遍历生成的 AST (`Story` -> `Paragraph` -> `Block` -> `Command`)。
      - 验证命令名是否在 Schema 中定义。
      - 验证参数类型（如期望 `number` 却传入 `string`）。
      - 验证 `required` 参数是否缺失。
      - 将发现的问题标记为 Warning 或 Error。

### 3.3. 代码格式化 (Formatting)

- **方案**: **Pretty Printer (重构生成)**
- **实现**:
  - 不直接操作文本字符串，而是基于 AST。
  - 实现一个 `Formatter` trait 或结构体，接收 `Story` AST。
  - 按照标准规范（如：缩进 4 空格，大括号换行规则，属性间距等）重新生成 Sixu 源代码字符串。
  - **优点**: 保证生成的代码绝对符合语法规范，能够自动修复格式混乱的代码。
  - **注意**: 需要确保 AST 中保留了必要的注释信息（`sixu` parser 目前似乎支持注释，需确认 AST 中是否包含注释节点，否则格式化可能会丢失注释）。_注：检查代码发现 `sixu` parser 会忽略注释，这在格式化时是一个需要解决的问题，可能需要修改 parser 以保留注释节点，或者采用 CST (Concrete Syntax Tree) 方案。初期可先实现不带注释的格式化，或仅对代码块内容进行格式化。_

### 3.4. 预览与调试 (F5 Run/Preview)

- **目标**: 支持用户按下 F5 预览当前脚本效果。
- **方案**:
  - **Task Provider**: 注册 VS Code Task，运行 `sixu run <current_file>` (假设 sixu cli 有此功能)。
  - **Webview Preview**: 插件内部集成一个简单的 Webview 运行时（基于 Sixu 的 JS/TS 运行时），直接在编辑器右侧渲染当前脚本的预览效果。
  - **Debug Adapter**: (远期规划) 实现 DAP 协议，支持断点调试。

### 3.5. 悬停提示 (Hover)

- **触发时机**: `textDocument/hover`。
- **功能**:
  - 当鼠标悬停在 **命令名** (`@cmd`) 上时，显示 Schema 中定义的命令描述 (`description`)。
  - 当鼠标悬停在 **参数名** (`arg=`) 上时，显示 Schema 中定义的参数描述。
- **实现**:
  - 复用 `scanner.rs` 的解析结果，判断光标位置是否落在命令名或参数名的 Range 内。
  - 查找 `commands.schema.json` 获取对应的文档信息。
  - 支持多路径查找 Schema（优先当前目录，其次查找 `sample-project` 等预设路径）。

### 3.6. 跳转定义 (Go to Definition)

- **触发时机**: `textDocument/definition` (Ctrl+Click 或 F12)。
- **支持场景**:
  - `#goto`, `#call`, `#replace` 系统调用。
  - 支持跳转到当前文件内的段落定义 (`::paragraph_name`)。
  - 支持跨文件跳转 (当指定 `story="filename"` 参数时)。
- **实现**:
  - 扫描当前行的系统调用。
  - 解析 `paragraph` 参数（支持命名参数 `paragraph="name"` 或位置参数）。
  - 解析 `story` 参数（可选）。
  - 如果是跨文件，计算目标文件路径并读取内容。
  - 扫描目标文件中的段落定义 (`::name`) 并返回位置。

### 3.7. 文档符号 (Document Symbols)

- **触发时机**: `textDocument/documentSymbol` (Outline 视图)。
- **功能**: 列出当前文件中定义的所有段落 (`::paragraph_name`)。
- **实现**:
  - 使用 `scanner.rs` 扫描 `::` 开头的标识符。
  - 返回 `SymbolKind::Class` 或 `Namespace` 类型的符号列表。

## 4. 数据结构与接口

Server 端将直接引用 `sixu` crate 的数据结构：

```rust
// 引用自 sixu::format
pub struct Story { ... }
pub struct Paragraph { ... }
pub struct Command { ... }
```

## 5. 开发计划

1.  **Phase 1: 基础框架**
    - 创建 `sixu-lsp` crate。
    - 搭建 `tower-lsp-server` 样板代码。
    - 配置 VS Code 插件启动 Server。
2.  **Phase 2: 核心解析与诊断**
    - 集成 `sixu` parser。
    - 实现 `textDocument/didChange` 解析代码。
    - 实现语法错误报告 (Diagnostics)。
3.  **Phase 3: Schema 驱动的补全与校验**
    - 实现 `commands.schema.json` 解析。
    - 实现基于 Schema 的参数校验。
    - 实现命令和参数的自动补全。
4.  **Phase 4: 格式化**
    - 实现 AST 到 String 的 Pretty Printer。
    - 处理缩进和换行。

## 6. 开发指南 (Development Guide)

### 6.1. 环境准备

- **Rust**: 安装最新稳定版 Rust (使用 rustup)。
- **Node.js**: 推荐 LTS 版本。
- **Yarn**: 包管理器。
- **VS Code**: 推荐编辑器。

### 6.2. 项目构建

本项目包含两部分：LSP Server (Rust) 和 VS Code Extension (TypeScript)。

#### 1. 构建 LSP Server

```bash
cd sixu-lsp
cargo build
# 生成的可执行文件位于 target/debug/sixu-lsp.exe
```

#### 2. 构建 VS Code Extension

```bash
cd sixu-vscode-extension
yarn install
yarn compile
```

### 6.3. 调试运行

1.  在 VS Code 中打开项目根目录 (`.\sixu`)。
2.  按 `F5` 启动调试。
    - 这会启动一个新的 "Extension Development Host" 窗口。
    - 该窗口会自动加载插件，并启动 `sixu-lsp` 服务器。
3.  在调试窗口中打开 `.sixu` 文件即可测试功能。

### 6.4. 常见问题排查

- **LSP 未启动**: 检查 `sixu-vscode-extension/src/extension.ts` 中的 Server 路径配置是否正确指向了 `target/debug/sixu-lsp.exe`。
- **查看日志**: 在 VS Code "输出 (Output)" 面板中选择 "Sixu Language Server" 查看服务端日志。

### 6.5. 关于日志打印 (重要!)

**千万不要使用 `println!`**。
LSP 通过标准输出 (stdout) 与编辑器通信。使用 `println!` 会破坏协议格式，导致插件崩溃。

- **正确做法 1 (推荐)**: 使用 `eprintln!`。它会输出到 stderr，VS Code 会将其捕获并显示在 Output 面板中。
- **正确做法 2**: 使用 `self.client.log_message(...)` 发送 LSP 日志消息。
