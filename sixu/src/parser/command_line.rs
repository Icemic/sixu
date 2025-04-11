use nom::character::complete::char;
use nom::combinator::cut;
use nom::sequence::*;
use nom::Parser;

use crate::result::SixuResult;

use super::argument::arguments;
use super::comment::span0;
use super::identifier::identifier;
use super::ChildContent;
use super::CommandLine;

pub fn command_line(input: &str) -> SixuResult<&str, ChildContent> {
    let (input, (command, arguments)) =
        preceded(span0, (preceded(char('@'), cut(identifier)), arguments)).parse(input)?;

    Ok((
        input,
        ChildContent::CommandLine(CommandLine {
            command: command.to_string(),
            flags: arguments
                .iter()
                .filter(|p| p.value.is_none())
                .map(|p| p.name.clone())
                .collect(),
            arguments: arguments
                .iter()
                .filter(|p| p.value.is_some())
                .map(|p| p.to_owned())
                .collect(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::{Argument, Primitive, RValue};

    use super::*;

    #[test]
    fn test_line() {
        assert_eq!(
            command_line("@command"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec![],
                    arguments: vec![],
                })
            ))
        );
        assert_eq!(
            command_line("@command a"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec!["a".to_string()],
                    arguments: vec![],
                })
            ))
        );
        assert_eq!(
            command_line("@command a = 1"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec![],
                    arguments: vec![Argument {
                        name: "a".to_string(),
                        value: Some(RValue::Primitive(Primitive::Integer(1))),
                    }],
                })
            ))
        );
        assert_eq!(
            command_line("@command a = 1 b"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec!["b".to_string()],
                    arguments: vec![Argument {
                        name: "a".to_string(),
                        value: Some(RValue::Primitive(Primitive::Integer(1))),
                    }],
                })
            ))
        );
        assert_eq!(
            command_line("@command a= 1 b = 2"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec![],
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(1))),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(2))),
                        },
                    ],
                })
            ))
        );
        assert_eq!(
            command_line("@command a=1 b = 2 c"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec!["c".to_string()],
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(1))),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(2))),
                        },
                    ],
                })
            ))
        );
        assert_eq!(
            command_line("@command (a=1,b = 2,c)"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    flags: vec!["c".to_string()],
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(1))),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(2))),
                        },
                    ],
                })
            ))
        );
    }
}
