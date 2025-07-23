use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::{anychar, line_ending};
use nom::combinator::{cut, opt};
use nom::multi::{many0, many_till};
use nom::sequence::*;
use nom::Parser;

use crate::format::{Child, ChildContent};
use crate::result::ParseResult;

use super::attribute::{attribute, balanced_delimiters};
use super::command_line::command_line;
use super::comment::{span0, span0_inline};
use super::systemcall_line::systemcall_line;
use super::text::text_line;
use super::Block;

pub fn block(input: &str) -> ParseResult<&str, Block> {
    let (input, _) = tag("{").parse(input)?;
    let (input, children) = cut(many0(preceded(span0, child))).parse(input)?;
    let (input, _) = preceded(span0, tag("}")).parse(input)?;
    Ok((input, Block { children }))
}

pub fn block_child(input: &str) -> ParseResult<&str, ChildContent> {
    let (input, block) = block.parse(input)?;
    Ok((input, ChildContent::Block(block)))
}

pub fn child(input: &str) -> ParseResult<&str, Child> {
    let (input, _) = span0.parse(input)?;
    let (input, attributes) = many0(attribute).parse(input)?;
    let (input, _) = span0.parse(input)?; // Ensure whitespace between attributes and content is handled correctly
    let (input, child) = alt((
        embedded_code,
        block_child,
        command_line,
        systemcall_line,
        text_line,
    ))
    .parse(input)?;
    Ok((
        input,
        Child {
            attributes,
            content: child,
        },
    ))
}

pub fn embedded_code(input: &str) -> ParseResult<&str, ChildContent> {
    alt((embedded_code_brace, embedded_code_hash)).parse(input)
}

/// Parse embedded code using @{...} syntax (recommended)
pub fn embedded_code_brace(input: &str) -> ParseResult<&str, ChildContent> {
    let (input, _) = tag("@{").parse(input)?;
    let (input, content) = cut(balanced_delimiters('{', '}')).parse(input)?;

    Ok((input, ChildContent::EmbeddedCode(content.to_string())))
}

