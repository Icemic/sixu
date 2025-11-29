use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, take_while, take_while1, take_while_m_n};
use nom::character::complete::{char, none_of, one_of};
use nom::combinator::{cut, map_opt, map_res, not, opt, peek, success, value};
use nom::error::{context, FromExternalError, ParseError};
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

use crate::format::{ChildContent, LeadingText, TailingText, TemplateLiteral, Text};
use crate::result::ParseResult;

use super::comment::{span0, span0_inline};
use super::template::template_literal;

/// Parse tailing text in the format #<non-whitespace-chars>
/// Example: #tag, #tag_123, #标签, #tag-name.ext
pub fn tailing_text(input: &str) -> ParseResult<&str, TailingText> {
    let mut parser = opt(preceded(
        char('#'),
        take_while1(|c: char| !c.is_whitespace()),
    ));

    let (remaining, result) = parser.parse(input)?;

    match result {
        Some(tag) => Ok((remaining, TailingText::Text(tag.to_string()))),
        None => Ok((remaining, TailingText::None)),
    }
}

pub fn text_line(input: &str) -> ParseResult<&str, ChildContent> {
    let (input, (_, _, leading, _, text, _, tailing)) = delimited(
        span0,
        (
            not(one_of("}@#")),
            span0_inline,
            alt((leading_text, success(LeadingText::None))),
            span0_inline,
            text,
            span0_inline,
            alt((tailing_text, success(TailingText::None))),
        ),
        span0,
    )
    .parse(input)?;

    Ok((input, ChildContent::TextLine(leading, text, tailing)))
}

pub fn leading_text(input: &str) -> ParseResult<&str, LeadingText> {
    context(
        "leading_text",
        delimited(
            one_of("["),
            alt((
                map_res(
                    // force quotes to be adjacent to the ] symbol to ensure that
                    // there is only one set of escaped text inside, otherwise it fails,
                    // fallback to plain text
                    (
                        span0_inline,
                        template_literal,
                        span0_inline,
                        peek(one_of("]")),
                    ),
                    |s: ((), TemplateLiteral, (), char)| {
                        Ok::<LeadingText, nom::error::Error<&str>>(LeadingText::TemplateLiteral(
                            s.1,
                        ))
                    },
                ),
                map_res(
                    // force quotes to be adjacent to the ] symbol to ensure that
                    // there is only one set of escaped text inside, otherwise it fails,
                    // fallback to plain text
                    (span0_inline, escaped_text, span0_inline, peek(one_of("]"))),
                    |s: ((), String, (), char)| {
                        Ok::<LeadingText, nom::error::Error<&str>>(LeadingText::Text(s.1))
                    },
                ),
                map_res(
                    take_while(|c| c != ']' && c != '\n' && c != '\r'),
                    |s: &str| {
                        Ok::<LeadingText, nom::error::Error<&str>>(LeadingText::Text(s.to_string()))
                    },
                ),
            )),
            char(']'),
        ),
    )
    .parse(input)
}

pub fn text(input: &str) -> ParseResult<&str, Text> {
    context(
        "text",
        alt((
            map_res(template_literal, |s| {
                Ok::<Text, nom::error::Error<&str>>(Text::TemplateLiteral(s))
            }),
            map_res(escaped_text, |s| {
                Ok::<Text, nom::error::Error<&str>>(Text::Text(s))
            }),
            map_res(plain_text, |s| {
                Ok::<Text, nom::error::Error<&str>>(Text::Text(s))
            }),
        )),
    )
    .parse(input)
}

pub fn plain_text(input: &str) -> ParseResult<&str, String> {
    // Find the end of plain text, which is either:
    // 1. A newline character
    // 2. A '#' followed by a non-whitespace character (tailing text)

    let mut end_pos = 0;
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        // Stop at newline
        if ch == '\n' || ch == '\r' {
            break;
        }

        // Check if this is a '#' followed by non-whitespace (potential tailing text)
        if ch == '#' {
            // Check if there's a next character and it's not whitespace
            if i + 1 < chars.len() && !chars[i + 1].is_whitespace() {
                // This is the start of tailing text, stop here
                break;
            }
        }

        end_pos = i + 1;
    }

    if end_pos == 0 {
        // Empty text is still valid
        return Ok((input, String::new()));
    }

    let (text, remaining) = input.split_at(
        input
            .char_indices()
            .nth(end_pos)
            .map(|(pos, _)| pos)
            .unwrap_or(input.len()),
    );

    Ok((remaining, text.to_string()))
}

