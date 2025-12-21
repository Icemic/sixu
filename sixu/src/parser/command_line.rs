use nom::character::complete::char;
use nom::combinator::cut;
use nom::sequence::*;
use nom::Parser;

use crate::result::ParseResult;

use super::argument::arguments;
use super::comment::span0;
use super::identifier::identifier;
use super::ChildContent;
use super::CommandLine;

pub fn command_line(input: &str) -> ParseResult<&str, ChildContent> {
    let (input, (command, arguments)) =
        preceded(span0, (preceded(char('@'), cut(identifier)), arguments)).parse(input)?;

    Ok((
        input,
        ChildContent::CommandLine(CommandLine {
            command: command.to_string(),
            arguments: arguments,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::{Argument, Literal, RValue};

    use super::*;

    #[test]
    fn test_line() {
        assert_eq!(
            command_line("@command"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
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
                    arguments: vec![Argument {
                        name: "a".to_string(),
                        value: RValue::Literal(Literal::Boolean(true)),
                    }],
                })
            ))
        );
        assert_eq!(
            command_line("@command a = 1"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    arguments: vec![Argument {
                        name: "a".to_string(),
                        value: RValue::Literal(Literal::Integer(1)),
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
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: RValue::Literal(Literal::Integer(1)),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: RValue::Literal(Literal::Boolean(true)),
                        }
                    ],
                })
            ))
        );
        assert_eq!(
            command_line("@command a= 1 b = 2"),
            Ok((
                "",
                ChildContent::CommandLine(CommandLine {
                    command: "command".to_string(),
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: RValue::Literal(Literal::Integer(1)),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: RValue::Literal(Literal::Integer(2)),
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
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: RValue::Literal(Literal::Integer(1)),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: RValue::Literal(Literal::Integer(2)),
                        },
                        Argument {
                            name: "c".to_string(),
                            value: RValue::Literal(Literal::Boolean(true)),
                        }
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
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: RValue::Literal(Literal::Integer(1)),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: RValue::Literal(Literal::Integer(2)),
                        },
                        Argument {
                            name: "c".to_string(),
                            value: RValue::Literal(Literal::Boolean(true)),
                        }
                    ],
                })
            ))
        );
    }
}
