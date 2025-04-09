use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, take_while_m_n};
use nom::character::complete::{char, none_of, not_line_ending, one_of};
use nom::combinator::{map_opt, map_res, not, value};
use nom::error::{context, FromExternalError, ParseError};
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

use crate::format::ChildContent;
use crate::result::SixuResult;

use super::comment::span0;

pub fn text_line(input: &str) -> SixuResult<&str, ChildContent> {
    let (input, text) = delimited(span0, alt((escaped_text, plain_text)), span0)(input)?;

    Ok((input, ChildContent::TextLine(text)))
}

pub fn plain_text(input: &str) -> SixuResult<&str, String> {
    let (input, s) = context("plain_text", preceded(not(one_of("}@#")), not_line_ending))(input)?;

    Ok((input, s.to_string()))
}

pub fn escaped_text(input: &str) -> SixuResult<&str, String> {
    let (input, s) = context(
        "escaped_text",
        alt((
            delimited(
                char('"'),
                escaped_transform(
                    none_of("\"\\"),
                    '\\',
                    alt((
                        parse_unicode,
                        value('\n', char('n')),
                        value('\r', char('r')),
                        value('\t', char('t')),
                        value('\\', char('\\')),
                        value('/', char('/')),
                        value('"', char('"')),
                        value('\'', char('\'')),
                    )),
                ),
                char('"'),
            ),
            delimited(
                char('\''),
                escaped_transform(
                    none_of("\'\\"),
                    '\\',
                    alt((
                        parse_unicode,
                        value('\n', char('n')),
                        value('\r', char('r')),
                        value('\t', char('t')),
                        value('\\', char('\\')),
                        value('/', char('/')),
                        value('"', char('"')),
                        value('\'', char('\'')),
                    )),
                ),
                char('\''),
            ),
        )),
    )(input)?;

    Ok((input, s.to_string()))
}

// from https://github.com/rust-bakery/nom/blob/a44b52ed9052a66f5eb2add9aa5b314f034dc580/examples/string.rs#L30
// with some modifications
fn parse_unicode<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // `take_while_m_n` parses between `m` and `n` bytes (inclusive) that match
    // a predicate. `parse_hex` here parses between 1 and 6 hexadecimal numerals.
    let parse_hex = take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit());
    // `parse_hex2` parses between 1 and 4 hexadecimal numerals.
    // This is used for the `uXXXX` format, which is a single unicode code point.
    let parse_hex2 = take_while_m_n(1, 4, |c: char| c.is_ascii_hexdigit());

    // `preceded` takes a prefix parser, and if it succeeds, returns the result
    // of the body parser. In this case, it parses u{XXXX}.
    let parse_delimited_hex = preceded(
        char('u'),
        // `delimited` is like `preceded`, but it parses both a prefix and a suffix.
        // It returns the result of the middle parser. In this case, it parses
        // {XXXX}, where XXXX is 1 to 6 hex numerals, and returns XXXX
        alt((delimited(char('{'), parse_hex, char('}')), parse_hex2)),
    );

    // `map_res` takes the result of a parser and applies a function that returns
    // a Result. In this case we take the hex bytes from parse_hex and attempt to
    // convert them to a u32.
    let parse_u32 = map_res(parse_delimited_hex, move |hex| u32::from_str_radix(hex, 16));

    // map_opt is like map_res, but it takes an Option instead of a Result. If
    // the function returns None, map_opt returns an error. In this case, because
    // not all u32 values are valid unicode code points, we have to fallibly
    // convert to char with from_u32.
    map_opt(parse_u32, std::char::from_u32).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        assert_eq!(plain_text("foo"), Ok(("", "foo".to_string())));
        assert_eq!(plain_text("foo\n"), Ok(("\n", "foo".to_string())));
        assert_eq!(plain_text("foo\r\n"), Ok(("\r\n", "foo".to_string())));
        assert_eq!(plain_text("foo bar"), Ok(("", "foo bar".to_string())));
    }

    #[test]
    fn test_escaped_text() {
        assert_eq!(escaped_text(r#""foo""#), Ok(("", "foo".to_string())));
        assert_eq!(escaped_text(r#""foo\n""#), Ok(("", "foo\n".to_string())));
        assert_eq!(
            escaped_text(r#""foo\r\n""#),
            Ok(("", "foo\r\n".to_string()))
        );
        assert_eq!(
            escaped_text(r#""foo bar""#),
            Ok(("", "foo bar".to_string()))
        );
        assert_eq!(
            escaped_text(r#""foo\"bar""#),
            Ok(("", "foo\"bar".to_string()))
        );
        assert_eq!(
            escaped_text(r#""foo\'bar""#),
            Ok(("", "foo'bar".to_string()))
        );
        assert_eq!(
            escaped_text(r#""foo'bar""#),
            Ok(("", "foo'bar".to_string()))
        );
        assert_eq!(
            escaped_text(r#""foo\\bar""#),
            Ok(("", "foo\\bar".to_string()))
        );
        assert_eq!(
            escaped_text(r#""foo\u6D4B\u{8BD5}""#),
            Ok(("", "foo测试".to_string()))
        );
    }

    #[test]
    fn test_plain_text_line() {
        assert_eq!(
            text_line("foo"),
            Ok(("", ChildContent::TextLine("foo".to_string())))
        );
        assert_eq!(
            text_line("foo\n  \r"),
            Ok(("", ChildContent::TextLine("foo".to_string())))
        );
    }
}
