use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::context;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::result::ParseResult;

use super::Literal;

pub fn primitive(input: &str) -> ParseResult<&str, Literal> {
    context("primitive", alt((string, number, boolean))).parse(input)
}

pub fn string(input: &str) -> ParseResult<&str, Literal> {
    let (input, s) = context(
        "string",
        alt((
            delimited(tag("\""), take_until("\""), tag("\"")),
            delimited(tag("'"), take_until("'"), tag("'")),
        )),
    )
    .parse(input)?;
    Ok((input, Literal::String(s.to_string())))
}

// all integer, which should not start with 0
pub fn number(input: &str) -> ParseResult<&str, Literal> {
    // let (input, n) = recognize(many1(terminated(digit1, many0(char('_'))))).parse.parse(input)?;
    let (input, n) = context(
        "number",
        map_res(
            (
                opt(alt((tag("-"), tag("+")))),
                recognize(many1(terminated(digit1, many0(char('_'))))),
            ),
            |(sign, value)| {
                let value = &str::replace(value, "_", "");
                value
                    .parse::<i64>()
                    .map(|n| if sign == Some("-") { -n } else { n })
            },
        ),
    )
    .parse(input)?;
    Ok((input, Literal::Integer(n)))
}

pub fn boolean(input: &str) -> ParseResult<&str, Literal> {
    let (input, b) = context(
        "boolean",
        alt((value(true, tag("true")), value(false, tag("false")))),
    )
    .parse(input)?;
    Ok((input, Literal::Boolean(b)))
}

#[cfg(test)]
mod tests {
    use nom::Err;
    use nom_language::error::{VerboseError, VerboseErrorKind};

    use super::*;

    #[test]
    fn test_primitive() {
        assert_eq!(primitive("true"), Ok(("", Literal::Boolean(true))));
        assert_eq!(primitive("false"), Ok(("", Literal::Boolean(false))));
        assert_eq!(primitive("123"), Ok(("", Literal::Integer(123))));
        assert_eq!(primitive("+123"), Ok(("", Literal::Integer(123))));
        assert_eq!(primitive("-123"), Ok(("", Literal::Integer(-123))));
        assert_eq!(primitive("0123"), Ok(("", Literal::Integer(123))));
        assert_eq!(primitive("123_456"), Ok(("", Literal::Integer(123456))));
        assert_eq!(
            primitive("123_456_789_"),
            Ok(("", Literal::Integer(123456789)))
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
            Ok(("", Literal::String("hello".to_string())))
        );
        assert_eq!(
            primitive("'hello'"),
            Ok(("", Literal::String("hello".to_string())))
        );
    }
}
