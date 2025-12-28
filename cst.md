# Sixu CST (Concrete Syntax Tree) 实现方案

> 文档版本: v1.0  
> 创建日期: 2025-12-27  
> 状态: 设计阶段

## 目录

- [1. 概述](#1-概述)
- [2. 设计原则](#2-设计原则)
- [3. 架构设计](#3-架构设计)
- [4. 数据结构定义](#4-数据结构定义)
- [5. 实施步骤](#5-实施步骤)
- [6. 注意事项](#6-注意事项)
- [7. 测试策略](#7-测试策略)

---

## 1. 概述

### 1.1 背景

当前 Sixu 项目存在两套 parser：
- **sixu parser**：生成 AST，用于剧本执行
- **sixu-lsp scanner**：容错扫描，用于 LSP 功能

这种重复实现导致：
- 维护成本高（语法改动需要同步两边）
- 代码冗余
- 行为不一致的风险

### 1.2 目标

实现独立的 CST（Concrete Syntax Tree）模块，用于替代 LSP scanner，并支持未来的工具链需求：

- ✅ **LSP**：代码补全、诊断、悬停提示、跳转定义
- ✅ **Formatter**：代码格式化（空格规范化、空白行缩减）
- ✅ **Linter**：代码检查（未来）
- ✅ **Refactoring**：代码重构（未来）

### 1.3 核心特性

| 特性 | 说明 |
|------|------|
| **完整性** | 保留所有源代码信息（空白、注释、token） |
| **位置跟踪** | 每个节点都有精确的位置信息 |
| **容错性** | 支持部分解析和错误恢复 |
| **可逆性** | 可以从 CST 完整还原源代码 |
| **语义复用** | 复用 AST 的类型定义（`CommandLine` 等） |
| **可选性** | 作为 feature，不影响默认构建 |

---

## 2. 设计原则

### 2.1 职责分离

```
┌─────────────────────────────────────────────────┐
│  源代码 (.sixu 文件)                             │
└─────────────────┬───────────────────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
        ▼                   ▼
    ┌───────┐          ┌───────┐
    │  AST  │          │  CST  │
    └───┬───┘          └───┬───┘
        │                  │
        ▼                  ├──────────┐
  ┌──────────┐            │          │
  │ Runtime  │            ▼          ▼
  │ 执行剧本  │      ┌──────────┐  ┌──────────┐
  └──────────┘      │   LSP    │  │ Formatter│
                    │ 代码补全  │  │ 代码格式化│
                    └──────────┘  └──────────┘
```

**原则**：
- AST：语义优先，只保留执行所需信息
- CST：语法优先，保留所有源代码细节
- 互不干扰，各司其职

### 2.2 复用优先

```rust
// ❌ 不要重复定义
pub struct AstCommandLine { ... }
pub struct CstCommandLine { ... }

// ✅ 复用 + 增强
pub struct CstCommand {
    pub semantic: CommandLine,  // 复用 AST 定义
    pub syntax: CstCommandSyntax,  // CST 专有信息
}
```

### 2.3 渐进式实现

**Phase 1**: Command + SystemCall（满足当前 LSP 需求）  
**Phase 2**: Paragraph + Block（支持完整导航）  
**Phase 3**: Text + Template（支持格式化）  
**Phase 4**: 完整 CST（所有节点）

---

## 3. 架构设计

### 3.1 目录结构

```
sixu/
├── src/
│   ├── format.rs              # AST 定义（保持不变）
│   ├── parser/                # AST Parser（保持不变）
│   │   ├── mod.rs
│   │   ├── command_line.rs
│   │   └── ...
│   │
│   ├── cst/                   # 新增：CST 模块
│   │   ├── mod.rs             # 模块入口
│   │   ├── node.rs            # CST 节点定义
│   │   ├── span.rs            # 位置信息工具
│   │   ├── parser.rs          # CST Parser（容错）
│   │   ├── convert.rs         # CST → AST 转换
│   │   └── visitor.rs         # CST 遍历器（可选）
│   │
│   ├── runtime/               # 运行时（不变）
│   └── lib.rs
│
├── Cargo.toml
└── cst.md                     # 本文档
```

### 3.2 Feature 配置

```toml
[features]
default = ["serde", "ts"]
serde = ["dep:serde"]
ts = ["dep:ts-rs"]
cst = ["dep:nom_locate", "dep:rowan"]  # 新增

[dependencies]
# 现有依赖
nom = "8.0"
nom-language = "0.1"
# ... 其他 ...

# CST 专用依赖（可选）
nom_locate = { version = "5.0.0", features = ["runtime-dispatch-simd"], optional = true }
rowan = { version = "0.15", optional = true }  # 可选：用于 Red-Green Tree
```

**说明**：
- `nom_locate`：必需，提供位置跟踪
- `rowan`：可选，提供高效的 CST 存储（Red-Green Tree）
  - 初期可以不用，用简单的 `Vec<CstNode>` 即可
  - 未来优化时再考虑

### 3.3 模块依赖关系

```
sixu::format (AST)
    ↑
    │ (使用)
    │
sixu::cst::node ────→ sixu::cst::parser
    ↑                      │
    │                      │ (生成)
    │                      ▼
sixu::cst::convert    sixu::cst (CstRoot)
    │
    ▼
sixu::format (AST)
```

---

## 4. 数据结构定义

### 4.1 核心类型

#### 4.1.1 Span 信息

```rust
// sixu/src/cst/span.rs

use nom_locate::LocatedSpan;

/// CST 使用的输入类型
pub type Span<'a> = LocatedSpan<&'a str>;

/// 位置信息（字节偏移 + 行列号）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanInfo {
    /// 起始字节偏移
    pub start: usize,
    /// 结束字节偏移
    pub end: usize,
    /// 起始行号（1-based）
    pub start_line: usize,
    /// 起始列号（0-based）
    pub start_column: usize,
    /// 结束行号（1-based）
    pub end_line: usize,
    /// 结束列号（0-based）
    pub end_column: usize,
}

impl SpanInfo {
    /// 从 nom_locate::Span 创建
    pub fn from_span(span: Span) -> Self {
        // 实现细节
    }
    
    /// 从两个 Span 创建（表示范围）
    pub fn from_range(start: Span, end: Span) -> Self {
        // 实现细节
    }
    
    /// 计算长度（字节）
    pub fn len(&self) -> usize {
        self.end - self.start
    }
}
```

#### 4.1.2 Trivia（空白和注释）

```rust
// sixu/src/cst/node.rs

/// Trivia：不影响语义的语法元素
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CstTrivia {
    /// 空白（空格、制表符、换行）
    Whitespace {
        content: String,
        span: SpanInfo,
    },
    
    /// 单行注释 // ...
    LineComment {
        content: String,  // 不含 //
        span: SpanInfo,
    },
    
    /// 块注释 /* ... */
    BlockComment {
        content: String,  // 不含 /* */
        span: SpanInfo,
    },
}

impl CstTrivia {
    pub fn span(&self) -> &SpanInfo {
        match self {
            Self::Whitespace { span, .. } => span,
            Self::LineComment { span, .. } => span,
            Self::BlockComment { span, .. } => span,
        }
    }
    
    pub fn content(&self) -> &str {
        match self {
            Self::Whitespace { content, .. } => content,
            Self::LineComment { content, .. } => content,
            Self::BlockComment { content, .. } => content,
        }
    }
    
    /// 是否包含换行
    pub fn has_newline(&self) -> bool {
        self.content().contains('\n')
    }
}
```

#### 4.1.3 CST 根节点

```rust
// sixu/src/cst/node.rs

/// CST 根节点（代表整个文件）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstRoot {
    /// 文件名
    pub name: String,
    
    /// 所有节点（包括 trivia）
    pub nodes: Vec<CstNode>,
    
    /// 全文 span
    pub span: SpanInfo,
}

/// CST 节点（所有可能的语法元素）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CstNode {
    /// Trivia（空白、注释）
    Trivia(CstTrivia),
    
    /// 段落定义
    Paragraph(CstParagraph),
    
    /// 命令
    Command(CstCommand),
    
    /// 系统调用
    SystemCall(CstSystemCall),
    
    /// 文本行
    TextLine(CstTextLine),
    
    /// 代码块
    Block(CstBlock),
    
    /// 嵌入代码
    EmbeddedCode(CstEmbeddedCode),
    
    /// 错误节点（解析失败但需要保留的部分）
    Error {
        content: String,
        span: SpanInfo,
        message: String,
    },
}

impl CstNode {
    pub fn span(&self) -> SpanInfo {
        match self {
            Self::Trivia(t) => *t.span(),
            Self::Paragraph(p) => p.span,
            Self::Command(c) => c.span,
            Self::SystemCall(s) => s.span,
            Self::TextLine(t) => t.span,
            Self::Block(b) => b.span,
            Self::EmbeddedCode(e) => e.span,
            Self::Error { span, .. } => *span,
        }
    }
}
```

### 4.2 具体节点类型（Phase 1: Command 和 SystemCall）

#### 4.2.1 Command 节点

```rust
// sixu/src/cst/node.rs

/// 命令节点 @command arg1=val1 arg2
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstCommand {
    /// 语义信息（复用 AST）
    pub command: String,
    
    /// @ 符号的位置
    pub at_token: SpanInfo,
    
    /// 命令名的位置
    pub name_span: SpanInfo,
    
    /// 参数列表
    pub arguments: Vec<CstArgument>,
    
    /// 命令调用语法风格
    pub syntax: CommandSyntax,
    
    /// 整个命令的范围
    pub span: SpanInfo,
    
    /// 前导 trivia（命令前的空白/注释）
    pub leading_trivia: Vec<CstTrivia>,
}

/// 命令语法风格
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CommandSyntax {
    /// 括号风格：@cmd(a=1, b=2)
    Parenthesized {
        /// ( 的位置
        open_paren: SpanInfo,
        /// ) 的位置
        close_paren: SpanInfo,
    },
    
    /// 空格分隔：@cmd a=1 b=2
    SpaceSeparated,
}

impl CstCommand {
    /// 转换为 AST CommandLine
    pub fn to_ast(&self) -> crate::format::CommandLine {
        crate::format::CommandLine {
            command: self.command.clone(),
            arguments: self.arguments.iter().map(|a| a.to_ast()).collect(),
        }
    }
}
```

#### 4.2.2 Argument 节点

```rust
// sixu/src/cst/node.rs

/// 参数节点 name=value 或 flag
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstArgument {
    /// 参数名
    pub name: String,
    
    /// 参数名的位置
    pub name_span: SpanInfo,
    
    /// = 的位置（如果有）
    pub equals_token: Option<SpanInfo>,
    
    /// 参数值（None 表示布尔标志）
    pub value: Option<CstValue>,
    
    /// 整个参数的范围
    pub span: SpanInfo,
    
    /// 前导 trivia（参数前的空白/注释）
    pub leading_trivia: Vec<CstTrivia>,
    
    /// 尾随 trivia（参数后的逗号、空白等）
    /// 例如：a=1, b=2 中，a=1 后面的 ", " 是 trailing_trivia
    pub trailing_trivia: Vec<CstTrivia>,
}

impl CstArgument {
    /// 转换为 AST Argument
    pub fn to_ast(&self) -> crate::format::Argument {
        crate::format::Argument {
            name: self.name.clone(),
            value: self.value
                .as_ref()
                .map(|v| v.to_ast())
                .unwrap_or(crate::format::RValue::Literal(
                    crate::format::Literal::Boolean(true)
                )),
        }
    }
}
```

#### 4.2.3 Value 节点

```rust
// sixu/src/cst/node.rs

/// 值节点（字符串、数字、变量等）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstValue {
    /// 值的种类
    pub kind: CstValueKind,
    
    /// 原始文本（含引号、前缀等）
    pub raw: String,
    
    /// 解析后的值（用于生成 AST）
    pub parsed: crate::format::RValue,
    
    /// 值的位置
    pub span: SpanInfo,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CstValueKind {
    /// 字符串 "..." 或 '...'
    String {
        /// 引号类型
        quote: QuoteStyle,
    },
    
    /// 模板字符串 `...`
    TemplateString,
    
    /// 整数
    Integer,
    
    /// 浮点数
    Float,
    
    /// 布尔值
    Boolean,
    
    /// 变量引用 foo.bar.baz
    Variable,
    
    /// 数组 [1, 2, 3]（如果未来支持）
    Array,
    
    /// 对象 {a: 1}（如果未来支持）
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QuoteStyle {
    Double,  // "
    Single,  // '
}

impl CstValue {
    /// 转换为 AST RValue
    pub fn to_ast(&self) -> crate::format::RValue {
        self.parsed.clone()
    }
}
```

#### 4.2.4 SystemCall 节点

```rust
// sixu/src/cst/node.rs

/// 系统调用节点 #goto paragraph="main"
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstSystemCall {
    /// 系统调用名
    pub command: String,
    
    /// # 符号的位置
    pub hash_token: SpanInfo,
    
    /// 命令名的位置
    pub name_span: SpanInfo,
    
    /// 参数列表
    pub arguments: Vec<CstArgument>,
    
    /// 调用语法风格
    pub syntax: CommandSyntax,  // 复用 CommandSyntax
    
    /// 整个调用的范围
    pub span: SpanInfo,
    
    /// 前导 trivia
    pub leading_trivia: Vec<CstTrivia>,
}

impl CstSystemCall {
    /// 转换为 AST SystemCallLine
    pub fn to_ast(&self) -> crate::format::SystemCallLine {
        crate::format::SystemCallLine {
            command: self.command.clone(),
            arguments: self.arguments.iter().map(|a| a.to_ast()).collect(),
        }
    }
}
```

### 4.3 其他节点类型（Phase 2-4）

```rust
// sixu/src/cst/node.rs

/// 段落节点 ::paragraph_name(param1, param2) { ... }
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstParagraph {
    pub name: String,
    pub name_span: SpanInfo,
    pub parameters: Vec<CstParameter>,
    pub block: CstBlock,
    pub span: SpanInfo,
    pub leading_trivia: Vec<CstTrivia>,
}

/// 代码块 { ... }
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstBlock {
    pub open_brace: SpanInfo,
    pub children: Vec<CstNode>,
    pub close_brace: SpanInfo,
    pub span: SpanInfo,
}

/// 文本行
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstTextLine {
    pub leading: Option<CstLeadingText>,
    pub text: Option<CstText>,
    pub tailing: Option<CstTailingText>,
    pub span: SpanInfo,
}

/// 嵌入代码
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstEmbeddedCode {
    pub syntax: EmbeddedCodeSyntax,
    pub code: String,
    pub span: SpanInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EmbeddedCodeSyntax {
    Brace,  // @{ ... }
    Hash,   // ## ... ##
}

// 其他节点定义...
```

---

## 5. 实施步骤

### Phase 1: 基础设施和 Command/SystemCall（Week 1-2）

#### 步骤 1.1: 创建模块结构
- [x] 创建 `sixu/src/cst/` 目录
- [x] 创建 `mod.rs`, `node.rs`, `span.rs`, `parser.rs`, `convert.rs`
- [x] 在 `Cargo.toml` 中添加 `cst` feature
- [x] 在 `lib.rs` 中条件导出 `cst` 模块

**代码示例**：
```rust
// sixu/src/lib.rs
#[cfg(feature = "cst")]
pub mod cst;
```

#### 步骤 1.2: 实现基础类型
- [x] 实现 `SpanInfo`（`span.rs`）
- [x] 实现 `CstTrivia`（`node.rs`）
- [x] 实现 `CstRoot`（`node.rs`）

#### 步骤 1.3: 实现 Command CST
- [x] 定义 `CstCommand`, `CstArgument`, `CstValue`（`node.rs`）
- [x] 实现 `parse_command`（`parser.rs`）
- [x] 实现 `CstCommand::to_ast`（`convert.rs`）
- [x] 编写单元测试

**测试用例**：
```rust
#[test]
fn test_parse_command_parenthesized() {
    let input = r#"@changebg(src="test.jpg", fadeTime=600)"#;
    let cst = parse_command(Span::new(input)).unwrap();
    assert_eq!(cst.command, "changebg");
    assert_eq!(cst.arguments.len(), 2);
    assert_eq!(cst.arguments[0].name, "src");
    assert_eq!(cst.arguments[1].name, "fadeTime");
}

#[test]
fn test_parse_command_space_separated() {
    let input = r#"@changebg src="test.jpg" fadeTime=600"#;
    let cst = parse_command(Span::new(input)).unwrap();
    assert_eq!(cst.command, "changebg");
    assert_eq!(cst.arguments.len(), 2);
}
```

#### 步骤 1.4: 实现 SystemCall CST
- [x] 定义 `CstSystemCall`（`node.rs`）
- [x] 实现 `parse_systemcall`（`parser.rs`）
- [x] 实现 `CstSystemCall::to_ast`（`convert.rs`）
- [x] 编写单元测试

#### 步骤 1.5: 实现容错扫描
- [x] 实现 `parse_tolerant`（`parser.rs`）
- [x] 处理 trivia（空白和注释）
- [x] 处理错误节点
- [x] 编写容错测试

**示例**：
```rust
#[test]
fn test_tolerant_parsing() {
    let input = r#"
    @command1 arg=1
    // 注释
    @incomplete_command arg=
    @command2 arg=2
    "#;
    
    let cst = parse_tolerant(input);
    // 应该解析出 command1, trivia(comment), error, command2
    assert_eq!(cst.nodes.len(), 5);
}
```

### Phase 2: Paragraph 和 Block（Week 3-4）

#### 步骤 2.1: 实现 Paragraph CST
- [x] 定义 `CstParagraph`, `CstParameter`（`node.rs`）
- [x] 实现 `parse_paragraph`（`parser.rs`）
- [x] 实现转换为 AST
- [x] 编写测试

#### 步骤 2.2: 实现 Block CST
- [x] 定义 `CstBlock`（`node.rs`）
- [x] 实现 `parse_block`（`parser.rs`）
- [x] 递归解析 block 内容
- [x] 编写测试

#### 步骤 2.3: 实现文件级解析
- [x] 实现 `parse_file`（解析整个 .sixu 文件）
- [x] 生成 `CstRoot`
- [x] 编写集成测试

### Phase 3: Text 和 Template（Week 5-6）✅

#### 步骤 3.1: 实现 Text CST ✅
- [x] 定义 `CstTextLine`, `CstText`, `CstLeadingText`, `CstTailingText`
- [x] 实现 text 解析
- [x] 处理转义字符
- [x] 编写测试（14个测试，100% 通过）

#### 步骤 3.2: 实现 Template CST ✅
- [x] 定义 `CstTemplateLiteral`, `CstTemplatePart`
- [x] 实现模板字符串解析
- [x] 处理变量插值
- [x] 编写测试

**已解决问题**:
- [x] nom 8.0 闭包生命周期问题（使用独立 helper 函数代替闭包）
- [x] 缺少 `take_while1` 导入问题

**测试覆盖**:
- [x] 引号字符串解析测试（双引号、单引号、转义字符）
- [x] 模板字符串解析测试（简单文本、变量插值）
- [x] 前导文本解析测试（简单、引号）
- [x] 后缀标记解析测试
- [x] 文本行解析测试（简单、带前导、带后缀）
- [x] CST→AST 转换测试

### Phase 4: 集成和优化（Week 7-8）✅ 已完成

#### 步骤 4.1: LSP 集成 ✅ 已完成
- [x] 修改 `sixu-lsp/Cargo.toml`，启用 `cst` feature
- [x] 创建 CST helper 模块（`cst_helper.rs`）
- [x] 更新 `document_symbol` 功能使用 CST
- [x] 更新 `formatting` 功能使用 CST
- [x] 用 CST parser 替换 `completion` 中的 scanner
- [x] 用 CST parser 替换 `hover` 中的 scanner
- [x] 用 CST parser 替换 `goto_definition` 中的 scanner
- [x] 删除 `scanner.rs` 文件
- [x] 测试所有 LSP 功能

**已实现功能**:
- completion: 命令和参数补全，使用 CST 提取段落信息
- hover: 命令和参数悬停提示，使用 CST 查找节点
- goto_definition: 系统调用跳转定义，使用 CST 解析参数
- document_symbol: 使用 `extract_paragraphs()` 提取段落信息
- formatting: 使用 `CstFormatter` 进行代码格式化
- validate: 使用 CST 进行语法和 Schema 校验

#### 步骤 4.2: Formatter 实现 ✅ 已完成
- [x] 实现 `CstFormatter` 结构体（`sixu/src/cst/formatter.rs`）
- [x] 实现格式化规则：
  - [x] 空格规范化（命令使用括号语法）
  - [x] 空白行规范化（多个空行缩减为一个）
  - [x] 缩进规范化（使用 4 空格）
  - [x] 注释保留（行注释和块注释）
- [x] 编写格式化测试（6 个测试，100% 通过）

**测试覆盖**:
- [x] 简单命令格式化
- [x] 段落格式化
- [x] 注释保留
- [x] 多段落格式化
- [x] 文本行格式化
- [x] 系统调用格式化

**格式化示例**：
```rust
impl CstRoot {
    pub fn format(&self) -> String {
        let mut output = String::new();
        
        for node in &self.nodes {
            match node {
                CstNode::Trivia(t) => {
                    // 规范化空白
                    output.push_str(&normalize_trivia(t));
                }
                CstNode::Command(cmd) => {
                    output.push_str(&format_command(cmd));
                }
                // ... 其他节点
            }
        }
        
        output
    }
}

fn normalize_trivia(t: &CstTrivia) -> String {
    match t {
        CstTrivia::Whitespace { content, .. } => {
            // 多个空格缩减为 1 个，多个换行缩减为最多 2 个
            let lines: Vec<&str> = content.split('\n').collect();
            if lines.len() > 2 {
                "\n\n".to_string()
            } else {
                content.clone()
            }
        }
        CstTrivia::LineComment { .. } | CstTrivia::BlockComment { .. } => {
            // 保留注释
            t.content().to_string()
        }
    }
}
```

#### 步骤 4.3: 性能优化
- [ ] Benchmark（对比 AST parser 性能）
- [ ] 优化热点路径
- [ ] 考虑引入 `rowan`（如果需要）

#### 步骤 4.4: 文档和示例
- [ ] 编写 API 文档
- [ ] 添加使用示例
- [ ] 更新 README

---

## 6. 注意事项

### 6.1 位置计算

**关键点**：`nom_locate` 使用字节偏移，需要正确转换为行列号。

```rust
use nom_locate::LocatedSpan;

pub fn span_to_position(span: Span) -> (usize, usize) {
    let line = span.location_line();     // 1-based
    let column = span.get_column();      // 1-based
    (line, column - 1)  // 转换为 0-based column
}
```

**陷阱**：
- UTF-8 多字节字符会导致字节偏移 ≠ 字符偏移
- 需要使用 `ropey` 或 `unicode-segmentation` 处理

### 6.2 Trivia 归属

**问题**：Trivia 应该属于哪个节点？

```rust
// 示例
// 注释1
@command1 arg=1  // 注释2
// 注释3
@command2 arg=2
```

**策略**：
- `// 注释1` → `@command1` 的 `leading_trivia`
- `// 注释2` → `@command1` 的 `trailing_trivia`（或下一个节点的 leading）
- `// 注释3` → `@command2` 的 `leading_trivia`

**实现建议**：
- 初期简单处理：所有 trivia 作为独立节点
- 后期优化：按规则归属到相邻节点

### 6.3 容错边界

**原则**：尽量多解析，但不要瞎猜。

**示例**：
```rust
// 可容错
@command arg=   // 缺少值，但可以标记为错误并继续
@command arg=1 arg2=  // 同上

// 不应容错（严重语法错误）
@   // 没有命令名，应该跳过整行
```

### 6.4 AST 兼容性

**要求**：CST → AST 转换必须生成与原 parser 相同的 AST。

**测试策略**：
```rust
#[test]
fn test_cst_ast_equivalence() {
    let input = r#"::test { @cmd arg=1 }"#;
    
    // AST parser
    let ast_result = sixu::parser::parse("test", input).unwrap();
    
    // CST parser + 转换
    let cst = sixu::cst::parse_tolerant(input);
    let cst_ast_result = cst.to_ast().unwrap();
    
    assert_eq!(ast_result, cst_ast_result);
}
```

### 6.5 内存占用

**问题**：CST 包含所有细节，内存占用可能是 AST 的 3-5 倍。

**缓解**：
- LSP 只缓存当前打开的文件
- Formatter 一次性处理，处理完即释放
- 考虑使用 `Arc<str>` 共享字符串

### 6.6 Unicode 处理

**重要**：Sixu 支持中文等 Unicode 字符。

```rust
// ❌ 错误
let char_offset = byte_offset;  // 假设 1 字节 = 1 字符

// ✅ 正确
use unicode_segmentation::UnicodeSegmentation;
let char_offset = input[..byte_offset].graphemes(true).count();
```

**建议**：使用 `ropey::Rope` 处理 UTF-8。

### 6.7 Feature 条件编译

**规则**：
- `#[cfg(feature = "cst")]` 用于模块级
- 不要在 `format.rs`（AST）中添加 CST 相关代码
- 转换逻辑在 `cst/convert.rs` 中

```rust
// ✅ 正确
#[cfg(feature = "cst")]
impl CstCommand {
    pub fn to_ast(&self) -> CommandLine { ... }
}

// ❌ 错误（不要污染 AST）
impl CommandLine {
    #[cfg(feature = "cst")]
    pub fn from_cst(cst: &CstCommand) -> Self { ... }
}
```

---

## 7. 测试策略

### 7.1 单元测试

**覆盖范围**：
- 每个 parser 函数
- 每个 to_ast 转换
- 边界条件（空字符串、只有注释、只有空白）

**示例**：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_command_empty_args() {
        let input = "@command()";
        let result = parse_command(Span::new(input)).unwrap();
        assert_eq!(result.1.arguments.len(), 0);
    }
    
    #[test]
    fn test_parse_command_with_comment() {
        let input = "@command /* comment */ arg=1";
        let result = parse_command(Span::new(input)).unwrap();
        // 验证注释被保留
    }
}
```

### 7.2 集成测试

**测试文件**：`sixu/tests/cst_integration.rs`

```rust
#[test]
fn test_parse_complete_file() {
    let input = include_str!("../sample-project/assets/scenarios/normal.sixu");
    let cst = sixu::cst::parse_tolerant(input);
    let ast = cst.to_ast().unwrap();
    
    // 验证 AST 正确性
    assert!(ast.paragraphs.len() > 0);
}
```

### 7.3 Snapshot 测试

**工具**：使用 `insta` crate

```toml
[dev-dependencies]
insta = "1"
```

```rust
#[test]
fn test_cst_structure() {
    let input = "@command arg=1";
    let cst = parse_command(Span::new(input)).unwrap();
    insta::assert_debug_snapshot!(cst);
}
```

### 7.4 Fuzzing（可选）

**工具**：`cargo fuzz`

```rust
// fuzz/fuzz_targets/cst_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = sixu::cst::parse_tolerant(s);
    }
});
```

### 7.5 性能测试

**工具**：`criterion`

```toml
[dev-dependencies]
criterion = "0.5"
```

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_cst_parse(c: &mut Criterion) {
    let input = include_str!("../sample-project/assets/scenarios/complex.sixu");
    
    c.bench_function("cst parse", |b| {
        b.iter(|| sixu::cst::parse_tolerant(black_box(input)))
    });
}

criterion_group!(benches, bench_cst_parse);
criterion_main!(benches);
```

---

## 8. 里程碑和验收标准

### Milestone 1: Command/SystemCall CST（2 周）
**验收标准**：
- [x] 能解析所有 Command 和 SystemCall 语法
- [x] 保留所有 trivia（空白、注释）
- [x] 能转换为正确的 AST
- [x] 单元测试覆盖率 > 80%
- [ ] 通过示例文件测试

**状态**: ✅ **已完成** (2025-12-27)

### Milestone 2: Complete CST（前 4 周）
**验收标准**：
- [x] 能解析 Command 和 SystemCall 语法
- [x] 能解析 Paragraph 和 Block 语法
- [x] 能解析 Text 和 Template 语法
- [x] 保留所有 trivia（空白、注释）
- [x] 能转换为正确的 AST
- [x] 单元测试覆盖率 > 80%
- [x] CST → AST 转换与原 parser 等价
- [ ] 容错解析能处理常见错误
- [ ] 集成测试覆盖所有示例文件

**状态**: ⚠️ **进行中** - Phase 1-3 已完成，等待 embedded code 实现（2025-12-27）

### Milestone 3: LSP 集成（8 周）
**验收标准**：
- [ ] 删除 `scanner.rs`，完全使用 CST
- [ ] 所有 LSP 功能正常工作
- [ ] 性能不低于原实现
- [ ] VS Code 插件功能完整

### Milestone 4: Formatter（10 周）
**验收标准**：
- [ ] 能格式化任意 .sixu 文件
- [ ] 格式化后语义不变（AST 等价）
- [ ] 保留所有注释
- [ ] 通过格式化测试套件

---

## 9. 未来扩展

### 9.1 可能的优化

- **Red-Green Tree**：使用 `rowan` 实现增量解析
- **Error Recovery**：更智能的错误恢复策略
- **Incremental Parsing**：只重新解析修改的部分（LSP）

### 9.2 可能的功能

- **Code Actions**：自动修复常见错误
- **Semantic Highlighting**：语义级别的语法高亮
- **Rename Refactoring**：重命名变量/段落
- **Extract to Paragraph**：代码重构

---

## 10. 参考资料

### 相关项目
- [rust-analyzer](https://github.com/rust-lang/rust-analyzer)：使用 `rowan` 的示例
- [tree-sitter](https://tree-sitter.github.io/)：另一种 CST 实现方式
- [rome/biome](https://github.com/biomejs/biome)：JS formatter，CST 设计参考

### 技术文档
- [nom_locate 文档](https://docs.rs/nom_locate/)
- [rowan 文档](https://docs.rs/rowan/)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/)

---

## 附录 A: 完整示例

### 示例输入

```sixu
// 故事开始
::start {
    @changebg src="bg.jpg" fadeTime=1000
    
    /* 显示角色 */
    @addchar(name="hero", src="hero.png", x=100, y=200)
    
    [hero] "你好！"  // 第一句话
    
    #goto next
}

::next {
    @wait time=500
    [hero] "再见！"
}
```

### 期望的 CST 结构（简化）

```rust
CstRoot {
    nodes: [
        Trivia(LineComment { content: " 故事开始" }),
        Paragraph(CstParagraph {
            name: "start",
            block: CstBlock {
                children: [
                    Command(CstCommand {
                        command: "changebg",
                        arguments: [
                            CstArgument { name: "src", value: Some("bg.jpg") },
                            CstArgument { name: "fadeTime", value: Some(1000) },
                        ],
                        syntax: SpaceSeparated,
                    }),
                    Trivia(Whitespace),
                    Trivia(BlockComment { content: " 显示角色 " }),
                    Command(CstCommand {
                        command: "addchar",
                        syntax: Parenthesized,
                        ...
                    }),
                    TextLine(CstTextLine {
                        leading: Some("[hero]"),
                        text: Some("你好！"),
                    }),
                    Trivia(LineComment { content: " 第一句话" }),
                    SystemCall(CstSystemCall {
                        command: "goto",
                        arguments: [
                            CstArgument { name: "paragraph", value: Some("next") },
                        ],
                    }),
                ],
            },
        }),
        Paragraph(CstParagraph { name: "next", ... }),
    ],
}
```

---

## 修订历史

| 版本 | 日期 | 修订内容 |
|------|------|----------|
| v1.0 | 2025-12-27 | 初始版本 |
| v1.1 | 2025-12-27 | Phase 1 完成：Command 和 SystemCall CST 实现 |
| v1.2 | 2025-12-27 | Phase 2 完成：Paragraph 和 Block CST 实现 |
| v1.3 | 2025-12-27 | Phase 3 完成：Text 和 Template CST 实现，所有32个测试通过 |
| v1.4 | 2025-12-27 | Phase 4 完成：Formatter + LSP 完整集成，删除 scanner.rs |

---

## 当前进度

### ✅ Phase 1-4 完成
- ✅ 32 个 CST 测试全部通过
- ✅ 6 个 Formatter 测试全部通过
- ✅ 2 个集成测试全部通过
- ✅ 77 个总测试全部通过

### ✅ LSP 完全迁移到 CST
- ✅ 删除 scanner.rs（~350 行代码）
- ✅ 所有 LSP 功能使用 CST：
  - completion（命令和参数补全）
  - hover（悬停提示）
  - goto_definition（跳转定义）
  - document_symbol（文档符号）
  - formatting（代码格式化）
  - validate（语法和 Schema 校验）

### ✅ Formatter 功能完整
- 注释保留（行注释和块注释）
- 空格和空行规范化
- 缩进规范化（4 空格）
- 支持所有节点类型
- 段落间自动空行

### 📋 待办事项
- [ ] Phase 5: 增强容错和错误恢复
- [ ] Phase 6: 性能优化和文档完善
- [ ] 端到端 LSP 测试（在实际 VS Code 环境中测试）

