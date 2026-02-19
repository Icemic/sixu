//! CST node definitions

use crate::format;
use super::span::SpanInfo;

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

impl CstRoot {
    /// 转换为 AST Story
    pub fn to_ast(&self) -> crate::error::Result<crate::format::Story> {
        let mut paragraphs = Vec::new();
        
        for node in &self.nodes {
            if let CstNode::Paragraph(para) = node {
                paragraphs.push(para.to_ast()?);
            }
        }
        
        Ok(crate::format::Story {
            name: self.name.clone(),
            paragraphs,
        })
    }
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
    
    /// 属性（如 #[cond(...)], #[while(...)], #[loop]）
    Attribute(CstAttribute),
    
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
            Self::Attribute(a) => a.span,
            Self::Error { span, .. } => *span,
        }
    }
}

/// 属性节点 #[keyword(condition)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstAttribute {
    /// 属性关键字（cond, if, while, loop 等）
    pub keyword: String,
    
    /// 关键字的位置
    pub keyword_span: SpanInfo,
    
    /// 条件表达式（如果有）
    pub condition: Option<String>,
    
    /// 条件表达式的位置（如果有）
    pub condition_span: Option<SpanInfo>,
    
    /// #[ 的位置
    pub open_token: SpanInfo,
    
    /// ] 的位置
    pub close_token: SpanInfo,
    
    /// 整个属性的范围
    pub span: SpanInfo,
    
    /// 前导 trivia
    pub leading_trivia: Vec<CstTrivia>,
}

impl CstAttribute {
    /// 转换为 AST Attribute
    pub fn to_ast(&self) -> format::Attribute {
        format::Attribute {
            keyword: self.keyword.clone(),
            condition: self.condition.clone(),
        }
    }
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

impl CstCommand {
    /// 转换为 AST CommandLine
    pub fn to_ast(&self) -> format::CommandLine {
        format::CommandLine {
            command: self.command.clone(),
            arguments: self.arguments.iter().map(|a| a.to_ast()).collect(),
        }
    }
}

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
    pub syntax: CommandSyntax,
    
    /// 整个调用的范围
    pub span: SpanInfo,
    
    /// 前导 trivia
    pub leading_trivia: Vec<CstTrivia>,
}

impl CstSystemCall {
    /// 转换为 AST SystemCallLine
    pub fn to_ast(&self) -> format::SystemCallLine {
        format::SystemCallLine {
            command: self.command.clone(),
            arguments: self.arguments.iter().map(|a| a.to_ast()).collect(),
        }
    }
}

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
    pub trailing_trivia: Vec<CstTrivia>,
}

impl CstArgument {
    /// 转换为 AST Argument
    pub fn to_ast(&self) -> format::Argument {
        format::Argument {
            name: self.name.clone(),
            value: self.value
                .as_ref()
                .map(|v| v.to_ast())
                .unwrap_or(format::RValue::Literal(format::Literal::Boolean(true))),
        }
    }
}

/// 引号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QuoteStyle {
    Double,  // "
    Single,  // '
    Backtick, // `
}

/// 值的种类
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
}

/// 值节点（字符串、数字、变量等）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstValue {
    /// 值的种类
    pub kind: CstValueKind,
    
    /// 原始文本（含引号、前缀等）
    pub raw: String,
    
    /// 解析后的值（用于生成 AST）
    pub parsed: format::RValue,
    
    /// 值的位置
    pub span: SpanInfo,
}

impl CstValue {
    /// 转换为 AST RValue
    pub fn to_ast(&self) -> format::RValue {
        self.parsed.clone()
    }
}

// ===== Phase 2-4 的节点（暂时使用占位定义） =====

/// 段落节点 ::paragraph_name(param1, param2="default") { ... }
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstParagraph {
    /// 段落名
    pub name: String,
    
    /// :: 符号的位置
    pub colon_token: SpanInfo,
    
    /// 段落名的位置
    pub name_span: SpanInfo,
    
    /// 参数列表（可选）
    pub parameters: Vec<CstParameter>,
    
    /// ( 的位置（如果有参数）
    pub open_paren: Option<SpanInfo>,
    
    /// ) 的位置（如果有参数）
    pub close_paren: Option<SpanInfo>,
    
    /// 段落体
    pub block: CstBlock,
    
    /// 整个段落的范围
    pub span: SpanInfo,
    
    /// 前导 trivia
    pub leading_trivia: Vec<CstTrivia>,
}

