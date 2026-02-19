use sixu::cst::{node::*, span::SpanInfo};
use tower_lsp_server::ls_types::{Position, Range};

/// 将 CST SpanInfo 转换为 LSP Range
pub fn span_to_range(span: &SpanInfo) -> Range {
    Range {
        start: Position {
            line: (span.start_line - 1) as u32,  // SpanInfo 是 1-based
            character: span.start_column as u32, // SpanInfo 是 0-based
        },
        end: Position {
            line: (span.end_line - 1) as u32,
            character: span.end_column as u32,
        },
    }
}

/// 检查位置是否在范围内
#[allow(dead_code)]
pub fn contains(range: &Range, pos: &Position) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character >= range.end.character {
        return false;
    }
    true
}

/// 从 CST 中提取所有命令节点
pub fn extract_commands(cst: &CstRoot) -> Vec<&CstCommand> {
    let mut commands = Vec::new();

    fn visit_node<'a>(node: &'a CstNode, commands: &mut Vec<&'a CstCommand>) {
        match node {
            CstNode::Command(cmd) => commands.push(cmd),
            CstNode::Paragraph(para) => {
                visit_block(&para.block, commands);
            }
            CstNode::Block(block) => {
                visit_block(block, commands);
            }
            _ => {}
        }
    }

    fn visit_block<'a>(block: &'a CstBlock, commands: &mut Vec<&'a CstCommand>) {
        for child in &block.children {
            visit_node(child, commands);
        }
    }

    for node in &cst.nodes {
        visit_node(node, &mut commands);
    }

    commands
}

/// 从 CST 中提取所有系统调用节点
pub fn extract_system_calls(cst: &CstRoot) -> Vec<&CstSystemCall> {
    let mut system_calls = Vec::new();

    fn visit_node<'a>(node: &'a CstNode, calls: &mut Vec<&'a CstSystemCall>) {
        match node {
            CstNode::SystemCall(call) => calls.push(call),
            CstNode::Paragraph(para) => {
                visit_block(&para.block, calls);
            }
            CstNode::Block(block) => {
                visit_block(block, calls);
            }
            _ => {}
        }
    }

    fn visit_block<'a>(block: &'a CstBlock, calls: &mut Vec<&'a CstSystemCall>) {
        for child in &block.children {
            visit_node(child, calls);
        }
    }

    for node in &cst.nodes {
        visit_node(node, &mut system_calls);
    }

    system_calls
}

/// 从 CST 中提取所有段落节点
pub fn extract_paragraphs(cst: &CstRoot) -> Vec<&CstParagraph> {
    cst.nodes
        .iter()
        .filter_map(|node| match node {
            CstNode::Paragraph(para) => Some(para),
            _ => None,
        })
        .collect()
}

/// 从系统调用中获取参数值（字符串形式）
pub fn get_systemcall_argument_value(call: &CstSystemCall, arg_name: &str) -> Option<String> {
    call.arguments.iter().find_map(|arg| {
        if arg.name == arg_name {
            arg.value.as_ref().map(|v| match &v.kind {
                CstValueKind::String { .. } => {
                    // 去掉引号
                    let s = v.raw.trim();
                    if (s.starts_with('"') && s.ends_with('"'))
                        || (s.starts_with('\'') && s.ends_with('\''))
                    {
                        s[1..s.len() - 1].to_string()
                    } else {
                        v.raw.clone()
                    }
                }
                _ => v.raw.clone(),
            })
        } else {
            None
        }
    })
}

/// 检查位置是否在字符串内部
/// 简单检查：统计光标前的引号数量
pub fn is_inside_string(line_prefix: &str) -> bool {
    let mut in_double = false;
    let mut in_single = false;
    let mut in_template = false;
    let mut escape_next = false;

    for ch in line_prefix.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => escape_next = true,
            '"' if !in_single && !in_template => in_double = !in_double,
            '\'' if !in_double && !in_template => in_single = !in_single,
            '`' if !in_double && !in_single => in_template = !in_template,
            _ => {}
        }
    }

    in_double || in_single || in_template
}

/// 在当前行找到命令或系统调用，并检查光标是否在有效的参数补全位置
/// 返回：(命令名, 是否括号语法, 已有参数列表)
pub fn find_command_at_position(
    line: &str,
    cursor_col: usize,
) -> Option<(String, bool, Vec<String>)> {
    // 将字符索引转换为字节索引（处理多字节字符如中文）
    let mut char_count = 0;
    let mut byte_pos = 0;
    for (idx, _) in line.char_indices() {
        if char_count >= cursor_col {
            break;
        }
        byte_pos = idx;
        char_count += 1;
    }
    // 如果还没到达目标字符数，使用字符串末尾
    let slice_end = if char_count < cursor_col {
        line.len()
    } else {
        byte_pos
    };
    let line_prefix = &line[..slice_end];

    // 检查是否在字符串内
    if is_inside_string(line_prefix) {
        return None;
    }

    // 找到最后一个 @ 或 #
    let last_at = line_prefix.rfind('@');
    let last_hash = line_prefix.rfind('#');

    let (trigger_idx, _trigger_char) = match (last_at, last_hash) {
        (Some(at), Some(hash)) => {
            if at > hash {
                (at, '@')
            } else {
                (hash, '#')
            }
        }
        (Some(at), None) => (at, '@'),
        (None, Some(hash)) => (hash, '#'),
        (None, None) => return None,
    };

    let after_trigger = &line_prefix[trigger_idx + 1..];

    // 提取命令名
    let cmd_end = after_trigger
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(after_trigger.len());

    if cmd_end == 0 {
        // 命令名为空，不补全参数
        return None;
    }

    let cmd_name = &after_trigger[..cmd_end];
    let after_cmd = &after_trigger[cmd_end..];

    // 检查语法风格
    let is_paren = after_cmd.trim_start().starts_with('(');

    if is_paren {
        // 括号语法：检查光标是否在 ) 之前
        let close_paren_pos = after_cmd.find(')');
        if let Some(pos) = close_paren_pos
            && cmd_end + pos < after_trigger.len() {
                // 光标在 ) 之后，不补全
                return None;
            }
    }

    // 提取已有参数
    let existing_args = extract_argument_names(after_cmd, is_paren);

    Some((cmd_name.to_string(), is_paren, existing_args))
}

