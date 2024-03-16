use nom::character::complete::char;
use nom::sequence::*;
use nom::IResult;

use super::argument::arguments;
use super::comment::span0;
use super::identifier::identifier;
use super::Child;
use super::SystemCallLine;

pub fn systemcall_line(input: &str) -> IResult<&str, Child> {
    let (input, (command, arguments)) = delimited(
        span0,
        tuple((
            preceded(char('#'), identifier),
            delimited(span0, arguments, span0),
        )),
        span0,
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
    }
}
