use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;
use nom::Parser;

use super::Primitive;

pub fn primitive(input: &str) -> IResult<&str, Primitive> {
    alt((string, number, boolean))(input)
}

pub fn string(input: &str) -> IResult<&str, Primitive> {
    let (input, s) = delimited(tag("\""), take_until("\""), tag("\""))(input)?;
    Ok((input, Primitive::String(s.to_string())))
}

// all integer, which should not start with 0
pub fn number(input: &str) -> IResult<&str, Primitive> {
    // let (input, n) = recognize(many1(terminated(digit1, many0(char('_'))))).parse(input)?;
    let (input, n) = map_res(
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
    )
    .parse(input)?;
    Ok((input, Primitive::Integer(n)))
}

pub fn boolean(input: &str) -> IResult<&str, Primitive> {
    let (input, b) = alt((tag("true"), tag("false")))(input)?;
    Ok((input, Primitive::Boolean(b == "true")))
}

#[cfg(test)]
mod tests {
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
            Err(Err::Error(nom::error::Error {
                input: "_123",
                code: nom::error::ErrorKind::Tag
            }))
        );
        assert_eq!(
            primitive("\"hello\""),
            Ok(("", Primitive::String("hello".to_string())))
        );
        // assert_eq!(
        //     primitive(r#"\"hello\""#),
        //     Ok(("", Primitive::String("\"hello\"".to_string())))
        // );
    }
}
