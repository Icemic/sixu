use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::{anychar, line_ending, multispace1};
use nom::combinator::{cut, opt};
use nom::error::ParseError;
use nom::multi::{many0, many_till};
use nom::sequence::*;
use nom::Parser;
use nom_language::error::VerboseError;

use crate::format::{Child, ChildContent, LineMarker};
use crate::result::ParseResult;

use super::attribute::{attribute, balanced_delimiters};
use super::command_line::command_line;
use super::comment::{comment, marker_directive_comment, span0, span0_inline};
use super::systemcall_line::systemcall_line;
use super::text::text_line;
use super::Block;

pub fn block(input: &str) -> ParseResult<&str, Block> {
    let (input, _) = tag("{").parse(input)?;
    let (input, children) = cut(block_children).parse(input)?;
    let (input, _) = preceded(span0, tag("}")).parse(input)?;
    Ok((input, Block { children }))
}

fn block_children(mut input: &str) -> ParseResult<&str, Vec<Child>> {
    let mut children = Vec::new();

    loop {
        let (next_input, marker) = leading_child_trivia(input)?;

        if let Ok((_, _)) = tag::<&str, &str, VerboseError<&str>>("}").parse(next_input) {
            if marker.is_some() {
                return Err(nom::Err::Error(VerboseError::from_error_kind(
                    next_input,
                    nom::error::ErrorKind::Tag,
                )));
            }
            return Ok((next_input, children));
        }

        let (after_child, mut child) = child(next_input)?;
        child.marker = marker;
        children.push(child);
        input = after_child;
    }
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
            marker: None,
            attributes,
            content: child,
        },
    ))
}

