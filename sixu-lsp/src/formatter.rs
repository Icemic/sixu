use sixu::format::*;

pub struct Formatter {
    indent_str: String,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            indent_str: "    ".to_string(),
        }
    }

    pub fn format(&self, story: &Story) -> String {
        let mut output = String::new();
        for (i, p) in story.paragraphs.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&self.format_paragraph(p));
        }
        output
    }

    fn format_paragraph(&self, p: &Paragraph) -> String {
        let mut s = format!("::{}", p.name);
        if !p.parameters.is_empty() {
            s.push('(');
            for (i, param) in p.parameters.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&param.name);
                if let Some(default) = &param.default_value {
                    s.push('=');
                    s.push_str(&self.format_literal(default));
                }
            }
            s.push(')');
        }
        s.push_str(" {\n");
        s.push_str(&self.format_block(&p.block, 1));
        s.push_str("}\n");
        s
    }

    fn format_block(&self, block: &Block, level: usize) -> String {
        let mut s = String::new();
        let indent = self.indent_str.repeat(level);

        for child in &block.children {
            s.push_str(&indent);
            s.push_str(&self.format_child(child, level));
            s.push('\n');
        }
        s
    }

    fn format_child(&self, child: &Child, level: usize) -> String {
        let mut s = String::new();
        for attr in &child.attributes {
            s.push_str(&format!("@{}", attr.keyword));
            if let Some(cond) = &attr.condition {
                s.push_str(&format!("({})", cond));
            }
            s.push(' ');
        }

        match &child.content {
            ChildContent::Block(b) => {
                s.push_str("{\n");
                s.push_str(&self.format_block(b, level + 1));
                s.push_str(&self.indent_str.repeat(level));
                s.push_str("}");
            }
            ChildContent::TextLine(leading, text, tailing) => {
                match leading {
                    LeadingText::None => {}
                    LeadingText::Text(t) => s.push_str(&format!("[{}] ", t)),
                    LeadingText::TemplateLiteral(t) => {
                        s.push_str(&format!("[`{}`] ", self.format_template(t)))
                    }
                }

                match text {
                    Text::None => {}
                    Text::Text(t) => s.push_str(&format!("\"{}\"", t.escape_debug())),
                    Text::TemplateLiteral(t) => {
                        s.push_str(&format!("`{}`", self.format_template(t)))
                    }
                }

                match tailing {
                    TailingText::None => {}
                    TailingText::Text(t) => s.push_str(&format!(" #{}", t)),
                }
            }
            ChildContent::CommandLine(cmd) => {
                s.push('@');
                s.push_str(&cmd.command);
                for arg in &cmd.arguments {
                    s.push(' ');
                    s.push_str(&arg.name);
                    match &arg.value {
                        RValue::Literal(l) => {
                            if let Literal::Boolean(true) = l {
                                // omit =true
                            } else {
                                s.push('=');
                                s.push_str(&self.format_literal(l));
                            }
                        }
                        RValue::Variable(v) => {
                            s.push('=');
                            s.push_str(&v.chain.join("."));
                        }
                    }
                }
            }
            ChildContent::SystemCallLine(sys) => {
                s.push('#');
                s.push_str(&sys.command);
                for arg in &sys.arguments {
                    s.push(' ');
                    s.push_str(&arg.name);
                    match &arg.value {
                        RValue::Literal(l) => {
                            if let Literal::Boolean(true) = l {
                                // omit
                            } else {
                                s.push('=');
                                s.push_str(&self.format_literal(l));
                            }
                        }
                        RValue::Variable(v) => {
                            s.push('=');
                            s.push_str(&v.chain.join("."));
                        }
                    }
                }
            }
            ChildContent::EmbeddedCode(code) => {
                s.push_str("@{ ");
                s.push_str(code);
                s.push_str(" }");
            }
        }
        s
    }

    fn format_template(&self, t: &TemplateLiteral) -> String {
        let mut s = String::new();
        for part in &t.parts {
            match part {
                TemplateLiteralPart::Text(text) => s.push_str(text),
                TemplateLiteralPart::Value(val) => {
                    s.push_str("${");
                    match val {
                        RValue::Literal(l) => s.push_str(&self.format_literal(l)),
                        RValue::Variable(v) => s.push_str(&v.chain.join(".")),
                    }
                    s.push('}');
                }
            }
        }
        s
    }

    fn format_literal(&self, l: &Literal) -> String {
        match l {
            Literal::Null => "null".to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Integer(i) => i.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::Boolean(b) => b.to_string(),
            Literal::Array(_) => "[...]".to_string(), // TODO
            Literal::Object(_) => "{...}".to_string(), // TODO
        }
    }
}
