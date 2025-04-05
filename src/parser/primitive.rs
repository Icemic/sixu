use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::context;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::result::SixuResult;

use super::Primitive;

pub fn primitive(input: &str) -> SixuResult<&str, Primitive> {
    context("primitive", alt((string, number, boolean)))(input)
}

pub fn string(input: &str) -> SixuResult<&str, Primitive> {
    let (input, s) = context(
        "string",
        alt((
            delimited(tag("\""), take_until("\""), tag("\"")),
            delimited(tag("'"), take_until("'"), tag("'")),
        )),
    )(input)?;
    Ok((input, Primitive::String(s.to_string())))
}

// all integer, which should not start with 0
pub fn number(input: &str) -> SixuResult<&str, Primitive> {
    // let (input, n) = recognize(many1(terminated(digit1, many0(char('_'))))).parse(input)?;
    let (input, n) = context(
        "number",
        map_res(
            tuple((
                opt(alt((tag("-"), tag("+")))),
                recognize(many1(terminated(digit1, many0(char('_'))))),
            )),
            |(sign, value)| {
                let value = &str::replace(value, "_", "");
                value
                    .parse::<i64>()
                    .map(|n| if sign == Some("-") { -n } else { n })
            },
        ),
    )
    .parse(input)?;
    Ok((input, Primitive::Integer(n)))
}

pub fn boolean(input: &str) -> SixuResult<&str, Primitive> {
    let (input, b) = context(
        "boolean",
        alt((value(true, tag("true")), value(false, tag("false")))),
    )(input)?;
    Ok((input, Primitive::Boolean(b)))
}

#[cfg(test)]
mod tests {
    use nom::error::{VerboseError, VerboseErrorKind};
    use nom::Err;

    use super::*;

    #[test]
    fn test_primitive() {
        assert_eq!(primitive("true"), Ok(("", Primitive::Boolean(true))));
        assert_eq!(primitive("false"), Ok(("", Primitive::Boolean(false))));
        assert_eq!(primitive("123"), Ok(("", Primitive::Integer(123))));
        assert_eq!(primitive("+123"), Ok(("", Primitive::Integer(123))));
        assert_eq!(primitive("-123"), Ok(("", Primitive::Integer(-123))));
        assert_eq!(primitive("0123"), Ok(("", Primitive::Integer(123))));
        assert_eq!(primitive("123_456"), Ok(("", Primitive::Integer(123456))));
        assert_eq!(
            primitive("123_456_789_"),
            Ok(("", Primitive::Integer(123456789)))
        );
        assert_eq!(
            primitive("_123"),
            Err(Err::Error(VerboseError {
                errors: vec![
                    ("_123", VerboseErrorKind::Nom(nom::error::ErrorKind::Tag)),
                    ("_123", VerboseErrorKind::Nom(nom::error::ErrorKind::Alt)),
                    ("_123", VerboseErrorKind::Context("boolean")),
                    ("_123", VerboseErrorKind::Nom(nom::error::ErrorKind::Alt)),
                    ("_123", VerboseErrorKind::Context("primitive"))
                ]
            }))
        );
        assert_eq!(
            primitive("\"hello\""),
            Ok(("", Primitive::String("hello".to_string())))
        );
        assert_eq!(
            primitive("'hello'"),
            Ok(("", Primitive::String("hello".to_string())))
        );
    }
}
