use nom::character::complete::char;
use nom::combinator::cut;
use nom::multi::many0;
use nom::sequence::*;

use crate::result::SixuResult;

use super::argument::argument;
use super::comment::span0;
use super::comment::span0_inline;
use super::identifier::identifier;
use super::Child;
use super::CommandLine;

pub fn command_line(input: &str) -> SixuResult<&str, Child> {
    let (input, (command, arguments)) = preceded(
        span0,
        tuple((
            preceded(char('@'), cut(identifier)),
            cut(many0(delimited(span0_inline, argument, span0_inline))),
        )),
    )(input)?;

    Ok((
        input,
        Child::CommandLine(CommandLine {
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
                Child::CommandLine(CommandLine {
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
                Child::CommandLine(CommandLine {
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
                Child::CommandLine(CommandLine {
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
                Child::CommandLine(CommandLine {
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
                Child::CommandLine(CommandLine {
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
                Child::CommandLine(CommandLine {
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
