use nom::bytes::complete::*;
use nom::combinator::*;
use nom::multi::separated_list0;
use nom::sequence::*;

use crate::result::SixuResult;

use super::comment::span0;
use super::identifier::identifier;
use super::rvalue::rvalue;
use super::Argument;

pub fn arguments(input: &str) -> SixuResult<&str, Vec<Argument>> {
    let (input, _) = tag("(")(input)?;
    let (input, _) = span0(input)?;
    let (input, arguments) = separated_list0(delimited(span0, tag(","), span0), argument)(input)?;
    let (input, _) = span0(input)?;
    let (input, _) = tag(")")(input)?;
    Ok((input, arguments))
}

pub fn argument(input: &str) -> SixuResult<&str, Argument> {
    let (input, name) = identifier(input)?;
    let (input, _) = span0(input)?;
    let (input, value) = opt(preceded(tag("="), preceded(span0, cut(rvalue))))(input)?;
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
    use crate::format::{Primitive, RValue};

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
                    value: Some(RValue::Primitive(Primitive::Integer(1))),
                }
            ))
        );
        assert_eq!(
            argument("a = 1 "),
            Ok((
                " ",
                Argument {
                    name: "a".to_string(),
                    value: Some(RValue::Primitive(Primitive::Integer(1))),
                }
            ))
        );
        assert_eq!(
            argument(r#"foo = "bar" "#),
            Ok((
                " ",
                Argument {
                    name: "foo".to_string(),
                    value: Some(RValue::Primitive(Primitive::String("bar".to_string()))),
                }
            ))
        );
    }
}
