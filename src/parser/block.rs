use nom::branch::alt;
use nom::bytes::complete::*;
use nom::combinator::cut;
use nom::multi::many0;
use nom::sequence::*;

use crate::format::Child;
use crate::result::SixuResult;

use super::command_line::command_line;
use super::comment::span0;
use super::systemcall_line::systemcall_line;
use super::text::text_line;
use super::Block;

pub fn block(input: &str) -> SixuResult<&str, Block> {
    let (input, _) = tag("{")(input)?;
    let (input, children) = cut(many0(preceded(span0, child)))(input)?;
    let (input, _) = preceded(span0, tag("}"))(input)?;
    Ok((
        input,
        Block {
            attributes: vec![],
            children,
        },
    ))
}

pub fn block_child(input: &str) -> SixuResult<&str, Child> {
    let (input, block) = block(input)?;
    Ok((input, Child::Block(block)))
}

pub fn child(input: &str) -> SixuResult<&str, Child> {
    let (input, _) = span0(input)?;
    let (input, child) = alt((block_child, command_line, systemcall_line, text_line))(input)?;
    Ok((input, child))
}

#[cfg(test)]
mod tests {
    use crate::format::{Argument, Child, CommandLine, Primitive, RValue, SystemCallLine};

    use super::*;

    #[test]
    fn test_block() {
        assert_eq!(
            block("{}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![],
                }
            ))
        );
        assert_eq!(
            block("{\n}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![],
                }
            ))
        );
        assert_eq!(
            block("{\n@command foo=false}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![Child::CommandLine(CommandLine {
                        command: "command".to_string(),
                        flags: vec![],
                        arguments: vec![Argument {
                            name: "foo".to_string(),
                            value: Some(RValue::Primitive(Primitive::Boolean(false))),
                        }],
                    })],
                }
            ))
        );
        assert_eq!(
            block("{\n@command foo=false\ntext\n}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![
                        Child::CommandLine(CommandLine {
                            command: "command".to_string(),
                            flags: vec![],
                            arguments: vec![Argument {
                                name: "foo".to_string(),
                                value: Some(RValue::Primitive(Primitive::Boolean(false))),
                            }],
                        }),
                        Child::TextLine("text".to_string())
                    ],
                }
            ))
        );
        assert_eq!(
            block("{\n#command(foo=false)\ntext\n}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![
                        Child::SystemCallLine(SystemCallLine {
                            command: "command".to_string(),
                            arguments: vec![Argument {
                                name: "foo".to_string(),
                                value: Some(RValue::Primitive(Primitive::Boolean(false))),
                            }],
                        }),
                        Child::TextLine("text".to_string())
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
                    attributes: vec![],
                    children: vec![
                        Child::CommandLine(CommandLine {
                            command: "command".to_string(),
                            flags: vec![],
                            arguments: vec![Argument {
                                name: "foo".to_string(),
                                value: Some(RValue::Primitive(Primitive::Boolean(false))),
                            }],
                        }),
                        Child::TextLine("text".to_string()),
                        Child::Block(Block {
                            attributes: vec![],
                            children: vec![Child::CommandLine(CommandLine {
                                command: "command".to_string(),
                                flags: vec![],
                                arguments: vec![Argument {
                                    name: "bar".to_string(),
                                    value: Some(RValue::Primitive(Primitive::Boolean(true))),
                                }],
                            })],
                        })
                    ],
                }
            ))
        );
    }
}