/// Parse embedded code using ##...## syntax (legacy support)
pub fn embedded_code_hash(input: &str) -> ParseResult<&str, ChildContent> {
    let (input, _) = (tag("##"), span0_inline, opt(line_ending)).parse(input)?;
    let (input, (content, _)) =
        cut(many_till(anychar, (tag("##"), span0_inline, line_ending))).parse(input)?;
    Ok((
        input,
        ChildContent::EmbeddedCode(content.into_iter().collect::<String>()),
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::{
        Argument, Attribute, ChildContent, CommandLine, LeadingText, Literal, RValue,
        SystemCallLine, TemplateLiteral, TemplateLiteralPart, Text, Variable,
    };

    use super::*;

    #[test]
    fn test_block() {
        assert_eq!(block("{}"), Ok(("", Block { children: vec![] })));
        assert_eq!(block("{\n}"), Ok(("", Block { children: vec![] })));
        assert_eq!(
            block("{\n@command foo=false}"),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        attributes: vec![],
                        content: ChildContent::CommandLine(CommandLine {
                            command: "command".to_string(),
                            flags: vec![],
                            arguments: vec![Argument {
                                name: "foo".to_string(),
                                value: Some(RValue::Literal(Literal::Boolean(false))),
                            }],
                        }),
                    }],
                }
            ))
        );
        assert_eq!(
            block("{\n@command foo=false\ntext\n}"),
            Ok((
                "",
                Block {
                    children: vec![
                        Child {
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                flags: vec![],
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: Some(RValue::Literal(Literal::Boolean(false))),
                                }],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::Text("text".to_string())
                            ),
                        }
                    ],
                }
            ))
        );
        assert_eq!(
            block("{\n#command(foo=false)\ntext\n}"),
            Ok((
                "",
                Block {
                    children: vec![
                        Child {
                            attributes: vec![],
                            content: ChildContent::SystemCallLine(SystemCallLine {
                                command: "command".to_string(),
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: Some(RValue::Literal(Literal::Boolean(false))),
                                }],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::Text("text".to_string())
                            ),
                        }
                    ],
                }
            ))
        );
        // recursive blocks
        assert_eq!(
            block("{\n@command foo=false\ntext\n{\n@command bar=true\n}\n}"),
            Ok((
                "",
                Block {
                    children: vec![
                        Child {
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                flags: vec![],
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: Some(RValue::Literal(Literal::Boolean(false))),
                                }],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::Text("text".to_string())
                            ),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::Block(Block {
                                children: vec![Child {
                                    attributes: vec![],
                                    content: ChildContent::CommandLine(CommandLine {
                                        command: "command".to_string(),
                                        flags: vec![],
                                        arguments: vec![Argument {
                                            name: "bar".to_string(),
                                            value: Some(RValue::Literal(Literal::Boolean(
                                                true
                                            ))),
                                        }],
                                    }),
                                }],
                            }),
                        }
                    ],
                }
            ))
        );
    }

    #[test]
    fn test_embedded_code_hash() {
        // inline code
        assert_eq!(
            embedded_code_hash("##code##\n"),
            Ok(("", ChildContent::EmbeddedCode("code".to_string())))
        );
        // inline code with other text
        assert_eq!(
            embedded_code_hash("##code##\ntext\n"),
            Ok(("text\n", ChildContent::EmbeddedCode("code".to_string())))
        );
        // multi-line code
        assert_eq!(
            embedded_code_hash("## \n  code \n ##  \ntext\n"),
            Ok((
                "text\n",
                ChildContent::EmbeddedCode("  code \n ".to_string()),
            ))
        );
        // ## is mixed with text
        assert_eq!(
            embedded_code_hash("##\ncode\n'aaa##'\n##\ntext\n"),
            Ok((
                "text\n",
                ChildContent::EmbeddedCode("code\n'aaa##'\n".to_string())
            ))
        );
    }

    #[test]
    fn test_embedded_code_brace() {
        // Simple code
        assert_eq!(
            embedded_code_brace("@{let a = 1;}"),
            Ok(("", ChildContent::EmbeddedCode("let a = 1;".to_string())))
        );

        // Multi-line code
        assert_eq!(
            embedded_code_brace("@{  \n  let a = 1;\n  console.log(a);\n  }"),
            Ok((
                "",
                ChildContent::EmbeddedCode("  \n  let a = 1;\n  console.log(a);\n  ".to_string())
            ))
        );

        // Nested braces
        assert_eq!(
            embedded_code_brace("@{if (condition) { doSomething(); }}"),
            Ok((
                "",
                ChildContent::EmbeddedCode("if (condition) { doSomething(); }".to_string())
            ))
        );

        // Contains various brackets and quotes
        assert_eq!(
            embedded_code_brace(
                "@{function test() { return `template ${value}` && obj['key'] && (1 + 2); }}"
            ),
            Ok((
                "",
                ChildContent::EmbeddedCode(
                    "function test() { return `template ${value}` && obj['key'] && (1 + 2); }"
                        .to_string()
                )
            ))
        );

        // Followed by other content
        assert_eq!(
            embedded_code_brace("@{let x = 10;}remaining text"),
            Ok((
                "remaining text",
                ChildContent::EmbeddedCode("let x = 10;".to_string())
            ))
        );
    }

    #[test]
    fn test_embedded_code() {
        // Test if both syntaxes can be correctly parsed by the embedded_code function

        // @{} syntax
        assert_eq!(
            embedded_code("@{const x = 42;}"),
            Ok(("", ChildContent::EmbeddedCode("const x = 42;".to_string())))
        );

        // ## ## syntax
        assert_eq!(
            embedded_code("##const y = 'hello';##\n"),
            Ok((
                "",
                ChildContent::EmbeddedCode("const y = 'hello';".to_string())
            ))
        );
    }

    #[test]
    fn test_block_with_embedded_code() {
        // Test both embedded code syntaxes used in a block
        let input = "{@{let a = 1;}\n##let b = 2;##\n}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![
                        Child {
                            attributes: vec![],
                            content: ChildContent::EmbeddedCode("let a = 1;".to_string()),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::EmbeddedCode("let b = 2;".to_string()),
                        }
                    ],
                }
            ))
        );
    }

    #[test]
    fn test_embedded_code_with_attributes() {
        // Test embedded code combined with attributes
        let input = "{#[condition(a > b)]\n@{let x = a > b ? a : b;}}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        attributes: vec![Attribute {
                            keyword: "condition".to_string(),
                            condition: Some("a > b".to_string()),
                        }],
                        content: ChildContent::EmbeddedCode("let x = a > b ? a : b;".to_string()),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_template_line_mix_with_command() {
        let input = "{`hello \n${world} ${123} world` \n \n@command foo=false\n}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::TemplateLiteral(TemplateLiteral {
                                    parts: vec![
                                        TemplateLiteralPart::Text("hello \n".to_string()),
                                        TemplateLiteralPart::Value(RValue::Variable(Variable {
                                            chain: vec!["world".to_string()],
                                        })),
                                        TemplateLiteralPart::Text(" ".to_string()),
                                        TemplateLiteralPart::Value(RValue::Literal(
                                            Literal::Integer(123)
                                        )),
                                        TemplateLiteralPart::Text(" world".to_string()),
                                    ],
                                })
                            ),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                flags: vec![],
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: Some(RValue::Literal(Literal::Boolean(false))),
                                }],
                            }),
                        }
                    ],
                }
            ))
        );
    }

    #[test]
    fn test_line_with_attribute() {
        let input = "{#[attribute_name(a = 123 )]\ntext\n}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        attributes: vec![Attribute {
                            keyword: "attribute_name".to_string(),
                            condition: Some("a = 123 ".to_string()),
                        }],
                        content: ChildContent::TextLine(
                            LeadingText::None,
                            Text::Text("text".to_string())
                        ),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_line_with_multiple_attributes() {
        let input =
            "{#[attribute_name(a = 123 )]\n#[attribute_name(a && (b + 1) > '])'.length)]\ntext\n}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        attributes: vec![
                            Attribute {
                                keyword: "attribute_name".to_string(),
                                condition: Some("a = 123 ".to_string()),
                            },
                            Attribute {
                                keyword: "attribute_name".to_string(),
                                condition: Some("a && (b + 1) > '])'.length".to_string()),
                            }
                        ],
                        content: ChildContent::TextLine(
                            LeadingText::None,
                            Text::Text("text".to_string())
                        ),
                    }],
                }
            ))
        );
    }
}