/// 提取参数名列表（简单实现，基于字符串解析）
fn extract_argument_names(after_cmd: &str, is_paren: bool) -> Vec<String> {
    let mut args = Vec::new();
    let content = if is_paren {
        // 提取括号内的内容
        let start = after_cmd.find('(').map(|i| i + 1).unwrap_or(0);
        let end = after_cmd.find(')').unwrap_or(after_cmd.len());
        &after_cmd[start..end]
    } else {
        after_cmd
    };

    // 简单的参数提取：找到所有 identifier= 或 identifier 模式
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    while i < chars.len() {
        // 跳过空白和逗号
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }

        if i >= chars.len() {
            break;
        }

        // 收集标识符
        let start = i;
        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }

        if i > start {
            let arg_name: String = chars[start..i].iter().collect();
            args.push(arg_name);

            // 跳过 =value 部分
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }

            if i < chars.len() && chars[i] == '=' {
                i += 1; // skip =

                // 跳过值（可能是字符串、数字、变量等）
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }

                if i < chars.len() {
                    if chars[i] == '"' || chars[i] == '\'' || chars[i] == '`' {
                        // 字符串值
                        let quote = chars[i];
                        i += 1;
                        while i < chars.len() && chars[i] != quote {
                            if chars[i] == '\\' {
                                i += 2; // skip escape
                            } else {
                                i += 1;
                            }
                        }
                        if i < chars.len() {
                            i += 1; // skip closing quote
                        }
                    } else {
                        // 非字符串值
                        while i < chars.len()
                            && !chars[i].is_whitespace()
                            && chars[i] != ','
                            && chars[i] != ')'
                        {
                            i += 1;
                        }
                    }
                }
            }
        }
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_inside_string() {
        assert!(!is_inside_string("@command "));
        assert!(is_inside_string("@command arg=\""));
        assert!(!is_inside_string("@command arg=\"value\" "));
        assert!(is_inside_string("@command arg='"));
        assert!(!is_inside_string("@command arg='value' "));
        assert!(is_inside_string("@command text=`template "));
        assert!(!is_inside_string("@command text=`template` "));

        // 转义字符测试
        assert!(is_inside_string(r#"@command arg="test \" "#));
        assert!(!is_inside_string(r#"@command arg="test \"" "#));
    }

    #[test]
    fn test_find_command_at_position() {
        // 基本命令
        let result = find_command_at_position("@changebg ", 10);
        assert_eq!(result, Some(("changebg".to_string(), false, vec![])));

        // 带参数的命令
        let result = find_command_at_position("@changebg src=\"test.jpg\" ", 25);
        assert_eq!(
            result,
            Some(("changebg".to_string(), false, vec!["src".to_string()]))
        );

        // 多个参数
        let result = find_command_at_position("@changebg src=\"test.jpg\" fadeTime=600 ", 39);
        assert_eq!(
            result,
            Some((
                "changebg".to_string(),
                false,
                vec!["src".to_string(), "fadeTime".to_string()]
            ))
        );

        // 括号语法
        let result = find_command_at_position("@changebg(src=\"test.jpg\", ", 26);
        assert_eq!(
            result,
            Some(("changebg".to_string(), true, vec!["src".to_string()]))
        );

        // 括号语法，多个参数
        let result = find_command_at_position("@changebg(src=\"test.jpg\", fadeTime=600, ", 41);
        assert_eq!(
            result,
            Some((
                "changebg".to_string(),
                true,
                vec!["src".to_string(), "fadeTime".to_string()]
            ))
        );

        // 光标在括号后面，不应该补全
        let result = find_command_at_position("@changebg(src=\"test.jpg\") ", 26);
        assert_eq!(result, None);

        // 光标在字符串内，不应该补全
        let result = find_command_at_position("@changebg src=\"test", 19);
        assert_eq!(result, None);

        // 系统调用
        let result = find_command_at_position("#goto ", 6);
        assert_eq!(result, Some(("goto".to_string(), false, vec![])));

        let result = find_command_at_position("#goto paragraph=\"main\" ", 23);
        assert_eq!(
            result,
            Some(("goto".to_string(), false, vec!["paragraph".to_string()]))
        );
    }

    #[test]
    fn test_extract_argument_names() {
        // 空格分隔
        assert_eq!(
            extract_argument_names(" src=\"test.jpg\" fadeTime=600", false),
            vec!["src".to_string(), "fadeTime".to_string()]
        );

        // 括号分隔
        assert_eq!(
            extract_argument_names("(src=\"test.jpg\", fadeTime=600)", true),
            vec!["src".to_string(), "fadeTime".to_string()]
        );

        // 布尔标志
        assert_eq!(
            extract_argument_names(" flag1 arg2=123 flag3", false),
            vec!["flag1".to_string(), "arg2".to_string(), "flag3".to_string()]
        );

        // 混合引号
        assert_eq!(
            extract_argument_names(r#" a="double" b='single' c=`template`"#, false),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );

        // 变量值
        assert_eq!(
            extract_argument_names(" x=variable.path y=123", false),
            vec!["x".to_string(), "y".to_string()]
        );
    }
}
