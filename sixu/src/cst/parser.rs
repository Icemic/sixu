//! CST parser with error tolerance

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while, take_while1},
    character::complete::{
        alpha1, alphanumeric1, char, digit1, multispace1, one_of, space0, space1,
    },
    combinator::{opt, recognize, value},
    multi::{many0, many1, many_till, separated_list0},
    sequence::{delimited, pair, preceded},
    IResult, Parser,
};

use super::node::*;
use super::span::{Span, SpanInfo};
use crate::format;

type ParseResult<'a, T> = IResult<Span<'a>, T>;

/// 容错解析入口
pub fn parse_tolerant(name: &str, input: &str) -> CstRoot {
    let span = Span::new(input);
    let start_info = SpanInfo::from_span(span);

    let mut nodes = Vec::new();
    let mut remaining = span;

    while !remaining.fragment().is_empty() {
        // 尝试解析 trivia
        if let Ok((rest, trivia)) = parse_trivia(remaining) {
            nodes.push(CstNode::Trivia(trivia));
            remaining = rest;
            continue;
        }

        // 尝试解析段落
        if let Ok((rest, para)) = parse_paragraph(remaining) {
            nodes.push(CstNode::Paragraph(para));
            remaining = rest;
            continue;
        }

        // 尝试解析命令
        if let Ok((rest, cmd)) = parse_command(remaining) {
            nodes.push(CstNode::Command(cmd));
            remaining = rest;
            continue;
        }

        // 尝试解析系统调用
        if let Ok((rest, sc)) = parse_systemcall(remaining) {
            nodes.push(CstNode::SystemCall(sc));
            remaining = rest;
            continue;
        }

        // 容错：跳过一个字符
        if let Ok((rest, _)) = take::<usize, Span, nom::error::Error<Span>>(1usize)(remaining) {
            remaining = rest;
        } else {
            break;
        }
    }

    let _end_span = remaining;
    let end_info = if !remaining.fragment().is_empty() {
        SpanInfo::from_span(remaining)
    } else {
        start_info
    };

    CstRoot {
        name: name.to_string(),
        nodes,
        span: SpanInfo {
            start: start_info.start,
            end: end_info.end,
            start_line: start_info.start_line,
            start_column: start_info.start_column,
            end_line: end_info.end_line,
            end_column: end_info.end_column,
        },
    }
}

/// 解析 trivia（空白或注释）
fn parse_trivia(input: Span) -> ParseResult<CstTrivia> {
    alt((parse_line_comment, parse_block_comment, parse_whitespace)).parse(input)
}

