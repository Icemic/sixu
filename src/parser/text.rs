use nom::bytes::complete::take_till;
use nom::character::complete::one_of;
use nom::combinator::not;
use nom::error::context;
use nom::sequence::{delimited, preceded};

use crate::format::{Child, ScriptBlock, ScriptLine};
use crate::result::SixuResult;

use super::comment::span0;

pub fn plain_text_line(input: &str) -> SixuResult<&str, Child> {
    let (input, text) = delimited(span0, plain_text, span0)(input)?;

    Ok((
        input,
        Child::ScriptBlock(ScriptBlock {
            attributes: vec![],
            lines: vec![ScriptLine { _content: text }],
        }),
    ))
}

pub fn plain_text(input: &str) -> SixuResult<&str, String> {
    let (input, s) = context(
        "plain_text",
        preceded(not(one_of("}@#")), take_till(|c| c == '\n' || c == '\r')),
    )(input)?;

    Ok((input, s.to_string()))
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
    fn test_plain_text_line() {
        assert_eq!(
            plain_text_line("foo"),
            Ok((
                "",
                Child::ScriptBlock(ScriptBlock {
                    attributes: vec![],
                    lines: vec![ScriptLine {
                        _content: "foo".to_string()
                    }],
                })
            ))
        );
        assert_eq!(
            plain_text_line("foo\n  \r"),
            Ok((
                "",
                Child::ScriptBlock(ScriptBlock {
                    attributes: vec![],
                    lines: vec![ScriptLine {
                        _content: "foo".to_string()
                    }],
                })
            ))
        );
    }
}
