use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::context;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::parser::comment::span0_inline;
use crate::result::ParseResult;

use super::Literal;

pub fn primitive(input: &str) -> ParseResult<&str, Literal> {
    context("primitive", alt((string, float, integer, boolean, array))).parse(input)
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

// all integer, supports decimal and hexadecimal (0x/0X prefix)
pub fn integer(input: &str) -> ParseResult<&str, Literal> {
    let (input, n) = context(
        "integer",
        map_res(
            (
                opt(alt((tag("-"), tag("+")))),
                span0_inline,
                alt((
                    // Hexadecimal: 0x123 or 0X123
                    recognize((
                        alt((tag("0x"), tag("0X"))),
                        many1(terminated(hex_digit1, many0(char('_')))),
                    )),
                    // Decimal: 123
                    recognize(many1(terminated(digit1, many0(char('_'))))),
                )),
            ),
            |(sign, _, value)| {
                let value = &str::replace(value, "_", "");
                let parsed_value = if value.starts_with("0x") || value.starts_with("0X") {
                    // Parse hexadecimal
                    i64::from_str_radix(&value[2..], 16)
                } else {
                    // Parse decimal
                    value.parse::<i64>()
                };
                parsed_value.map(|n| if sign == Some("-") { -n } else { n })
            },
        ),
    )
    .parse(input)?;
    Ok((input, Literal::Integer(n)))
}

// float numbers, supports various formats like 123., 123.0, -123.123, 0.111
pub fn float(input: &str) -> ParseResult<&str, Literal> {
    let (input, f) = context(
        "float",
        map_res(
            (
                opt(alt((tag("-"), tag("+")))),
                span0_inline,
                alt((
                    // Format: 123.456 or 123.
                    recognize((
                        recognize(many1(terminated(digit1, many0(char('_'))))),
                        tag("."),
                        opt(recognize(many1(terminated(digit1, many0(char('_')))))),
                    )),
                    // Format: .123
                    recognize((
                        tag("."),
                        recognize(many1(terminated(digit1, many0(char('_'))))),
                    )),
                )),
            ),
            |(sign, _, value)| {
                let value = &str::replace(value, "_", "");
                value
                    .parse::<f64>()
                    .map(|n| if sign == Some("-") { -n } else { n })
            },
        ),
    )
    .parse(input)?;
    Ok((input, Literal::Float(f)))
}

pub fn boolean(input: &str) -> ParseResult<&str, Literal> {
    let (input, b) = context(
        "boolean",
        alt((value(true, tag("true")), value(false, tag("false")))),
    )
    .parse(input)?;
    Ok((input, Literal::Boolean(b)))
}

// array of primitives, supports nesting
pub fn array(input: &str) -> ParseResult<&str, Literal> {
    let (input, elements) = context(
        "array",
        delimited(
            preceded(tag("["), multispace0),
            terminated(
                separated_list0(
                    delimited(multispace0, tag(","), multispace0),
                    preceded(multispace0, primitive),
                ),
                opt(preceded(multispace0, tag(","))),
            ),
            preceded(multispace0, tag("]")),
        ),
    )
    .parse(input)?;
    Ok((input, Literal::Array(elements)))
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
        assert_eq!(primitive("- 123"), Ok(("", Literal::Integer(-123))));
        assert_eq!(primitive("0123"), Ok(("", Literal::Integer(123))));
        assert_eq!(primitive("123_456"), Ok(("", Literal::Integer(123456))));
        assert_eq!(
            primitive("123_456_789_"),
            Ok(("", Literal::Integer(123456789)))
        );
        // Hexadecimal tests
        assert_eq!(primitive("0x123"), Ok(("", Literal::Integer(0x123))));
        assert_eq!(primitive("0X123"), Ok(("", Literal::Integer(0x123))));
        assert_eq!(primitive("-0x123"), Ok(("", Literal::Integer(-0x123))));
        assert_eq!(primitive("+0xff"), Ok(("", Literal::Integer(0xff))));
        assert_eq!(primitive("0xFF"), Ok(("", Literal::Integer(0xFF))));
        assert_eq!(primitive("0xAB_CD"), Ok(("", Literal::Integer(0xABCD))));
        assert_eq!(primitive("0x0"), Ok(("", Literal::Integer(0))));
        assert_eq!(primitive("123."), Ok(("", Literal::Float(123.))));
        assert_eq!(primitive("123.0"), Ok(("", Literal::Float(123.0))));
        assert_eq!(primitive("123.456"), Ok(("", Literal::Float(123.456))));
        assert_eq!(primitive("- 123.123"), Ok(("", Literal::Float(-123.123))));
        assert_eq!(primitive("+123.456"), Ok(("", Literal::Float(123.456))));
        assert_eq!(primitive("0.111"), Ok(("", Literal::Float(0.111))));
        assert_eq!(primitive(".123"), Ok(("", Literal::Float(0.123))));
        assert_eq!(primitive("-.456"), Ok(("", Literal::Float(-0.456))));
        assert_eq!(primitive("- .456"), Ok(("", Literal::Float(-0.456))));
        assert_eq!(primitive("12_3.45_6"), Ok(("", Literal::Float(123.456))));
        assert_eq!(primitive("0."), Ok(("", Literal::Float(0.))));
        // Array tests
        assert_eq!(primitive("[]"), Ok(("", Literal::Array(vec![]))));
        assert_eq!(
            primitive("[1]"),
            Ok(("", Literal::Array(vec![Literal::Integer(1)])))
        );
        assert_eq!(
            primitive("[1, 2.0, 3]"),
            Ok((
                "",
                Literal::Array(vec![
                    Literal::Integer(1),
                    Literal::Float(2.0),
                    Literal::Integer(3)
                ])
            ))
        );
        assert_eq!(
            primitive("[1, \"hello\", true]"),
            Ok((
                "",
                Literal::Array(vec![
                    Literal::Integer(1),
                    Literal::String("hello".to_string()),
                    Literal::Boolean(true)
                ])
            ))
        );
        assert_eq!(
            primitive("[1, [2, 3], 4]"),
            Ok((
                "",
                Literal::Array(vec![
                    Literal::Integer(1),
                    Literal::Array(vec![Literal::Integer(2), Literal::Integer(3)]),
                    Literal::Integer(4)
                ])
            ))
        );
        assert_eq!(
            primitive("[1, 2, 3,]"),
            Ok((
                "",
                Literal::Array(vec![
                    Literal::Integer(1),
                    Literal::Integer(2),
                    Literal::Integer(3)
                ])
            ))
        );
        assert_eq!(
            primitive("[ 1 , 2 , 3 ]"),
            Ok((
                "",
                Literal::Array(vec![
                    Literal::Integer(1),
                    Literal::Integer(2),
                    Literal::Integer(3)
                ])
            ))
        );
        assert_eq!(
            primitive("_123"),
            Err(Err::Error(VerboseError {
                errors: vec![
                    ("_123", VerboseErrorKind::Nom(nom::error::ErrorKind::Tag)),
                    ("_123", VerboseErrorKind::Context("array")),
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