fn leading_child_trivia(mut input: &str) -> ParseResult<&str, Option<LineMarker>> {
    let mut marker = None;

    loop {
        if let Ok((next_input, next_marker)) = marker_directive_comment(input) {
            if marker.is_some() {
                return Err(nom::Err::Error(VerboseError::from_error_kind(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
            marker = Some(next_marker);
            input = next_input;
            continue;
        }

        if let Ok((next_input, _)) = comment(input) {
            input = next_input;
            continue;
        }

        if let Ok((next_input, _)) = multispace1::<&str, VerboseError<&str>>(input) {
            input = next_input;
            continue;
        }

        return Ok((input, marker));
    }
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
        SystemCallLine, TailingText, TemplateLiteral, TemplateLiteralPart, Text, Variable,
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
                        marker: None,
                        attributes: vec![],
                        content: ChildContent::CommandLine(CommandLine {
                            command: "command".to_string(),
                            arguments: vec![Argument {
                                name: "foo".to_string(),
                                value: RValue::Literal(Literal::Boolean(false)),
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
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: RValue::Literal(Literal::Boolean(false)),
                                }],
                            }),
                        },
                        Child {
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::Text("text".to_string()),
                                TailingText::None,
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
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::SystemCallLine(SystemCallLine {
                                command: "command".to_string(),
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: RValue::Literal(Literal::Boolean(false)),
                                }],
                            }),
                        },
                        Child {
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::Text("text".to_string()),
                                TailingText::None,
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
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: RValue::Literal(Literal::Boolean(false)),
                                }],
                            }),
                        },
                        Child {
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::TextLine(
                                LeadingText::None,
                                Text::Text("text".to_string()),
                                TailingText::None,
                            ),
                        },
                        Child {
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::Block(Block {
                                children: vec![Child {
                                    marker: None,
                                    attributes: vec![],
                                    content: ChildContent::CommandLine(CommandLine {
                                        command: "command".to_string(),
                                        arguments: vec![Argument {
                                            name: "bar".to_string(),
                                            value: RValue::Literal(Literal::Boolean(true)),
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
    fn test_block_marker_directive_binds_next_child() {
        assert_eq!(
            block("{\n//#marker id=Labc123\n@command foo=false\n}"),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: Some(LineMarker {
                            id: "Labc123".to_string(),
                        }),
                        attributes: vec![],
                        content: ChildContent::CommandLine(CommandLine {
                            command: "command".to_string(),
                            arguments: vec![Argument {
                                name: "foo".to_string(),
                                value: RValue::Literal(Literal::Boolean(false)),
                            }],
                        }),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_block_marker_directive_allows_regular_comments_between() {
        assert_eq!(
            block("{\n//#marker id=Labc123\n// comment\ntext\n}"),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: Some(LineMarker {
                            id: "Labc123".to_string(),
                        }),
                        attributes: vec![],
                        content: ChildContent::TextLine(
                            LeadingText::None,
                            Text::Text("text".to_string()),
                            TailingText::None,
                        ),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_block_marker_directive_rejects_duplicate_before_child() {
        assert!(block("{\n//#marker id=Labc123\n//#marker id=Ldef456\ntext\n}").is_err());
    }

    #[test]
    fn test_block_marker_directive_rejects_dangling_marker() {
        assert!(block("{\n//#marker id=Labc123\n}").is_err());
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
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::EmbeddedCode("let a = 1;".to_string()),
                        },
                        Child {
                            marker: None,
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
        let input = "{#[condition(\"a > b\")]\n@{let x = a > b ? a : b;}}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
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
                            marker: None,
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
                                }),
                                TailingText::None,
                            ),
                        },
                        Child {
                            marker: None,
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: RValue::Literal(Literal::Boolean(false)),
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
        let input = "{#[attribute_name(\"a = 123\")]\ntext\n}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
                        attributes: vec![Attribute {
                            keyword: "attribute_name".to_string(),
                            condition: Some("a = 123".to_string()),
                        }],
                        content: ChildContent::TextLine(
                            LeadingText::None,
                            Text::Text("text".to_string()),
                            TailingText::None,
                        ),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_line_with_multiple_attributes() {
        let input =
            "{#[attribute_name(\"a = 123\")]\n#[attribute_name(\"a && (b + 1) > '])'.length\")]\ntext\n}";

        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
                        attributes: vec![
                            Attribute {
                                keyword: "attribute_name".to_string(),
                                condition: Some("a = 123".to_string()),
                            },
                            Attribute {
                                keyword: "attribute_name".to_string(),
                                condition: Some("a && (b + 1) > '])'.length".to_string()),
                            }
                        ],
                        content: ChildContent::TextLine(
                            LeadingText::None,
                            Text::Text("text".to_string()),
                            TailingText::None,
                        ),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_cond_attribute_on_block() {
        let input = "{#[cond(\"x > 0\")]\n{\ntext\n}\n}";
        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
                        attributes: vec![Attribute {
                            keyword: "cond".to_string(),
                            condition: Some("x > 0".to_string()),
                        }],
                        content: ChildContent::Block(Block {
                            children: vec![Child {
                                marker: None,
                                attributes: vec![],
                                content: ChildContent::TextLine(
                                    LeadingText::None,
                                    Text::Text("text".to_string()),
                                    TailingText::None,
                                ),
                            }],
                        }),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_if_attribute_on_text_line() {
        let input = "{#[if(\"save.x = 1\")]\nsome text\n}";
        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
                        attributes: vec![Attribute {
                            keyword: "if".to_string(),
                            condition: Some("save.x = 1".to_string()),
                        }],
                        content: ChildContent::TextLine(
                            LeadingText::None,
                            Text::Text("some text".to_string()),
                            TailingText::None,
                        ),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_while_attribute_on_block() {
        let input = "{#[while(\"counter < 3\")]\n{\n@cmd arg=1\n}\n}";
        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
                        attributes: vec![Attribute {
                            keyword: "while".to_string(),
                            condition: Some("counter < 3".to_string()),
                        }],
                        content: ChildContent::Block(Block {
                            children: vec![Child {
                                marker: None,
                                attributes: vec![],
                                content: ChildContent::CommandLine(CommandLine {
                                    command: "cmd".to_string(),
                                    arguments: vec![Argument {
                                        name: "arg".to_string(),
                                        value: RValue::Literal(Literal::Integer(1)),
                                    }],
                                }),
                            }],
                        }),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_loop_attribute_on_block() {
        let input = "{#[loop]\n{\n@cmd arg=1\n#break\n}\n}";
        assert_eq!(
            block.parse(input),
            Ok((
                "",
                Block {
                    children: vec![Child {
                        marker: None,
                        attributes: vec![Attribute {
                            keyword: "loop".to_string(),
                            condition: None,
                        }],
                        content: ChildContent::Block(Block {
                            children: vec![
                                Child {
                                    marker: None,
                                    attributes: vec![],
                                    content: ChildContent::CommandLine(CommandLine {
                                        command: "cmd".to_string(),
                                        arguments: vec![Argument {
                                            name: "arg".to_string(),
                                            value: RValue::Literal(Literal::Integer(1)),
                                        }],
                                    }),
                                },
                                Child {
                                    marker: None,
                                    attributes: vec![],
                                    content: ChildContent::SystemCallLine(SystemCallLine {
                                        command: "break".to_string(),
                                        arguments: vec![],
                                    }),
                                },
                            ],
                        }),
                    }],
                }
            ))
        );
    }

    #[test]
    fn test_multiple_if_attributes_from_complex_sixu() {
        // Based on the complex.sixu example: three #[if(...)] on one block
        let input = concat!(
            "{",
            "#[if(\"a =123 && (b + 1) > '])'.length\")]\n",
            "#[if(\"save.x = 1\")]\n",
            "#[if(\"save.x = 1\")]\n",
            "{\n",
            "  `这是一行${embed_str}文本`\n",
            "}\n",
            "}",
        );
        let result = block.parse(input);
        assert!(
            result.is_ok(),
            "Should parse complex.sixu attribute example"
        );
        let (_, parsed_block) = result.unwrap();
        assert_eq!(parsed_block.children.len(), 1);
        let child = &parsed_block.children[0];
        assert_eq!(child.attributes.len(), 3);
        assert_eq!(child.attributes[0].keyword, "if");
        assert_eq!(child.attributes[1].keyword, "if");
        assert_eq!(child.attributes[2].keyword, "if");
        assert!(matches!(child.content, ChildContent::Block(_)));
    }
}
