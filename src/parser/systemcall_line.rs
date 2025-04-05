use nom::character::complete::char;
use nom::combinator::cut;
use nom::sequence::*;

use crate::result::SixuResult;

use super::argument::arguments;
use super::comment::span0;
use super::comment::span0_inline;
use super::identifier::identifier;
use super::Child;
use super::SystemCallLine;

pub fn systemcall_line(input: &str) -> SixuResult<&str, Child> {
    let (input, (command, arguments)) = preceded(
        span0,
        tuple((
            preceded(char('#'), cut(identifier)),
            delimited(span0_inline, cut(arguments), span0_inline),
        )),
    )(input)?;

    Ok((
        input,
        Child::SystemCallLine(SystemCallLine {
            command: command.to_string(),
            arguments,
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
            systemcall_line("#command()"),
            Ok((
                "",
                Child::SystemCallLine(SystemCallLine {
                    command: "command".to_string(),
                    arguments: vec![],
                })
            ))
        );
        assert_eq!(
            systemcall_line("#command(a=1)"),
            Ok((
                "",
                Child::SystemCallLine(SystemCallLine {
                    command: "command".to_string(),
                    arguments: vec![Argument {
                        name: "a".to_string(),
                        value: Some(RValue::Primitive(Primitive::Integer(1))),
                    }],
                })
            ))
        );
        assert_eq!(
            systemcall_line("#command(a=1, b='aa')"),
            Ok((
                "",
                Child::SystemCallLine(SystemCallLine {
                    command: "command".to_string(),
                    arguments: vec![
                        Argument {
                            name: "a".to_string(),
                            value: Some(RValue::Primitive(Primitive::Integer(1))),
                        },
                        Argument {
                            name: "b".to_string(),
                            value: Some(RValue::Primitive(Primitive::String("aa".to_string()))),
                        }
                    ],
                })
            ))
        );
    }
}