impl CstParagraph {
    pub fn to_ast(&self) -> crate::error::Result<format::Paragraph> {
        Ok(format::Paragraph {
            name: self.name.clone(),
            parameters: self.parameters.iter().map(|p| p.to_ast()).collect(),
            block: self.block.to_ast()?,
        })
    }
}

/// 段落参数 param1, param2="default"
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstParameter {
    /// 参数名
    pub name: String,
    
    /// 参数名的位置
    pub name_span: SpanInfo,
    
    /// = 的位置（如果有默认值）
    pub equals_token: Option<SpanInfo>,
    
    /// 默认值（可选）
    pub default_value: Option<CstValue>,
    
    /// 整个参数的范围
    pub span: SpanInfo,
    
    /// 前导 trivia
    pub leading_trivia: Vec<CstTrivia>,
    
    /// 尾随 trivia（逗号、空白等）
    pub trailing_trivia: Vec<CstTrivia>,
}

impl CstParameter {
    pub fn to_ast(&self) -> format::Parameter {
        format::Parameter {
            name: self.name.clone(),
            default_value: self.default_value.as_ref().and_then(|v| {
                match &v.parsed {
                    format::RValue::Literal(lit) => Some(lit.clone()),
                    _ => None,
                }
            }),
        }
    }
}

/// 代码块（Phase 2）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstBlock {
    pub open_brace: SpanInfo,
    pub children: Vec<CstNode>,
    pub close_brace: SpanInfo,
    pub span: SpanInfo,
}

impl CstBlock {
    pub fn to_ast(&self) -> crate::error::Result<format::Block> {
        let mut children = Vec::new();
        let mut pending_attributes: Vec<format::Attribute> = Vec::new();
        
        for node in &self.children {
            match node {
                CstNode::Attribute(attr) => {
                    pending_attributes.push(attr.to_ast());
                }
                CstNode::Command(cmd) => {
                    children.push(format::Child {
                        attributes: std::mem::take(&mut pending_attributes),
                        content: format::ChildContent::CommandLine(cmd.to_ast()),
                    });
                }
                CstNode::SystemCall(sc) => {
                    children.push(format::Child {
                        attributes: std::mem::take(&mut pending_attributes),
                        content: format::ChildContent::SystemCallLine(sc.to_ast()),
                    });
                }
                CstNode::TextLine(tl) => {
                    let mut child = tl.to_ast()?;
                    child.attributes = std::mem::take(&mut pending_attributes);
                    children.push(child);
                }
                CstNode::Block(b) => {
                    children.push(format::Child {
                        attributes: std::mem::take(&mut pending_attributes),
                        content: format::ChildContent::Block(b.to_ast()?),
                    });
                }
                CstNode::EmbeddedCode(ec) => {
                    children.push(format::Child {
                        attributes: std::mem::take(&mut pending_attributes),
                        content: format::ChildContent::EmbeddedCode(ec.code.clone()),
                    });
                }
                CstNode::Trivia(_) => {
                    // Trivia 不转换到 AST
                }
                CstNode::Paragraph(_) => {
                    // Paragraph 不应该在 block 内
                }
                CstNode::Error { .. } => {
                    // 错误节点跳过
                }
            }
        }
        
        Ok(format::Block { children })
    }
}

/// 文本行 [leading] text #tailing
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstTextLine {
    /// 前导文本（如 [角色名]）
    pub leading: Option<CstLeadingText>,
    
    /// 主文本内容
    pub text: Option<CstText>,
    
    /// 后缀标记（如 #wait）
    pub tailing: Option<CstTailingText>,
    
    /// 整行的范围
    pub span: SpanInfo,
    
    /// 前导 trivia
    pub leading_trivia: Vec<CstTrivia>,
}

impl CstTextLine {
    pub fn to_ast(&self) -> crate::error::Result<format::Child> {
        let leading_ast = match &self.leading {
            Some(l) => l.to_ast(),
            None => format::LeadingText::None,
        };
        
        let text_ast = match &self.text {
            Some(t) => t.to_ast()?,
            None => format::Text::None,
        };
        
        let tailing_ast = match &self.tailing {
            Some(t) => t.to_ast(),
            None => format::TailingText::None,
        };
        
        Ok(format::Child {
            attributes: vec![],
            content: format::ChildContent::TextLine(leading_ast, text_ast, tailing_ast),
        })
    }
}