/// 解析空白
fn parse_whitespace(input: Span) -> ParseResult<CstTrivia> {
    let start_span = input;
    let (input, ws) = multispace1(input)?;
    let end_span = input;

    Ok((
        input,
        CstTrivia::Whitespace {
            content: ws.fragment().to_string(),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析单行注释 // ...
fn parse_line_comment(input: Span) -> ParseResult<CstTrivia> {
    let start_span = input;
    let (input, _) = tag("//")(input)?;
    let (input, content) = take_while(|c| c != '\n' && c != '\r')(input)?;
    let end_span = input;

    Ok((
        input,
        CstTrivia::LineComment {
            content: content.fragment().to_string(),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析块注释 /* ... */
fn parse_block_comment(input: Span) -> ParseResult<CstTrivia> {
    let start_span = input;
    let (input, _) = tag("/*")(input)?;
    let (input, content) = take_until("*/")(input)?;
    let (input, _) = tag("*/")(input)?;
    let end_span = input;

    Ok((
        input,
        CstTrivia::BlockComment {
            content: content.fragment().to_string(),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析标识符
fn parse_identifier(input: Span) -> ParseResult<(String, SpanInfo)> {
    let start_span = input;
    let (input, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;
    let end_span = input;

    Ok((
        input,
        (
            name.fragment().to_string(),
            SpanInfo::from_range(start_span, end_span),
        ),
    ))
}

/// 解析命令 @command arg1=val1 arg2
pub fn parse_command(input: Span) -> ParseResult<CstCommand> {
    let start_span = input;

    // 收集前导 trivia
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // @ 符号
    let at_start = input;
    let (input, _) = tag("@")(input)?;
    let at_token = SpanInfo::from_span_and_len(at_start, 1);

    // 命令名
    let (input, (command, name_span)) = parse_identifier(input)?;

    // 解析参数（支持两种语法）
    let (input, (arguments, syntax)) = alt((
        parse_arguments_parenthesized,
        parse_arguments_space_separated,
    ))
    .parse(input)?;

    let end_span = input;

    Ok((
        input,
        CstCommand {
            command,
            at_token,
            name_span,
            arguments,
            syntax,
            span: SpanInfo::from_range(start_span, end_span),
            leading_trivia,
        },
    ))
}

/// 解析系统调用 #goto paragraph="main"
pub fn parse_systemcall(input: Span) -> ParseResult<CstSystemCall> {
    let start_span = input;

    // 收集前导 trivia
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // # 符号
    let hash_start = input;
    let (input, _) = tag("#")(input)?;
    let hash_token = SpanInfo::from_span_and_len(hash_start, 1);

    // 命令名
    let (input, (command, name_span)) = parse_identifier(input)?;

    // 解析参数（支持两种语法）
    let (input, (arguments, syntax)) = alt((
        parse_arguments_parenthesized,
        parse_arguments_space_separated,
    ))
    .parse(input)?;

    let end_span = input;

    Ok((
        input,
        CstSystemCall {
            command,
            hash_token,
            name_span,
            arguments,
            syntax,
            span: SpanInfo::from_range(start_span, end_span),
            leading_trivia,
        },
    ))
}

/// 解析括号风格的参数 (arg1=val1, arg2=val2)
fn parse_arguments_parenthesized(input: Span) -> ParseResult<(Vec<CstArgument>, CommandSyntax)> {
    let (input, _) = space0(input)?;
    let open_start = input;
    let (input, _) = tag("(")(input)?;
    let open_paren = SpanInfo::from_span_and_len(open_start, 1);

    let (input, _) = space0(input)?;
    let (input, arguments) =
        separated_list0(delimited(space0, tag(","), space0), parse_argument).parse(input)?;
    let (input, _) = space0(input)?;

    let close_start = input;
    let (input, _) = tag(")")(input)?;
    let close_paren = SpanInfo::from_span_and_len(close_start, 1);

    Ok((
        input,
        (
            arguments,
            CommandSyntax::Parenthesized {
                open_paren,
                close_paren,
            },
        ),
    ))
}

/// 解析空格分隔的参数 arg1=val1 arg2=val2
fn parse_arguments_space_separated(input: Span) -> ParseResult<(Vec<CstArgument>, CommandSyntax)> {
    let (input, arguments) = many0(preceded(space1, parse_argument)).parse(input)?;
    Ok((input, (arguments, CommandSyntax::SpaceSeparated)))
}

/// 解析单个参数 name=value 或 flag
fn parse_argument(input: Span) -> ParseResult<CstArgument> {
    let start_span = input;

    // 前导 trivia（在逗号后的空白）
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // 参数名
    let (input, (name, name_span)) = parse_identifier(input)?;

    // 可选的 = 和值
    let (input, equals_and_value) =
        opt((preceded(space0, tag("=")), preceded(space0, parse_value))).parse(input)?;

    let (equals_token, value) = if let Some((eq, val)) = equals_and_value {
        let eq_span = SpanInfo::from_span_and_len(Span::new(eq.fragment()), 1);
        (Some(eq_span), Some(val))
    } else {
        (None, None)
    };

    let end_span = input;

    Ok((
        input,
        CstArgument {
            name,
            name_span,
            equals_token,
            value,
            span: SpanInfo::from_range(start_span, end_span),
            leading_trivia,
            trailing_trivia: vec![],
        },
    ))
}

/// 解析值
fn parse_value(input: Span) -> ParseResult<CstValue> {
    alt((
        parse_string_value,
        parse_template_string_value,
        parse_number_value,
        parse_boolean_value,
        parse_variable_value,
    ))
    .parse(input)
}

/// 解析字符串值 "..." 或 '...'
fn parse_string_value(input: Span) -> ParseResult<CstValue> {
    let start_span = input;

    let (input, quote_char) = alt((char('"'), char('\''))).parse(input)?;
    let quote_style = if quote_char == '"' {
        QuoteStyle::Double
    } else {
        QuoteStyle::Single
    };

    // 简化实现：暂不处理转义
    let (input, content) = take_while(move |c| c != quote_char)(input)?;
    let (input, _) = char(quote_char)(input)?;

    let end_span = input;
    let raw = format!("{}{}{}", quote_char, content.fragment(), quote_char);

    Ok((
        input,
        CstValue {
            kind: CstValueKind::String { quote: quote_style },
            raw: raw.clone(),
            parsed: format::RValue::Literal(format::Literal::String(
                content.fragment().to_string(),
            )),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析模板字符串 `...`
fn parse_template_string_value(input: Span) -> ParseResult<CstValue> {
    let start_span = input;

    let (input, _) = char('`')(input)?;
    let (input, content) = take_while(|c| c != '`')(input)?;
    let (input, _) = char('`')(input)?;

    let end_span = input;
    let raw = format!("`{}`", content.fragment());

    // 简化实现：暂不解析模板变量
    Ok((
        input,
        CstValue {
            kind: CstValueKind::TemplateString,
            raw: raw.clone(),
            parsed: format::RValue::Literal(format::Literal::String(
                content.fragment().to_string(),
            )),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析数字值
fn parse_number_value(input: Span) -> ParseResult<CstValue> {
    let start_span = input;

    let (input, number_str) =
        recognize((opt(char('-')), digit1, opt((char('.'), digit1)))).parse(input)?;

    let end_span = input;
    let raw = number_str.fragment().to_string();

    let parsed = if raw.contains('.') {
        // 浮点数
        format::RValue::Literal(format::Literal::Float(raw.parse::<f64>().unwrap_or(0.0)))
    } else {
        // 整数
        format::RValue::Literal(format::Literal::Integer(raw.parse::<i64>().unwrap_or(0)))
    };

    let kind = if raw.contains('.') {
        CstValueKind::Float
    } else {
        CstValueKind::Integer
    };

    Ok((
        input,
        CstValue {
            kind,
            raw,
            parsed,
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析布尔值
fn parse_boolean_value(input: Span) -> ParseResult<CstValue> {
    let start_span = input;

    let (input, bool_str) = alt((tag("true"), tag("false"))).parse(input)?;
    let end_span = input;

    let raw = bool_str.fragment().to_string();
    let value = raw == "true";

    Ok((
        input,
        CstValue {
            kind: CstValueKind::Boolean,
            raw,
            parsed: format::RValue::Literal(format::Literal::Boolean(value)),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析变量引用 foo.bar.baz
fn parse_variable_value(input: Span) -> ParseResult<CstValue> {
    let start_span = input;

    let (input, var_str) =
        recognize(many1(alt((alphanumeric1, tag("."), tag("_"))))).parse(input)?;

    let end_span = input;
    let raw = var_str.fragment().to_string();

    // 解析为变量链
    let chain: Vec<String> = raw.split('.').map(|s| s.to_string()).collect();

    Ok((
        input,
        CstValue {
            kind: CstValueKind::Variable,
            raw,
            parsed: format::RValue::Variable(format::Variable { chain }),
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析段落 ::paragraph_name(param1, param2="default") { ... }
pub fn parse_paragraph(input: Span) -> ParseResult<CstParagraph> {
    let start_span = input;
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // 解析 ::
    let colon_start = input;
    let (input, _) = tag("::").parse(input)?;
    let colon_span = SpanInfo::from_span_and_len(colon_start, 2);

    // 解析段落名
    let name_start = input;
    let (input, (name, _)) = parse_identifier(input)?;
    let name_end = input;
    let name_span = SpanInfo::from_range(name_start, name_end);

    // 解析可选的参数列表
    let (input, params_opt) = opt(parse_parameters).parse(input)?;
    let (open_paren, parameters, close_paren) = match params_opt {
        Some((op, ps, cp)) => (Some(op), ps, Some(cp)),
        None => (None, vec![], None),
    };

    // 解析空白
    let (input, _) = space0(input)?;

    // 解析块
    let (input, block) = parse_block(input)?;

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstParagraph {
            name: name.clone(),
            colon_token: colon_span,
            name_span,
            parameters,
            open_paren,
            close_paren,
            block,
            span,
            leading_trivia,
        },
    ))
}

/// 解析参数列表 (param1, param2="default")
fn parse_parameters(input: Span) -> ParseResult<(SpanInfo, Vec<CstParameter>, SpanInfo)> {
    let (input, _) = space0(input)?;

    let open_paren_start = input;
    let (input, _) = char('(').parse(input)?;
    let open_paren_span = SpanInfo::from_span_and_len(open_paren_start, 1);

    let (input, _) = space0(input)?;

    // 解析参数列表
    let (input, parameters) =
        separated_list0(delimited(space0, char(','), space0), parse_parameter).parse(input)?;

    let (input, _) = space0(input)?;

    let close_paren_start = input;
    let (input, _) = char(')').parse(input)?;
    let close_paren_span = SpanInfo::from_span_and_len(close_paren_start, 1);

    Ok((input, (open_paren_span, parameters, close_paren_span)))
}

/// 解析单个参数 param1 或 param2="default"
fn parse_parameter(input: Span) -> ParseResult<CstParameter> {
    let start_span = input;
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // 解析参数名
    let name_start = input;
    let (input, (name, _)) = parse_identifier(input)?;
    let name_end = input;
    let name_span = SpanInfo::from_range(name_start, name_end);

    // 解析可选的默认值
    let (input, opt_default) = opt(preceded(
        space0,
        pair(parse_equals_token, preceded(space0, parse_value)),
    ))
    .parse(input)?;

    let (equals_token, default_value) = match opt_default {
        Some((eq, val)) => (Some(eq), Some(val)),
        None => (None, None),
    };

    let (input, trailing_trivia) = many0(parse_trivia).parse(input)?;

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstParameter {
            name: name.clone(),
            name_span,
            equals_token,
            default_value,
            span,
            leading_trivia,
            trailing_trivia,
        },
    ))
}

/// 解析等号（用于参数默认值）
fn parse_equals_token(input: Span) -> ParseResult<SpanInfo> {
    let eq_start = input;
    let (input, _) = char('=').parse(input)?;
    Ok((input, SpanInfo::from_span_and_len(eq_start, 1)))
}

/// 解析属性 #[keyword(condition)] 或 #[keyword]
fn parse_cst_attribute(input: Span) -> ParseResult<CstAttribute> {
    let start_span = input;

    // 收集前导 trivia（如空白）
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // 解析 #[
    let open_start = input;
    let (input, _) = tag("#[").parse(input)?;
    let open_token = SpanInfo::from_span_and_len(open_start, 2);

    // 跳过空白
    let (input, _) = space0(input)?;

    // 解析关键字（identifier）
    let keyword_start = input;
    let (input, keyword) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;
    let keyword_str = keyword.fragment().to_string();
    let keyword_span = SpanInfo::from_range(keyword_start, input);

    // 跳过空白
    let (input, _) = space0(input)?;

    // 尝试解析条件：条件必须是括号内的带引号字符串
    // 例如 #[cond("x > 10")] 或 #[cond('counter < 3')]
    let (input, condition, condition_span) = if input.fragment().starts_with('(') {
        let (input, _) = char('(').parse(input)?;
        let (input, _) = space0(input)?;

        // 解析带引号的字符串
        let cond_start = input;
        let quote_char = input.fragment().chars().next().ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Char))
        })?;
        if quote_char != '"' && quote_char != '\'' {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Char,
            )));
        }
        let (input, _) = char(quote_char).parse(input)?;
        let (input, condition_content) = take_while(move |c| c != quote_char)(input)?;
        let condition_str = condition_content.fragment().to_string();
        let (input, _) = char(quote_char).parse(input)?;
        let cond_span = SpanInfo::from_range(cond_start, input);

        let (input, _) = space0(input)?;
        let (input, _) = char(')').parse(input)?;

        (input, Some(condition_str), Some(cond_span))
    } else {
        (input, None, None)
    };

    // 跳过空白
    let (input, _) = space0(input)?;

    // 解析 ]
    let close_start = input;
    let (input, _) = char(']').parse(input)?;
    let close_token = SpanInfo::from_span_and_len(close_start, 1);

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstAttribute {
            keyword: keyword_str,
            keyword_span,
            condition,
            condition_span,
            open_token,
            close_token,
            span,
            leading_trivia,
        },
    ))
}

/// 解析块 { ... }
pub fn parse_block(input: Span) -> ParseResult<CstBlock> {
    let start_span = input;

    // 解析 {
    let open_brace_start = input;
    let (input, _) = char('{').parse(input)?;
    let open_brace_span = SpanInfo::from_span_and_len(open_brace_start, 1);

    // 解析块内容
    let (input, children) = parse_block_children(input)?;

    // 解析 }
    let close_brace_start = input;
    let (input, _) = char('}').parse(input)?;
    let close_brace_span = SpanInfo::from_span_and_len(close_brace_start, 1);

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstBlock {
            open_brace: open_brace_span,
            children,
            close_brace: close_brace_span,
            span,
        },
    ))
}

/// 解析块内的子节点
fn parse_block_children(input: Span) -> ParseResult<Vec<CstNode>> {
    let mut nodes = Vec::new();
    let mut remaining = input;

    while !remaining.fragment().is_empty() {
        // 检查是否到达块结束（需要在解析 trivia 之前检查，以便正确处理 } 前的空白）
        let trimmed = remaining.fragment().trim_start();
        if trimmed.starts_with('}') || trimmed.is_empty() {
            // 如果有前导空白，先收集它
            while let Ok((rest, trivia)) = parse_trivia(remaining) {
                nodes.push(CstNode::Trivia(trivia));
                remaining = rest;
            }
            break;
        }

        // 尝试解析 trivia
        if let Ok((rest, trivia)) = parse_trivia(remaining) {
            nodes.push(CstNode::Trivia(trivia));
            remaining = rest;
            continue;
        }

        // 尝试解析内容（按照 AST parser 的顺序）
        // 先尝试嵌入代码（需要在命令和系统调用之前，因为 @{ 和 @ 都以 @ 开头）
        if let Ok((rest, code)) = parse_embedded_code(remaining) {
            nodes.push(CstNode::EmbeddedCode(code));
            remaining = rest;
            continue;
        }

        // 尝试解析嵌套块（在命令之前，避免 { 被误判）
        if let Ok((rest, block)) = parse_block(remaining) {
            nodes.push(CstNode::Block(block));
            remaining = rest;
            continue;
        }

        // 检查是否看起来像命令、系统调用或属性
        let trimmed = remaining.fragment().trim_start();
        let looks_like_command = trimmed.starts_with('@') && !trimmed.starts_with("@{");
        let looks_like_attribute = trimmed.starts_with("#[");
        let looks_like_systemcall = trimmed.starts_with('#') && !looks_like_attribute;

        // 尝试解析属性（在系统调用之前，因为 #[ 和 # 都以 # 开头）
        if looks_like_attribute {
            match parse_cst_attribute(remaining) {
                Ok((rest, attr)) => {
                    nodes.push(CstNode::Attribute(attr));
                    remaining = rest;
                    continue;
                }
                Err(_) => {
                    // 属性解析失败，回退到系统调用解析
                }
            }
        }

        // 尝试解析命令
        if looks_like_command {
            match parse_command(remaining) {
                Ok((rest, cmd)) => {
                    nodes.push(CstNode::Command(cmd));
                    remaining = rest;
                    continue;
                }
                Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                    // 命令语法错误，创建 Error 节点
                    let start_span = remaining;
                    // 简单地读取到行尾（查找换行符或到字符串末尾）
                    let content = remaining.fragment();
                    let line_end = content.find('\n').unwrap_or(content.len());
                    let line_content = &content[..line_end];

                    // 前进到行尾后（包括换行符）
                    let bytes_to_skip = line_end + if line_end < content.len() { 1 } else { 0 };
                    let (rest, _) =
                        take::<usize, Span, nom::error::Error<Span>>(bytes_to_skip)(remaining)
                            .unwrap_or((remaining, remaining));

                    nodes.push(CstNode::Error {
                        content: line_content.to_string(),
                        span: SpanInfo::from_range(start_span, rest),
                        message: format!("Invalid command syntax: {:?}", e.code),
                    });

                    remaining = rest;
                    continue;
                }
                _ => {}
            }
        }

        // 尝试解析系统调用
        if looks_like_systemcall {
            match parse_systemcall(remaining) {
                Ok((rest, sc)) => {
                    nodes.push(CstNode::SystemCall(sc));
                    remaining = rest;
                    continue;
                }
                Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                    // 系统调用语法错误，创建 Error 节点
                    let start_span = remaining;
                    // 简单地读取到行尾
                    let content = remaining.fragment();
                    let line_end = content.find('\n').unwrap_or(content.len());
                    let line_content = &content[..line_end];

                    // 前进到行尾后（包括换行符）
                    let bytes_to_skip = line_end + if line_end < content.len() { 1 } else { 0 };
                    let (rest, _) =
                        take::<usize, Span, nom::error::Error<Span>>(bytes_to_skip)(remaining)
                            .unwrap_or((remaining, remaining));

                    nodes.push(CstNode::Error {
                        content: line_content.to_string(),
                        span: SpanInfo::from_range(start_span, rest),
                        message: format!("Invalid system call syntax: {:?}", e.code),
                    });

                    remaining = rest;
                    continue;
                }
                _ => {}
            }
        }

        // 不像命令或系统调用，尝试解析文本行
        if let Ok((rest, text_line)) = parse_text_line(remaining) {
            nodes.push(CstNode::TextLine(text_line));
            remaining = rest;
            continue;
        }

        // 容错：跳过一个字符
        if let Ok((rest, _)) = take::<usize, Span, nom::error::Error<Span>>(1usize)(remaining) {
            remaining = rest;
        } else {
            break;
        }
    }

    Ok((remaining, nodes))
}

/// 解析嵌入代码 @{ ... } 或 ## ... ##
pub fn parse_embedded_code(input: Span) -> ParseResult<CstEmbeddedCode> {
    alt((parse_embedded_code_brace, parse_embedded_code_hash)).parse(input)
}

/// 解析 @{...} 语法的嵌入代码（推荐）
fn parse_embedded_code_brace(input: Span) -> ParseResult<CstEmbeddedCode> {
    let start_span = input;
    let (input, _) = tag("@{").parse(input)?;

    // 手动匹配大括号，支持嵌套
    let mut depth = 1;
    let mut pos = 0;
    let content = input.fragment();
    let chars: Vec<char> = content.chars().collect();

    while pos < chars.len() && depth > 0 {
        match chars[pos] {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        pos += 1;
    }

    if depth != 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Char,
        )));
    }

    let code_end = pos - 1; // 不包含最后的 }
    let code = chars[..code_end].iter().collect::<String>();

    // 消耗代码和结束的 }
    let (input, _) = take(pos).parse(input)?;
    let end_span = input;

    Ok((
        input,
        CstEmbeddedCode {
            syntax: EmbeddedCodeSyntax::Brace,
            code,
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析 ##...## 语法的嵌入代码（兼容旧版本）
/// 与 AST parser 对齐：要求 ## 后可以有空格和可选换行，结束的 ## 后必须有空格和换行
fn parse_embedded_code_hash(input: Span) -> ParseResult<CstEmbeddedCode> {
    use nom::character::complete::anychar;

    let start_span = input;

    // 开始：## + 可选的行内空格 + 可选的换行
    let (input, _) = tag("##").parse(input)?;
    let (input, _) = parse_whitespace_inline.parse(input)?;
    let (input, _) = opt(parse_line_ending).parse(input)?;

    // 收集内容直到遇到：行内空格 + ##
    let (input, content_chars) =
        many_till(anychar, (parse_whitespace_inline, tag("##"))).parse(input)?;

    let code: String = content_chars.0.into_iter().collect();
    let end_span = input;

    Ok((
        input,
        CstEmbeddedCode {
            syntax: EmbeddedCodeSyntax::Hash,
            code,
            span: SpanInfo::from_range(start_span, end_span),
        },
    ))
}

/// 解析行内空白（不包括换行）
fn parse_whitespace_inline(input: Span) -> ParseResult<()> {
    value((), many0(one_of(" \t"))).parse(input)
}

/// 解析换行符（\n 或 \r\n）
fn parse_line_ending(input: Span) -> ParseResult<()> {
    value((), alt((tag("\r\n"), tag("\n")))).parse(input)
}

/// 解析文本行 [leading] text #tailing
pub fn parse_text_line(input: Span) -> ParseResult<CstTextLine> {
    let start_span = input;
    let (input, leading_trivia) = many0(parse_trivia).parse(input)?;

    // 检查是否以特殊字符开头（不是文本行）
    if input.fragment().trim_start().starts_with('@')
        || input.fragment().trim_start().starts_with('#')
        || input.fragment().trim_start().starts_with('{')
        || input.fragment().trim_start().starts_with('}')
        || input.fragment().trim_start().starts_with(':')
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // 解析前导文本（可选）
    let (input, leading) = opt(parse_leading_text).parse(input)?;
    let (input, _) = space0(input)?;

    // 解析主文本（可选）
    let (input, text) = opt(parse_text).parse(input)?;
    let (input, _) = space0(input)?;

    // 解析后缀标记（可选）
    let (input, tailing) = opt(parse_tailing_text).parse(input)?;

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstTextLine {
            leading,
            text,
            tailing,
            span,
            leading_trivia,
        },
    ))
}

// Helper functions for parse_leading_text
fn parse_leading_template(i: Span) -> ParseResult<CstLeadingTextContent> {
    let (i, tpl) = parse_template_literal(i)?;
    Ok((i, CstLeadingTextContent::Template(tpl)))
}

fn parse_leading_quoted(i: Span) -> ParseResult<CstLeadingTextContent> {
    let (i, text) = parse_quoted_string(i)?;
    Ok((i, CstLeadingTextContent::Text(text)))
}

fn parse_leading_bare(i: Span) -> ParseResult<CstLeadingTextContent> {
    // 裸文本：读取到 ] 为止
    let (i, text) = take_while(|c| c != ']' && c != '\n').parse(i)?;
    Ok((
        i,
        CstLeadingTextContent::Text(text.fragment().trim().to_string()),
    ))
}

/// 解析前导文本 [...]
fn parse_leading_text(input: Span) -> ParseResult<CstLeadingText> {
    let start_span = input;

    // 解析 [
    let open_start = input;
    let (input, _) = char('[').parse(input)?;
    let open_bracket = SpanInfo::from_span_and_len(open_start, 1);

    let (input, _) = space0(input)?;

    // 解析内容（模板或普通文本）
    let (input, content) = alt((
        parse_leading_template,
        parse_leading_quoted,
        parse_leading_bare,
    ))
    .parse(input)?;

    let (input, _) = space0(input)?;

    // 解析 ]
    let close_start = input;
    let (input, _) = char(']').parse(input)?;
    let close_bracket = SpanInfo::from_span_and_len(close_start, 1);

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstLeadingText {
            open_bracket,
            content,
            close_bracket,
            span,
        },
    ))
}

/// 解析主文本
fn parse_text(input: Span) -> ParseResult<CstText> {
    let start_span = input;

    // 尝试模板字符串 `...`
    if let Ok((i, tpl)) = parse_template_literal(input) {
        let span = SpanInfo::from_range(start_span, i);
        return Ok((
            i,
            CstText {
                kind: CstTextKind::Template(tpl.clone()),
                raw: start_span.fragment()[..span.len()].to_string(),
                parsed: String::new(), // template 不需要 parsed
                span,
            },
        ));
    }

    // 尝试带引号的字符串 "..." 或 '...'
    if let Some(quote_char) = input.fragment().chars().next() {
        if quote_char == '"' || quote_char == '\'' {
            if let Ok((i, text)) = parse_quoted_string(input) {
                let quote_style = if quote_char == '"' {
                    QuoteStyle::Double
                } else {
                    QuoteStyle::Single
                };
                let span = SpanInfo::from_range(start_span, i);
                return Ok((
                    i,
                    CstText {
                        kind: CstTextKind::Quoted(quote_style),
                        raw: start_span.fragment()[..span.len()].to_string(),
                        parsed: text,
                        span,
                    },
                ));
            }
        }
    }

    // 裸文本：读取到行尾或特殊字符
    let (i, text) =
        take_while1(|c: char| c != '\n' && c != '\r' && c != '#' && c != '@' && c != '{')
            .parse(input)?;

    let span = SpanInfo::from_range(start_span, i);
    let text_str = text.fragment().trim_end().to_string();

    Ok((
        i,
        CstText {
            kind: CstTextKind::Bare,
            raw: text_str.clone(),
            parsed: text_str,
            span,
        },
    ))
}

/// 解析后缀标记 #tag
fn parse_tailing_text(input: Span) -> ParseResult<CstTailingText> {
    let start_span = input;

    // 解析 #
    let hash_start = input;
    let (input, _) = char('#').parse(input)?;
    let hash_token = SpanInfo::from_span_and_len(hash_start, 1);

    // 解析标记名
    let marker_start = input;
    let (input, marker) = take_while1(|c: char| !c.is_whitespace()).parse(input)?;
    let marker_end = input;
    let marker_span = SpanInfo::from_range(marker_start, marker_end);

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((
        input,
        CstTailingText {
            hash_token,
            marker: marker.fragment().to_string(),
            marker_span,
            span,
        },
    ))
}

/// 解析模板字符串 `...${var}...`
fn parse_template_literal(input: Span) -> ParseResult<CstTemplateLiteral> {
    let start_span = input;

    // 解析开始的反引号
    let (input, _) = char('`').parse(input)?;

    let mut parts = Vec::new();
    let mut remaining = input;

    loop {
        // 检查是否到达结束引号
        if remaining.fragment().starts_with('`') {
            break;
        }

        if remaining.fragment().is_empty() {
            // 未闭合的模板字符串
            return Err(nom::Err::Error(nom::error::Error::new(
                remaining,
                nom::error::ErrorKind::Tag,
            )));
        }

        // 尝试解析变量插值 ${...}
        if remaining.fragment().starts_with("${") {
            let value_start = remaining;
            let (rest, _) = tag("${").parse(remaining)?;
            let open_token = SpanInfo::from_span_and_len(value_start, 2);

            // 解析变量名
            let var_start = rest;
            let (rest, (var_name, _)) = parse_identifier(rest)?;
            let var_end = rest;
            let variable_span = SpanInfo::from_range(var_start, var_end);

            // 解析 }
            let close_start = rest;
            let (rest, _) = char('}').parse(rest)?;
            let close_token = SpanInfo::from_span_and_len(close_start, 1);

            let part_span = SpanInfo::from_range(value_start, rest);

            parts.push(CstTemplatePart::Value {
                open_token,
                variable: format::Variable {
                    chain: vec![var_name.clone()],
                },
                variable_span,
                close_token,
                span: part_span,
            });

            remaining = rest;
        } else {
            // 解析文本部分
            let text_start = remaining;
            let mut text = String::new();

            while !remaining.fragment().is_empty()
                && !remaining.fragment().starts_with('`')
                && !remaining.fragment().starts_with("${")
            {
                let ch = remaining.fragment().chars().next().unwrap();

                // 处理转义字符
                if ch == '\\' && remaining.fragment().len() > 1 {
                    let (rest, _) = char('\\').parse(remaining)?;
                    let next_ch = rest.fragment().chars().next().unwrap();

                    let escaped = match next_ch {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '`' => '`',
                        '$' => '$',
                        _ => next_ch,
                    };

                    text.push(escaped);
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                } else {
                    text.push(ch);
                    let (rest, _) = take(1usize)(remaining)?;
                    remaining = rest;
                }
            }

            if !text.is_empty() {
                let text_end = remaining;
                let text_span = SpanInfo::from_range(text_start, text_end);

                parts.push(CstTemplatePart::Text {
                    content: text,
                    span: text_span,
                });
            }
        }
    }

    // 解析结束的反引号
    let (input, _) = char('`').parse(remaining)?;

    let end_span = input;
    let span = SpanInfo::from_range(start_span, end_span);

    Ok((input, CstTemplateLiteral { parts, span }))
}

/// 解析带引号的字符串（支持转义）
fn parse_quoted_string(input: Span) -> ParseResult<String> {
    let quote_char = input.fragment().chars().next().ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
    })?;

    if quote_char != '"' && quote_char != '\'' {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (input, _) = char(quote_char).parse(input)?;

    let mut result = String::new();
    let mut remaining = input;

    while !remaining.fragment().is_empty() {
        let ch = remaining.fragment().chars().next().unwrap();

        if ch == quote_char {
            // 结束引号
            let (rest, _) = char(quote_char).parse(remaining)?;
            return Ok((rest, result));
        }

        if ch == '\\' && remaining.fragment().len() > 1 {
            // 转义字符
            let (rest, _) = char('\\').parse(remaining)?;
            let next_ch = rest.fragment().chars().next().unwrap();

            match next_ch {
                'n' => {
                    result.push('\n');
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
                't' => {
                    result.push('\t');
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
                'r' => {
                    result.push('\r');
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
                '\\' => {
                    result.push('\\');
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
                '"' => {
                    result.push('"');
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
                '\'' => {
                    result.push('\'');
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
                'u' => {
                    // Unicode 转义 \uXXXX 或 \u{XXXX}
                    let (rest, _) = take(1usize)(rest)?; // 消耗 'u'

                    if rest.fragment().starts_with('{') {
                        // \u{XXXX} 格式
                        let (rest, _) = char('{').parse(rest)?;

                        // 读取十六进制数字直到 }
                        let mut hex_len = 0;
                        let chars: Vec<char> = rest.fragment().chars().collect();

                        while hex_len < chars.len() && chars[hex_len] != '}' {
                            if !chars[hex_len].is_ascii_hexdigit() {
                                return Err(nom::Err::Error(nom::error::Error::new(
                                    rest,
                                    nom::error::ErrorKind::HexDigit,
                                )));
                            }
                            hex_len += 1;
                        }

                        if hex_len == 0 || hex_len >= chars.len() {
                            return Err(nom::Err::Error(nom::error::Error::new(
                                rest,
                                nom::error::ErrorKind::HexDigit,
                            )));
                        }

                        let hex_str: String = chars[..hex_len].iter().collect();
                        let (rest, _) = take(hex_len)(rest)?;
                        let (rest, _) = char('}').parse(rest)?;

                        if let Ok(code_point) = u32::from_str_radix(&hex_str, 16) {
                            if let Some(unicode_char) = char::from_u32(code_point) {
                                result.push(unicode_char);
                            } else {
                                return Err(nom::Err::Error(nom::error::Error::new(
                                    rest,
                                    nom::error::ErrorKind::HexDigit,
                                )));
                            }
                        } else {
                            return Err(nom::Err::Error(nom::error::Error::new(
                                rest,
                                nom::error::ErrorKind::HexDigit,
                            )));
                        }

                        remaining = rest;
                    } else {
                        // \uXXXX 格式（固定4位）
                        let (rest, hex_digits) = take(4usize)(rest)?;
                        let hex_str = hex_digits.fragment();

                        if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                            return Err(nom::Err::Error(nom::error::Error::new(
                                rest,
                                nom::error::ErrorKind::HexDigit,
                            )));
                        }

                        if let Ok(code_point) = u16::from_str_radix(hex_str, 16) {
                            if let Some(unicode_char) = char::from_u32(code_point as u32) {
                                result.push(unicode_char);
                            } else {
                                return Err(nom::Err::Error(nom::error::Error::new(
                                    rest,
                                    nom::error::ErrorKind::HexDigit,
                                )));
                            }
                        } else {
                            return Err(nom::Err::Error(nom::error::Error::new(
                                rest,
                                nom::error::ErrorKind::HexDigit,
                            )));
                        }

                        remaining = rest;
                    }
                }
                _ => {
                    result.push(next_ch);
                    let (rest, _) = take(1usize)(rest)?;
                    remaining = rest;
                }
            }
        } else {
            result.push(ch);
            let (rest, _) = take(1usize)(remaining)?;
            remaining = rest;
        }
    }

    // 未闭合的字符串
    Err(nom::Err::Error(nom::error::Error::new(
        remaining,
        nom::error::ErrorKind::Tag,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_parenthesized() {
        let input = r#"@changebg(src="test.jpg", fadeTime=600)"#;
        let result = parse_command(Span::new(input));
        assert!(result.is_ok());

        let (_, cmd) = result.unwrap();
        assert_eq!(cmd.command, "changebg");
        assert_eq!(cmd.arguments.len(), 2);
        assert_eq!(cmd.arguments[0].name, "src");
        assert_eq!(cmd.arguments[1].name, "fadeTime");
        assert!(matches!(cmd.syntax, CommandSyntax::Parenthesized { .. }));
    }

    #[test]
    fn test_parse_command_space_separated() {
        let input = r#"@changebg src="test.jpg" fadeTime=600"#;
        let result = parse_command(Span::new(input));
        assert!(result.is_ok());

        let (_, cmd) = result.unwrap();
        assert_eq!(cmd.command, "changebg");
        assert_eq!(cmd.arguments.len(), 2);
        assert!(matches!(cmd.syntax, CommandSyntax::SpaceSeparated));
    }

    #[test]
    fn test_parse_command_boolean_flag() {
        let input = r#"@command flag"#;
        let result = parse_command(Span::new(input));
        assert!(result.is_ok());

        let (_, cmd) = result.unwrap();
        assert_eq!(cmd.arguments.len(), 1);
        assert_eq!(cmd.arguments[0].name, "flag");
        assert!(cmd.arguments[0].value.is_none());
    }

    #[test]
    fn test_parse_systemcall() {
        let input = r#"#goto paragraph="main""#;
        let result = parse_systemcall(Span::new(input));
        assert!(result.is_ok());

        let (_, sc) = result.unwrap();
        assert_eq!(sc.command, "goto");
        assert_eq!(sc.arguments.len(), 1);
        assert_eq!(sc.arguments[0].name, "paragraph");
    }

    #[test]
    fn test_parse_tolerant() {
        let input = r#"
        @command1 arg=1
        // 注释
        @command2 arg=2
        "#;

        let cst = parse_tolerant("test", input);

        // 应该包含空白、command1、空白、注释、空白、command2、空白
        assert!(cst.nodes.len() > 0);

        // 统计命令数量
        let cmd_count = cst
            .nodes
            .iter()
            .filter(|n| matches!(n, CstNode::Command(_)))
            .count();
        assert_eq!(cmd_count, 2);
    }

    #[test]
    fn test_parse_number_values() {
        let tests = vec![
            ("123", CstValueKind::Integer),
            ("-456", CstValueKind::Integer),
            ("3.14", CstValueKind::Float),
            ("-2.5", CstValueKind::Float),
        ];

        for (input, expected_kind) in tests {
            let result = parse_number_value(Span::new(input));
            assert!(result.is_ok(), "Failed to parse: {}", input);
            let (_, value) = result.unwrap();
            assert_eq!(value.kind, expected_kind);
        }
    }

    #[test]
    fn test_to_ast() {
        let input = r#"@changebg src="test.jpg" fadeTime=600"#;
        let (_, cmd) = parse_command(Span::new(input)).unwrap();

        let ast_cmd = cmd.to_ast();
        assert_eq!(ast_cmd.command, "changebg");
        assert_eq!(ast_cmd.arguments.len(), 2);
    }

    #[test]
    fn test_parse_parameter_without_default() {
        let input = "param1";
        let result = parse_parameter(Span::new(input));
        assert!(result.is_ok());

        let (_, param) = result.unwrap();
        assert_eq!(param.name, "param1");
        assert!(param.default_value.is_none());
        assert!(param.equals_token.is_none());
    }

    #[test]
    fn test_parse_parameter_with_default() {
        let input = r#"param2="default""#;
        let result = parse_parameter(Span::new(input));
        assert!(result.is_ok());

        let (_, param) = result.unwrap();
        assert_eq!(param.name, "param2");
        assert!(param.default_value.is_some());
        assert!(param.equals_token.is_some());
    }

    #[test]
    fn test_parse_parameters() {
        let input = r#"(param1, param2="default", param3=123)"#;
        let result = parse_parameters(Span::new(input));
        assert!(result.is_ok());

        let (_, (_, params, _)) = result.unwrap();
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "param1");
        assert_eq!(params[1].name, "param2");
        assert_eq!(params[2].name, "param3");
    }

    #[test]
    fn test_parse_block_empty() {
        let input = "{}";
        let result = parse_block(Span::new(input));
        assert!(result.is_ok());

        let (_, block) = result.unwrap();
        assert_eq!(block.children.len(), 0);
    }

    #[test]
    fn test_parse_block_with_commands() {
        let input = r#"{
            @command1 arg=1
            @command2 arg=2
        }"#;
        let result = parse_block(Span::new(input));
        assert!(result.is_ok());

        let (_, block) = result.unwrap();

        // 统计命令数量（忽略 trivia）
        let cmd_count = block
            .children
            .iter()
            .filter(|n| matches!(n, CstNode::Command(_)))
            .count();
        assert_eq!(cmd_count, 2);
    }

    #[test]
    fn test_parse_block_nested() {
        let input = r#"{
            @command1
            {
                @command2
            }
        }"#;
        let result = parse_block(Span::new(input));
        assert!(result.is_ok());

        let (_, block) = result.unwrap();

        // 应该包含嵌套块
        let has_nested_block = block
            .children
            .iter()
            .any(|n| matches!(n, CstNode::Block(_)));
        assert!(has_nested_block);
    }

    #[test]
    fn test_parse_paragraph_simple() {
        let input = r#"::main {
            @command arg=1
        }"#;
        let result = parse_paragraph(Span::new(input));
        assert!(result.is_ok());

        let (_, para) = result.unwrap();
        assert_eq!(para.name, "main");
        assert_eq!(para.parameters.len(), 0);
        assert!(para.open_paren.is_none());
        assert!(para.close_paren.is_none());
    }

    #[test]
    fn test_parse_paragraph_with_params() {
        let input = r#"::scene(location, time="morning") {
            @changebg src="bg.jpg"
        }"#;
        let result = parse_paragraph(Span::new(input));
        assert!(result.is_ok());

        let (_, para) = result.unwrap();
        assert_eq!(para.name, "scene");
        assert_eq!(para.parameters.len(), 2);
        assert!(para.open_paren.is_some());
        assert!(para.close_paren.is_some());

        assert_eq!(para.parameters[0].name, "location");
        assert!(para.parameters[0].default_value.is_none());

        assert_eq!(para.parameters[1].name, "time");
        assert!(para.parameters[1].default_value.is_some());
    }

    #[test]
    fn test_paragraph_to_ast() {
        let input = r#"::test(param1="value") {
            @command arg=1
        }"#;
        let (_, para) = parse_paragraph(Span::new(input)).unwrap();

        let ast_para = para.to_ast().unwrap();
        assert_eq!(ast_para.name, "test");
        assert_eq!(ast_para.parameters.len(), 1);
        assert_eq!(ast_para.parameters[0].name, "param1");
    }

    #[test]
    fn test_parse_file_with_paragraphs() {
        let input = r#"
        ::start {
            @changebg src="bg.jpg"
            #goto next
        }
        
        ::next {
            @wait time=1000
        }
        "#;

        let cst = parse_tolerant("test", input);

        // 统计段落数量
        let para_count = cst
            .nodes
            .iter()
            .filter(|n| matches!(n, CstNode::Paragraph(_)))
            .count();
        assert_eq!(para_count, 2);

        // 测试 CST 到 AST 的转换
        let ast = cst.to_ast().unwrap();
        assert_eq!(ast.name, "test");
        assert_eq!(ast.paragraphs.len(), 2);
        assert_eq!(ast.paragraphs[0].name, "start");
        assert_eq!(ast.paragraphs[1].name, "next");
    }

    #[test]
    fn test_cst_to_ast_equivalence() {
        let input = r#"
        ::main(location="classroom") {
            @changebg src="bg.jpg" fadeTime=600
            @addchar name="hero" src="hero.png"
            #goto ending
        }
        
        ::ending {
            @wait time=500
        }
        "#;

        // 使用 CST 解析
        let cst = parse_tolerant("test", input);
        let cst_ast = cst.to_ast().unwrap();

        // 使用原始 AST parser 解析
        let ast_result = crate::parser::parse("test", input);

        // 如果两者都成功，验证段落数量相同
        if let Ok((_, ast)) = ast_result {
            assert_eq!(cst_ast.paragraphs.len(), ast.paragraphs.len());
            assert_eq!(cst_ast.paragraphs[0].name, ast.paragraphs[0].name);
            assert_eq!(cst_ast.paragraphs[1].name, ast.paragraphs[1].name);
        }
    }

    #[test]
    fn test_parse_quoted_string_double() {
        let input = r#""hello world""#;
        let result = parse_quoted_string(Span::new(input));
        assert!(result.is_ok());

        let (_, text) = result.unwrap();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn test_parse_quoted_string_single() {
        let input = r#"'hello world'"#;
        let result = parse_quoted_string(Span::new(input));
        assert!(result.is_ok());

        let (_, text) = result.unwrap();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn test_parse_quoted_string_with_escapes() {
        let input = r#""hello\nworld\t!""#;
        let result = parse_quoted_string(Span::new(input));
        assert!(result.is_ok());

        let (_, text) = result.unwrap();
        assert_eq!(text, "hello\nworld\t!");
    }

    #[test]
    fn test_parse_template_literal_simple() {
        let input = "`hello world`";
        let result = parse_template_literal(Span::new(input));
        assert!(result.is_ok());

        let (_, tpl) = result.unwrap();
        assert_eq!(tpl.parts.len(), 1);

        if let CstTemplatePart::Text { content, .. } = &tpl.parts[0] {
            assert_eq!(content, "hello world");
        } else {
            panic!("Expected text part");
        }
    }

    #[test]
    fn test_parse_template_literal_with_variable() {
        let input = "`hello ${name}!`";
        let result = parse_template_literal(Span::new(input));
        assert!(result.is_ok());

        let (_, tpl) = result.unwrap();
        assert_eq!(tpl.parts.len(), 3); // "hello ", ${name}, "!"

        if let CstTemplatePart::Text { content, .. } = &tpl.parts[0] {
            assert_eq!(content, "hello ");
        } else {
            panic!("Expected text part");
        }

        if let CstTemplatePart::Value { variable, .. } = &tpl.parts[1] {
            assert_eq!(variable.chain, vec!["name".to_string()]);
        } else {
            panic!("Expected value part");
        }
    }

    #[test]
    fn test_parse_leading_text_simple() {
        let input = "[角色名]";
        let result = parse_leading_text(Span::new(input));
        assert!(result.is_ok());

        let (_, leading) = result.unwrap();
        if let CstLeadingTextContent::Text(text) = &leading.content {
            assert_eq!(text, "角色名");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_parse_leading_text_quoted() {
        let input = r#"["角色名"]"#;
        let result = parse_leading_text(Span::new(input));
        assert!(result.is_ok());

        let (_, leading) = result.unwrap();
        if let CstLeadingTextContent::Text(text) = &leading.content {
            assert_eq!(text, "角色名");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_parse_tailing_text() {
        let input = "#wait";
        let result = parse_tailing_text(Span::new(input));
        assert!(result.is_ok());

        let (_, tailing) = result.unwrap();
        assert_eq!(tailing.marker, "wait");
    }

    #[test]
    fn test_parse_text_bare() {
        let input = "这是一段文本";
        let result = parse_text(Span::new(input));
        assert!(result.is_ok());

        let (_, text) = result.unwrap();
        assert!(matches!(text.kind, CstTextKind::Bare));
        assert_eq!(text.parsed, "这是一段文本");
    }

    #[test]
    fn test_parse_text_quoted() {
        let input = r#""这是一段文本""#;
        let result = parse_text(Span::new(input));
        assert!(result.is_ok());

        let (_, text) = result.unwrap();
        assert!(matches!(text.kind, CstTextKind::Quoted(_)));
        assert_eq!(text.parsed, "这是一段文本");
    }

    #[test]
    fn test_parse_text_line_simple() {
        let input = "这是一行文本\n";
        let result = parse_text_line(Span::new(input));
        assert!(result.is_ok());

        let (_, line) = result.unwrap();
        assert!(line.leading.is_none());
        assert!(line.text.is_some());
        assert!(line.tailing.is_none());
    }

    #[test]
    fn test_parse_text_line_with_leading() {
        let input = "[角色名] \"对话内容\"\n";
        let result = parse_text_line(Span::new(input));
        assert!(result.is_ok());

        let (_, line) = result.unwrap();
        assert!(line.leading.is_some());
        assert!(line.text.is_some());
    }

    #[test]
    fn test_parse_text_line_with_tailing() {
        let input = "对话内容 #wait\n";
        let result = parse_text_line(Span::new(input));
        assert!(result.is_ok());

        let (_, line) = result.unwrap();
        assert!(line.text.is_some());
        assert!(line.tailing.is_some());
    }

    #[test]
    fn test_text_line_to_ast() {
        let input = "[角色] \"对话\"\n";
        let (_, line) = parse_text_line(Span::new(input)).unwrap();

        let ast_child = line.to_ast().unwrap();
        if let format::ChildContent::TextLine(leading, text, _) = ast_child.content {
            assert!(matches!(leading, format::LeadingText::Text(_)));
            assert!(matches!(text, format::Text::Text(_)));
        } else {
            panic!("Expected TextLine");
        }
    }
}
