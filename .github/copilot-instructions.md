# Sixu 项目 AI 编码指南

## 项目概述

Sixu（思绪）是一个为视觉小说（Visual Novel）设计的脚本语言，包含三个主要组件：

- **sixu**: 核心库（Rust crate），包含解析器、运行时和 AST 格式定义
- **sixu-lsp**: LSP 服务器（Rust），为编辑器提供智能功能
- **sixu-vscode-extension**: VS Code 扩展（TypeScript），作为 LSP 客户端

## 架构关键点

### 数据流架构

1. **解析流程**: `.sixu文件` → `parser::parse()` → `Story AST` → `Runtime` → `Executor trait实现`
2. **运行时层次**: `Runtime<E: RuntimeExecutor>` 管理 `RuntimeContext`（包含变量、状态栈）和具体的 `Executor` 实现
3. **LSP 工作流**: VS Code 客户端 ↔ stdio ↔ sixu-lsp 服务器 ↔ `sixu::parser`

### 核心设计模式

#### 1. AST 结构（[sixu/src/format.rs](../sixu/src/format.rs)）

```rust
Story { paragraphs: Vec<Paragraph> }
└── Paragraph { name, parameters, block }
    └── Block { lines: Vec<Line> }
        ├── Text { leading, text, tailing }
        ├── CommandLine { command, arguments }
        └── SystemCallLine { kind, targets }
```

#### 2. 运行时 Trait 模式

- `RuntimeExecutor` trait 定义了所有运行时行为（[sixu/src/runtime/executor.rs](../sixu/src/runtime/executor.rs)）
- 实现者需要处理：`handle_command()`, `handle_text()`, `eval_script()`, `read_story_file()`
- 使用泛型 `Runtime<E: RuntimeExecutor>` 实现依赖注入

#### 3. Parser 组合子（基于 nom）

- 所有 parser 模块遵循 `nom::Parser` trait
- 位于 `sixu/src/parser/` 下，每个语法元素一个文件（如 `command_line.rs`, `paragraph.rs`）
- 入口点：`pub fn parse<'a>(name: &'a str, input: &'a str) -> ParseResult<&'a str, Story>`

## 开发工作流

### 构建和测试

```bash
# Rust workspace（根目录）
cargo build                    # 构建所有crate
cargo test                     # 运行测试
cargo build --release          # 发布构建

# LSP服务器（调试用）
cd sixu-lsp
cargo run                      # 启动LSP服务器（stdio模式）

# VS Code扩展
cd sixu-vscode-extension
yarn compile                   # 编译TypeScript
# 按F5启动扩展调试（需要先编译LSP服务器到target/debug/sixu-lsp.exe）
```

### 调试 LSP

- LSP 服务器路径硬编码在 [sixu-vscode-extension/src/extension.ts](../sixu-vscode-extension/src/extension.ts#L23): `../target/debug/sixu-lsp.exe`
- 修改 LSP 后需重新 `cargo build` 然后重启 VS Code 调试会话
- LSP 通过 stdio 通信，调试日志需使用 `log::info!()` 等宏

## 项目特定约定

### Schema 驱动开发

- **命令定义**: 所有可用命令在 `commands.schema.json`（JSON Schema 格式）中定义
- LSP 服务器加载此文件提供：
  - 命令补全（触发字符：`@`）
  - 参数补全（触发字符：` `空格，支持括号和非括号语法）
  - 参数类型校验（[sixu-lsp/src/schema.rs](../sixu-lsp/src/schema.rs)）
- **示例**: [sample-project/commands.schema.json](../sample-project/commands.schema.json)

### 两种命令语法

```sixu
@changebg src="test.jpg" fadeTime=600        // 空格分隔
@changebg(src="test.jpg", fadeTime=600)      // 括号分隔（带逗号）
```

- LSP scanner 需同时支持两种格式（[sixu-lsp/src/scanner.rs](../sixu-lsp/src/scanner.rs)）
- parser 使用 nom 组合子统一处理（[sixu/src/parser/command_line.rs](../sixu/src/parser/command_line.rs)）

### 文本格式（三种）

```sixu
裸文本                                    // 不转义
"带转义的文本\n\u6D4B\u{8BD5}"            // 转义字符
`模板字符串 ${变量名}`                    // 支持变量插值和多行
```

### 系统调用

- 使用 `#` 前缀：`#goto paragraph_name`, `#call story::paragraph`
- LSP 提供跳转定义功能（[sixu-lsp/src/main.rs](../sixu-lsp/src/main.rs) 中的 `goto_definition`）

## 常见修改场景

### 添加新的语法元素

1. 在 `sixu/src/format.rs` 添加 AST 节点（带 `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`）
2. 在 `sixu/src/parser/` 创建新的 parser 模块
3. 在 `sixu/src/cst/` 创建新的 cst parser 模块
4. 集成到父级 parser（如 `block.rs` 或 `paragraph.rs`）
5. 更新测试用例（`sixu/tests/fixtures/`）

### 修改 LSP 功能

- **补全**: 修改 `sixu-lsp/src/main.rs` 中的 `completion()` 方法
- **诊断**: 修改 `validate()` 方法，遍历 AST 时添加检查逻辑
- **格式化**: 实现 `sixu-lsp/src/formatter.rs` 中的 Pretty Printer（注意：当前实现会丢失注释）

### 扩展 Runtime

- 实现 `RuntimeExecutor` trait
- 示例见测试文件：[sixu/tests/usage.rs](../sixu/tests/usage.rs)
- 关键方法：`handle_command()` 返回 `bool` 表示是否立即执行下一行

## 陷阱与注意事项

⚠️ **注释处理**: 当前 parser 会忽略注释，格式化会丢失注释。如需保留，考虑切换到 CST 或在 AST 中保留注释节点

⚠️ **LSP 容错解析**: `sixu-lsp/src/scanner.rs` 使用独立的容错 parser，与主 parser 行为需保持同步

⚠️ **字符串位置计算**: LSP 诊断需正确映射 nom 的字节偏移到 LSP 的行/列（使用 `ropey::Rope`）

⚠️ **Workspace 结构**: 使用 Cargo workspace，子 crate 在 `members = ["sixu", "sixu-lsp"]` 中定义

⚠️ **两边 parser 合并**: `note.md` 提到需要合并 LSP scanner 和主 parser，避免重复实现

⚠️ **补全触发条件**: 需排除不应触发的位置（字符串内、段落外、已完成参数列表）——详见 [note.md](../note.md)

## 参考文档

- 语法规范: [docs/syntax.md](../docs/syntax.md)
- LSP 设计: [docs/lsp.md](../docs/lsp.md)
- 示例脚本: [sample-project/assets/scenarios/](../sample-project/assets/scenarios/)
- 开发笔记: [note.md](../note.md)（包含待办和优化点）