/// 前导文本 [...]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstLeadingText {
    /// [ 的位置
    pub open_bracket: SpanInfo,
    
    /// 前导文本内容（可以是字符串或模板）
    pub content: CstLeadingTextContent,
    
    /// ] 的位置
    pub close_bracket: SpanInfo,
    
    /// 整个前导文本的范围
    pub span: SpanInfo,
}

impl CstLeadingText {
    pub fn to_ast(&self) -> format::LeadingText {
        match &self.content {
            CstLeadingTextContent::Text(text) => format::LeadingText::Text(text.clone()),
            CstLeadingTextContent::Template(tpl) => {
                // 将 CST template 转为 AST template
                format::LeadingText::TemplateLiteral(tpl.to_ast())
            }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CstLeadingTextContent {
    /// 普通文本或带引号的文本
    Text(String),
    /// 模板字符串
    Template(CstTemplateLiteral),
}

/// 主文本内容
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstText {
    /// 文本种类
    pub kind: CstTextKind,
    
    /// 原始文本（含引号等）
    pub raw: String,
    
    /// 解析后的文本（用于生成 AST）
    pub parsed: String,
    
    /// 文本位置
    pub span: SpanInfo,
}

impl CstText {
    pub fn to_ast(&self) -> crate::error::Result<format::Text> {
        Ok(match &self.kind {
            CstTextKind::Bare => format::Text::Text(self.parsed.clone()),
            CstTextKind::Quoted(_) => format::Text::Text(self.parsed.clone()),
            CstTextKind::Template(tpl) => format::Text::TemplateLiteral(tpl.to_ast()),
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CstTextKind {
    /// 裸文本（不转义）
    Bare,
    /// 带引号的文本（支持转义）
    Quoted(QuoteStyle),
    /// 模板字符串
    Template(CstTemplateLiteral),
}

/// 后缀标记 #wait
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstTailingText {
    /// # 的位置
    pub hash_token: SpanInfo,
    
    /// 标记名
    pub marker: String,
    
    /// 标记名的位置
    pub marker_span: SpanInfo,
    
    /// 整个标记的范围
    pub span: SpanInfo,
}

impl CstTailingText {
    pub fn to_ast(&self) -> format::TailingText {
        format::TailingText::Text(self.marker.clone())
    }
}

/// 模板字符串 `text ${var}`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstTemplateLiteral {
    /// 模板的各个部分
    pub parts: Vec<CstTemplatePart>,
    
    /// 整个模板的范围
    pub span: SpanInfo,
}

impl CstTemplateLiteral {
    pub fn to_ast(&self) -> format::TemplateLiteral {
        let parts = self.parts.iter().map(|p| p.to_ast()).collect();
        format::TemplateLiteral { parts }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CstTemplatePart {
    /// 文本部分
    Text {
        content: String,
        span: SpanInfo,
    },
    /// 变量插值 ${...}
    Value {
        /// ${ 的位置
        open_token: SpanInfo,
        /// 变量
        variable: format::Variable,
        /// 变量的位置
        variable_span: SpanInfo,
        /// } 的位置
        close_token: SpanInfo,
        /// 整个插值的范围
        span: SpanInfo,
    },
}

impl CstTemplatePart {
    pub fn to_ast(&self) -> format::TemplateLiteralPart {
        match self {
            CstTemplatePart::Text { content, .. } => {
                format::TemplateLiteralPart::Text(content.clone())
            }
            CstTemplatePart::Value { variable, .. } => {
                format::TemplateLiteralPart::Value(format::RValue::Variable(variable.clone()))
            }
        }
    }
}

/// 嵌入代码语法风格
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EmbeddedCodeSyntax {
    Brace,  // @{ ... }
    Hash,   // ## ... ##
}

/// 嵌入代码节点
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstEmbeddedCode {
    pub syntax: EmbeddedCodeSyntax,
    pub code: String,
    pub span: SpanInfo,
}

impl CstEmbeddedCode {
    pub fn to_ast(&self) -> crate::error::Result<format::Child> {
        Ok(format::Child {
            attributes: vec![],
            content: format::ChildContent::EmbeddedCode(self.code.clone()),
        })
    }
}

