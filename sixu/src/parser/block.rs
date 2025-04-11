use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::{anychar, line_ending};
use nom::combinator::{cut, opt};
use nom::multi::{many0, many_till};
use nom::sequence::*;
use nom::Parser;

use crate::format::{Child, ChildContent};
use crate::result::SixuResult;

use super::command_line::command_line;
use super::comment::{span0, span0_inline};
use super::systemcall_line::systemcall_line;
use super::template::template_line;
use super::text::text_line;
use super::Block;

pub fn block(input: &str) -> SixuResult<&str, Block> {
    let (input, _) = tag("{").parse(input)?;
    let (input, children) = cut(many0(preceded(span0, child))).parse(input)?;
    let (input, _) = preceded(span0, tag("}")).parse(input)?;
    Ok((input, Block { children }))
}

pub fn block_child(input: &str) -> SixuResult<&str, ChildContent> {
    let (input, block) = block.parse(input)?;
    Ok((input, ChildContent::Block(block)))
}

pub fn child(input: &str) -> SixuResult<&str, Child> {
    let (input, _) = span0.parse(input)?;
    let (input, child) = alt((
        embedded_code,
        block_child,
        command_line,
        systemcall_line,
        template_line,
        text_line,
    ))
    .parse(input)?;
    Ok((
        input,
        Child {
            attributes: vec![],
            content: child,
        },
    ))
}

pub fn embedded_code(input: &str) -> SixuResult<&str, ChildContent> {
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
        Argument, ChildContent, CommandLine, LeadingText, Primitive, RValue, SystemCallLine,
        TemplateLiteral, Variable,
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
                                value: Some(RValue::Primitive(Primitive::Boolean(false))),
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
                                    value: Some(RValue::Primitive(Primitive::Boolean(false))),
                                }],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(LeadingText::None, "text".to_string()),
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
                                    value: Some(RValue::Primitive(Primitive::Boolean(false))),
                                }],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(LeadingText::None, "text".to_string()),
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
                                    value: Some(RValue::Primitive(Primitive::Boolean(false))),
                                }],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::TextLine(LeadingText::None, "text".to_string()),
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
                                            value: Some(RValue::Primitive(Primitive::Boolean(
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
    fn test_embedded_code() {
        // inline code
        assert_eq!(
            embedded_code("##code##\n"),
            Ok(("", ChildContent::EmbeddedCode("code".to_string())))
        );
        // inline code with other text
        assert_eq!(
            embedded_code("##code##\ntext\n"),
            Ok(("text\n", ChildContent::EmbeddedCode("code".to_string())))
        );
        // multi-line code
        assert_eq!(
            embedded_code("## \n  code \n ##  \ntext\n"),
            Ok((
                "text\n",
                ChildContent::EmbeddedCode("  code \n ".to_string()),
            ))
        );
        // ## is mixed with text
        assert_eq!(
            embedded_code("##\ncode\n'aaa##'\n##\ntext\n"),
            Ok((
                "text\n",
                ChildContent::EmbeddedCode("code\n'aaa##'\n".to_string())
            ))
        );
        // ## is mixed with text and has a line ending
        // FIXME: this test is not working
        // assert_eq!(
        //     embedded_code("##\ncode\n//##\n##\ntext\n"),
        //     Ok((
        //         "text\n",
        //         ChildContent::EmbeddedCode("code\n//##\n".to_string())
        //     ))
        // );
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
                            content: ChildContent::TemplateLiteral(TemplateLiteral {
                                strings: vec![
                                    "hello \n".to_string(),
                                    " ".to_string(),
                                    " world".to_string()
                                ],
                                values: vec![
                                    RValue::Variable(Variable {
                                        chain: vec!["world".to_string()],
                                    }),
                                    RValue::Primitive(Primitive::Integer(123)),
                                ],
                            }),
                        },
                        Child {
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                flags: vec![],
                                arguments: vec![Argument {
                                    name: "foo".to_string(),
                                    value: Some(RValue::Primitive(Primitive::Boolean(false))),
                                }],
                            }),
                        }
                    ],
                }
            ))
        );
    }
}
