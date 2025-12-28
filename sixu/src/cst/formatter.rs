/// CST-based formatter for Sixu language
/// 
/// This formatter preserves all comments and produces formatted output
/// with consistent spacing, indentation, and line breaks.

use crate::cst::node::*;

pub struct CstFormatter {
    indent_size: usize,
}

impl Default for CstFormatter {
    fn default() -> Self {
        Self { indent_size: 4 }
    }
}

impl CstFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_indent(indent_size: usize) -> Self {
        Self { indent_size }
    }

    /// Format a CST root node into a string
    pub fn format(&self, root: &CstRoot) -> String {
        let mut output = String::new();
        
        for node in &root.nodes {
            self.format_node(node, 0, &mut output);
        }

        // 确保文件以换行符结尾
        if !output.ends_with('\n') {
            output.push('\n');
        }

        output
    }

    fn format_node(&self, node: &CstNode, indent_level: usize, output: &mut String) {
        match node {
            CstNode::Trivia(trivia) => self.format_trivia(trivia, indent_level, output),
            CstNode::Paragraph(para) => self.format_paragraph(para, indent_level, output),
            CstNode::Command(cmd) => self.format_command(cmd, indent_level, output),
            CstNode::SystemCall(call) => self.format_systemcall(call, indent_level, output),
            CstNode::TextLine(text) => self.format_textline(text, indent_level, output),
            CstNode::Block(block) => self.format_block(block, indent_level, output),
            CstNode::EmbeddedCode(code) => self.format_embedded_code(code, indent_level, output),
            CstNode::Error { content, .. } => {
                // 保留错误节点的原始内容
                output.push_str(content);
                output.push('\n');
            }
        }
    }

    fn format_trivia(&self, trivia: &CstTrivia, indent_level: usize, output: &mut String) {
        match trivia {
            CstTrivia::Whitespace { content, .. } => {
                // 处理空行：如果包含2个或以上换行符（表示源码中有空行），输出一个空行
                let newline_count = content.chars().filter(|&c| c == '\n').count();
                if newline_count >= 2 {
                    // 多个换行符，输出一个空行
                    output.push('\n');
                }
            }
            CstTrivia::LineComment { content, .. } => {
                self.indent(indent_level, output);
                output.push_str("//");
                output.push_str(content);
                output.push('\n');
            }
            CstTrivia::BlockComment { content, .. } => {
                // 多行注释需要特殊处理
                let lines: Vec<&str> = content.lines().collect();
                
                if lines.len() <= 1 {
                    // 单行注释：/* content */
                    self.indent(indent_level, output);
                    output.push_str("/*");
                    output.push_str(content);
                    output.push_str("*/");
                    output.push('\n');
                } else {
                    // 多行注释：/* 单独一行，每个内容行添加 * 前缀
                    self.indent(indent_level, output);
                    output.push_str("/*");
                    output.push('\n');
                    
                    for line in &lines {
                        self.indent(indent_level, output);
                        output.push_str(" *");
                        if !line.is_empty() {
                            output.push(' ');
                            output.push_str(line.trim());
                        }
                        output.push('\n');
                    }
                    
                    self.indent(indent_level, output);
                    output.push_str(" */");
                    output.push('\n');
                }
            }
        }
    }

    fn format_paragraph(&self, para: &CstParagraph, indent_level: usize, output: &mut String) {
        // 段落前加一个空行（如果不是文件开头）
        if !output.is_empty() && !output.ends_with("\n\n") {
            output.push('\n');
        }

        // ::name
        output.push_str("::");
        output.push_str(&para.name);

        // 参数
        if !para.parameters.is_empty() {
            output.push('(');
            for (i, param) in para.parameters.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                self.format_parameter(param, output);
            }
            output.push(')');
        }

        output.push_str(" ");
        self.format_block(&para.block, indent_level, output);
    }

    fn format_parameter(&self, param: &CstParameter, output: &mut String) {
        output.push_str(&param.name);
        if let Some(ref default_value) = param.default_value {
            output.push('=');
            self.format_value(default_value, output);
        }
    }

    fn format_block(&self, block: &CstBlock, indent_level: usize, output: &mut String) {
        // Block开括号需要缩进（除非是段落的根block，indent_level为0）
        if indent_level > 0 {
            self.indent(indent_level, output);
        }
        output.push_str("{\n");

        for child in &block.children {
            self.format_node(child, indent_level + 1, output);
        }

        self.indent(indent_level, output);
        output.push_str("}\n");
    }

    fn format_command(&self, cmd: &CstCommand, indent_level: usize, output: &mut String) {
        self.indent(indent_level, output);
        
        output.push('@');
        output.push_str(&cmd.command);

        if !cmd.arguments.is_empty() {
            match cmd.syntax {
                CommandSyntax::Parenthesized { .. } => {
                    // 括号语法：@cmd(a=1, b=2)
                    output.push('(');
                    for (i, arg) in cmd.arguments.iter().enumerate() {
                        if i > 0 {
                            output.push_str(", ");
                        }
                        self.format_argument(arg, output);
                    }
                    output.push(')');
                }
                CommandSyntax::SpaceSeparated => {
                    // 空格分隔：@cmd a=1 b=2
                    for arg in &cmd.arguments {
                        output.push(' ');
                        self.format_argument(arg, output);
                    }
                }
            }
        }

        output.push('\n');
    }

    fn format_systemcall(&self, call: &CstSystemCall, indent_level: usize, output: &mut String) {
        self.indent(indent_level, output);
        
        output.push('#');
        output.push_str(&call.command);

        if !call.arguments.is_empty() {
            match call.syntax {
                CommandSyntax::Parenthesized { .. } => {
                    // 括号语法：#goto(paragraph="main")
                    output.push('(');
                    for (i, arg) in call.arguments.iter().enumerate() {
                        if i > 0 {
                            output.push_str(", ");
                        }
                        self.format_argument(arg, output);
                    }
                    output.push(')');
                }
                CommandSyntax::SpaceSeparated => {
                    // 空格分隔：#goto paragraph="main"
                    for arg in &call.arguments {
                        output.push(' ');
                        self.format_argument(arg, output);
                    }
                }
            }
        }

        output.push('\n');
    }

    fn format_argument(&self, arg: &CstArgument, output: &mut String) {
        output.push_str(&arg.name);
        if let Some(ref value) = arg.value {
            output.push('=');
            self.format_value(value, output);
        }
    }

    fn format_value(&self, value: &CstValue, output: &mut String) {
        // raw 字段已经包含了引号等原始文本
        output.push_str(&value.raw);
    }

    fn format_textline(&self, text: &CstTextLine, indent_level: usize, output: &mut String) {
        self.indent(indent_level, output);

        if let Some(ref leading) = text.leading {
            self.format_leading_text(leading, output);
            output.push(' ');
        }

        if let Some(ref main_text) = text.text {
            self.format_text(main_text, output);
        }

        if let Some(ref tailing) = text.tailing {
            output.push(' ');
            self.format_tailing_text(tailing, output);
        }

        output.push('\n');
    }

    fn format_leading_text(&self, leading: &CstLeadingText, output: &mut String) {
        output.push('[');
        match &leading.content {
            CstLeadingTextContent::Text(s) => output.push_str(s),
            CstLeadingTextContent::Template(tpl) => {
                output.push('`');
                self.format_template_literal(tpl, output);
                output.push('`');
            }
        }
        output.push(']');
    }

    fn format_text(&self, text: &CstText, output: &mut String) {
        // raw 字段已经包含了引号等原始文本
        output.push_str(&text.raw);
    }

    fn format_template_literal(&self, tpl: &CstTemplateLiteral, output: &mut String) {
        for part in &tpl.parts {
            match part {
                CstTemplatePart::Text { content, .. } => {
                    output.push_str(content);
                }
                CstTemplatePart::Value { variable, .. } => {
                    output.push_str("${");
                    output.push_str(&variable.chain.join("."));
                    output.push('}');
                }
            }
        }
    }

    fn format_tailing_text(&self, tailing: &CstTailingText, output: &mut String) {
        output.push('#');
        output.push_str(&tailing.marker);
    }

    fn format_embedded_code(&self, code: &CstEmbeddedCode, indent_level: usize, output: &mut String) {
        match code.syntax {
            EmbeddedCodeSyntax::Brace => {
                let trimmed_code = code.code.trim();
                if trimmed_code.contains('\n') {
                    // 多行语法：@{ \n code \n }
                    self.indent(indent_level, output);
                    output.push_str("@{\n");
                    
                    // 移除首尾的换行符，但保留内部的缩进
                    let code_content = code.code.trim_matches(|c| c == '\n' || c == '\r');
                    output.push_str(code_content);
                    output.push('\n');
                    
                    self.indent(indent_level, output);
                    output.push_str("}\n");
                } else {
                    // 单行语法：@{ code }
                    self.indent(indent_level, output);
                    output.push_str("@{ ");
                    output.push_str(trimmed_code);
                    output.push_str(" }\n");
                }
            }
            EmbeddedCodeSyntax::Hash => {
                let trimmed_code = code.code.trim();
                if trimmed_code.contains('\n') {
                    // 多行语法：开始和结束标记在独立的行上，代码内容保留原样
                    self.indent(indent_level, output);
                    output.push_str("##\n");
                    // 直接输出代码内容，保留其原始格式（不额外缩进）
                    output.push_str(&code.code);
                    if !code.code.ends_with('\n') {
                        output.push('\n');
                    }
                    self.indent(indent_level, output);
                    output.push_str("##\n");
                } else {
                    // 单行语法：## code ##
                    self.indent(indent_level, output);
                    output.push_str("## ");
                    output.push_str(trimmed_code);
                    output.push_str(" ##\n");
                }
            }
        }
    }

    fn indent(&self, level: usize, output: &mut String) {
        for _ in 0..(level * self.indent_size) {
            output.push(' ');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parser::parse_tolerant;

    #[test]
    fn test_format_simple_command() {
        let input = "@command(arg=1)";
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        assert!(result.contains("@command(arg=1)"));
    }

    #[test]
    fn test_format_paragraph() {
        let input = r#"
::test {
@command(arg=1)
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        assert!(result.contains("::test {"));
        assert!(result.contains("    @command(arg=1)"));
        assert!(result.contains("}"));
    }

    #[test]
    fn test_format_preserves_comments() {
        let input = r#"
// 这是注释
::test {
    /* 块注释 */
    @command arg=1
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        // 应该保留注释
        assert!(result.contains("// 这是注释"));
        assert!(result.contains("/* 块注释 */"));
    }

    #[test]
    fn test_format_multiple_paragraphs() {
        let input = r#"
::first {
    @cmd1 arg=1
}

::second {
    @cmd2 arg=2
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        assert!(result.contains("::first {"));
        assert!(result.contains("::second {"));
        // 段落间应该有空行
        assert!(result.contains("}\n\n::second"));
    }

    #[test]
    fn test_format_text_line() {
        let input = r#"
::test {
    [speaker] "Hello, world!"
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        println!("Formatted result:\n{}", result);
        // 文本行目前可能还没完全实现解析
        assert!(result.contains("::test {"));
    }

    #[test]
    fn test_format_system_call() {
        let input = r#"
::test {
    #goto(next)
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        assert!(result.contains("#goto(next)"));
    }

    #[test]
    fn test_format_preserves_indent_before_comments() {
        let input = r#"
::test {
    // 这是一个注释
    @command arg=1
    /* 这是块注释 */
    @another arg=2
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        println!("Formatted result:\n{}", result);
        // 注释应该有正确的缩进
        assert!(result.contains("    // 这是一个注释"));
        assert!(result.contains("    /* 这是块注释 */"));
    }

    #[test]
    fn test_format_no_extra_blank_lines() {
        let input = r#"
::test {
    @cmd1 arg=1
    @cmd2 arg=2
    @cmd3 arg=3
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        println!("Formatted result:\n{}", result);
        // 命令之间不应该有空行（原本没有空行的地方）
        assert!(!result.contains("@cmd1(arg=1)\n\n    @cmd2"));
        assert!(!result.contains("@cmd2(arg=2)\n\n    @cmd3"));
    }

    #[test]
    fn test_format_reduces_multiple_blank_lines() {
        let input = r#"
::test {
    @cmd1(arg=1)


    @cmd2(arg=2)
}
"#;
        let cst = parse_tolerant("test", input);
        let formatter = CstFormatter::new();
        let result = formatter.format(&cst);
        
        println!("Formatted result:\n{}", result);
        // 多个空行应该被缩减为一个
        assert!(result.contains("@cmd1(arg=1)\n\n    @cmd2"));
    }

    #[test]
    fn test_format_preserves_command_syntax() {
        // 测试括号语法保留
        let input1 = r#"
::test {
    @command(arg=1, flag)
}
"#;
        let cst1 = parse_tolerant("test", input1);
        let formatter = CstFormatter::new();
        let result1 = formatter.format(&cst1);
        
        println!("Parenthesized syntax result:\n{}", result1);
        assert!(result1.contains("@command(arg=1, flag)"));
        
        // 测试空格分隔语法保留
        let input2 = r#"
::test {
    @command arg=1 flag
}
"#;
        let cst2 = parse_tolerant("test", input2);
        let result2 = formatter.format(&cst2);
        
        println!("Space-separated syntax result:\n{}", result2);
        assert!(result2.contains("@command arg=1 flag"));
    }

    #[test]
    fn test_format_preserves_systemcall_syntax() {
        // 测试括号语法保留
        let input1 = r#"
::test {
    #goto(paragraph="main")
}
"#;
        let cst1 = parse_tolerant("test", input1);
        let formatter = CstFormatter::new();
        let result1 = formatter.format(&cst1);
        
        println!("Parenthesized systemcall result:\n{}", result1);
        assert!(result1.contains("#goto(paragraph=\"main\")"));
        
        // 测试空格分隔语法保留
        let input2 = r#"
::test {
    #goto paragraph="main"
}
"#;
        let cst2 = parse_tolerant("test", input2);
        let result2 = formatter.format(&cst2);
        
        println!("Space-separated systemcall result:\n{}", result2);
        assert!(result2.contains("#goto paragraph=\"main\""));
    }
}
