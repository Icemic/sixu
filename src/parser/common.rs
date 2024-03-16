use nom::bytes::complete::*;
use nom::combinator::*;
use nom::sequence::*;
use nom::IResult;

use super::comment::span0;
use super::identifier::identifier;
use super::primitive::primitive;
use super::Argument;

pub fn argument(input: &str) -> IResult<&str, Argument> {
    let (input, name) = identifier(input)?;
    let (input, _) = span0(input)?;
    let (input, value) = opt(preceded(tag("="), preceded(span0, primitive)))(input)?;
    Ok((
        input,
        Argument {
            name: name.to_string(),
            value,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::Primitive;

    use super::*;

    #[test]
    fn test_argument() {
        assert_eq!(
            argument("a"),
            Ok((
                "",
                Argument {
                    name: "a".to_string(),
                    value: None,
                }
            ))
        );
        assert_eq!(
            argument("a = 1"),
            Ok((
                "",
                Argument {
                    name: "a".to_string(),
                    value: Some(Primitive::Integer(1)),
                }
            ))
        );
        assert_eq!(
            argument("a = 1 "),
            Ok((
                " ",
                Argument {
                    name: "a".to_string(),
                    value: Some(Primitive::Integer(1)),
                }
            ))
        );
        assert_eq!(
            argument(r#"foo = "bar" "#),
            Ok((
                " ",
                Argument {
                    name: "foo".to_string(),
                    value: Some(Primitive::String("bar".to_string())),
                }
            ))
        );
    }
}