pub fn escaped_text(input: &str) -> ParseResult<&str, String> {
    let (input, s) = context(
        "escaped_text",
        alt((
            delimited(
                char('"'),
                cut(escaped_transform(
                    none_of("\"\\\n\r"),
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
                        value('`', char('`')),
                    )),
                )),
                char('"'),
            ),
            delimited(
                char('\''),
                cut(escaped_transform(
                    none_of("\'\\\n\r"),
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
                        value('`', char('`')),
                    )),
                )),
                char('\''),
            ),
        )),
    )
    .parse(input)?;

    Ok((input, s.to_string()))
}

// from https://github.com/rust-bakery/nom/blob/a44b52ed9052a66f5eb2add9aa5b314f034dc580/examples/string.rs#L30
// with some modifications
pub(crate) fn parse_unicode<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
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
    use crate::format::{Literal, RValue, TemplateLiteralPart, Text, Variable};

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
    fn test_leading_text() {
        assert_eq!(
            leading_text("[foo]"),
            Ok(("", LeadingText::Text("foo".to_string())))
        );
        assert_eq!(
            leading_text("[foo bar ]"),
            Ok(("", LeadingText::Text("foo bar ".to_string())))
        );
        assert_eq!(
            leading_text("['foo bar']"),
            Ok(("", LeadingText::Text("foo bar".to_string())))
        );
        assert_eq!(
            leading_text(r#"[foo"bar]"#),
            Ok(("", LeadingText::Text("foo\"bar".to_string())))
        );
        assert_eq!(
            leading_text(r#"[foo'bar]"#),
            Ok(("", LeadingText::Text("foo'bar".to_string())))
        );
        assert_eq!(
            leading_text(r#"[foo\\bar]"#),
            Ok(("", LeadingText::Text("foo\\\\bar".to_string())))
        );
        assert_eq!(
            leading_text(r#"['foo\u6D4B\u{8BD5}']"#),
            Ok(("", LeadingText::Text("foo测试".to_string())))
        );
    }

    #[test]
    fn test_plain_text_line() {
        assert_eq!(
            text_line("foo"),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("foo".to_string()),
                    TailingText::None
                )
            ))
        );
        assert_eq!(
            text_line("foo\n  \r"),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("foo".to_string()),
                    TailingText::None
                )
            ))
        );
    }

    #[test]
    fn test_escaped_text_line() {
        assert_eq!(
            text_line(r#""foo\u6D4B\u{8BD5}""#),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("foo测试".to_string()),
                    TailingText::None
                )
            ))
        );
    }

    #[test]
    fn test_leading_text_line() {
        assert_eq!(
            text_line("[foo] aaaaaa"),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::Text("foo".to_string()),
                    Text::Text("aaaaaa".to_string()),
                    TailingText::None
                )
            ))
        );
        assert_eq!(
            text_line("[foo bar] aaaaaa"),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::Text("foo bar".to_string()),
                    Text::Text("aaaaaa".to_string()),
                    TailingText::None
                )
            ))
        );
        // backslash in plain text will be preserved
        assert_eq!(
            text_line(r#"[foo bar] aaa\aaa"#),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::Text("foo bar".to_string()),
                    Text::Text(r#"aaa\aaa"#.to_string()),
                    TailingText::None
                )
            ))
        );
        assert_eq!(
            text_line(r#"[foo bar] aaa\n\raaa"#),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::Text("foo bar".to_string()),
                    Text::Text(r#"aaa\n\raaa"#.to_string()),
                    TailingText::None
                )
            ))
        );
        assert_eq!(
            text_line(r#"aaa\n\raaa"#),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text(r#"aaa\n\raaa"#.to_string()),
                    TailingText::None
                )
            ))
        );
        // spaces around the plain text will not be trimmed
        assert_eq!(
            text_line("[ foo bar ] aaaaaa\n"),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::Text(" foo bar ".to_string()),
                    Text::Text("aaaaaa".to_string()),
                    TailingText::None
                )
            ))
        );
        // spaces around the quoted text are ignored
        assert_eq!(
            text_line("[ 'foo bar' ] \naaaaaa\r\n"),
            Ok((
                "aaaaaa\r\n",
                ChildContent::TextLine(
                    LeadingText::Text("foo bar".to_string()),
                    Text::Text("".to_string()),
                    TailingText::None
                )
            ))
        );
        // only one set of quotes is allowed, or it will fallback to plain text
        assert_eq!(
            text_line("[ 'foo bar' ''] \naaaaaa\r\n"),
            Ok((
                "aaaaaa\r\n",
                ChildContent::TextLine(
                    LeadingText::Text(" 'foo bar' ''".to_string()),
                    Text::Text("".to_string()),
                    TailingText::None
                )
            ))
        );
        // use template literal in leading text
        assert_eq!(
            text_line("[ `foo ${bar}` ] \naaaaaa\r\n"),
            Ok((
                "aaaaaa\r\n",
                ChildContent::TextLine(
                    LeadingText::TemplateLiteral(TemplateLiteral {
                        parts: vec![
                            TemplateLiteralPart::Text("foo ".to_string()),
                            TemplateLiteralPart::Value(RValue::Variable(Variable {
                                chain: vec!["bar".to_string()],
                            })),
                        ],
                    }),
                    Text::Text("".to_string()),
                    TailingText::None
                )
            ))
        );
    }

    #[test]
    fn test_template_line() {
        let input = "  \n `hello \n${world} ${123} world` \n";
        let (remaining, result) = text_line.parse(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(
            result,
            ChildContent::TextLine(
                LeadingText::None,
                Text::TemplateLiteral(TemplateLiteral {
                    parts: vec![
                        TemplateLiteralPart::Text("hello \n".to_string()),
                        TemplateLiteralPart::Value(RValue::Variable(Variable {
                            chain: vec!["world".to_string()],
                        })),
                        TemplateLiteralPart::Text(" ".to_string()),
                        TemplateLiteralPart::Value(RValue::Literal(Literal::Integer(123))),
                        TemplateLiteralPart::Text(" world".to_string()),
                    ],
                }),
                TailingText::None
            )
        );
    }

    #[test]
    fn test_tailing_text() {
        // Test with quoted text
        assert_eq!(
            text_line(r##""hello world"#tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world".to_string()),
                    TailingText::Text("tag".to_string())
                )
            ))
        );

        // Test with space before tailing text
        assert_eq!(
            text_line(r##""hello world" #tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world".to_string()),
                    TailingText::Text("tag".to_string())
                )
            ))
        );

        // Test with plain text
        assert_eq!(
            text_line(r##"hello world #tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world ".to_string()),
                    TailingText::Text("tag".to_string())
                )
            ))
        );

        // Test with leading and tailing text
        assert_eq!(
            text_line(r##"[speaker] "dialogue"#tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::Text("speaker".to_string()),
                    Text::Text("dialogue".to_string()),
                    TailingText::Text("tag".to_string())
                )
            ))
        );

        // Test with special characters in tailing text
        assert_eq!(
            text_line(r##""text"#tag_123-abc.xyz"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("text".to_string()),
                    TailingText::Text("tag_123-abc.xyz".to_string())
                )
            ))
        );

        // Test with Unicode in tailing text
        assert_eq!(
            text_line(r##""text"#标签"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("text".to_string()),
                    TailingText::Text("标签".to_string())
                )
            ))
        );

        // Test with template literal
        assert_eq!(
            text_line(r##"`hello ${world}`#tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::TemplateLiteral(TemplateLiteral {
                        parts: vec![
                            TemplateLiteralPart::Text("hello ".to_string()),
                            TemplateLiteralPart::Value(RValue::Variable(Variable {
                                chain: vec!["world".to_string()],
                            })),
                        ],
                    }),
                    TailingText::Text("tag".to_string())
                )
            ))
        );

        // Test without tailing text
        assert_eq!(
            text_line(r#""hello world""#),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world".to_string()),
                    TailingText::None
                )
            ))
        );

        // Test plain text without tailing text
        assert_eq!(
            text_line(r#"hello world"#),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world".to_string()),
                    TailingText::None
                )
            ))
        );

        // Test with # followed by space (should be part of text, not tailing)
        assert_eq!(
            text_line(r##"hello world # not a tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world # not a tag".to_string()),
                    TailingText::None
                )
            ))
        );

        // Test with # at end of line (no non-whitespace after)
        assert_eq!(
            text_line(r##"hello world #"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello world #".to_string()),
                    TailingText::None
                )
            ))
        );

        // Test tailing text after space
        assert_eq!(
            text_line(r##"some text #tag"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("some text ".to_string()),
                    TailingText::Text("tag".to_string())
                )
            ))
        );

        // Test with emoji in tailing text
        assert_eq!(
            text_line(r##""text"#tag😀"##),
            Ok((
                "",
                ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("text".to_string()),
                    TailingText::Text("tag😀".to_string())
                )
            ))
        );
    }
}
