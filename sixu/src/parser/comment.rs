use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::error::ParseError;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::format::LineMarker;
use crate::result::ParseResult;

/// parse comment like `// C++/EOL-style comments`
pub fn comment(input: &str) -> ParseResult<&str, &str> {
    alt((comment_single, comment_multi)).parse(input)
}

/// parse comment like `// C++/EOL-style comments`
pub fn comment_single(input: &str) -> ParseResult<&str, &str> {
    preceded(tag("//"), cut(is_not("\r\n"))).parse(input)
}

pub fn marker_directive_comment(input: &str) -> ParseResult<&str, LineMarker> {
    let (input, _) = tag("//#marker id=").parse(input)?;
    let (input, id) = cut(take_while1(|ch: char| ch.is_ascii_alphanumeric() || ch == '_')).parse(input)?;
    let (input, _) = opt(line_ending).parse(input)?;

    let marker = LineMarker::parse_id(id).ok_or_else(|| {
        nom::Err::Error(nom_language::error::VerboseError::from_error_kind(
            input,
            nom::error::ErrorKind::Tag,
        ))
    })?;

    Ok((input, marker))
}

/**
 * parse comment like `/* C-style comments */`
 * allows multiple * at the beginning and end of the comment
 * and allows multiple lines in the comment, including starts with `*` and ends with `*`
 * and allows empty comment, which is `/* */`
 * returns purely the comment content, without `/*` and `*/`
*/
pub fn comment_multi(input: &str) -> ParseResult<&str, &str> {
    delimited(tag("/*"), take_until("*/"), tag("*/")).parse(input)
}

/// match contiguous comments or whitespaces, which can be multiple lines
pub fn span0(input: &str) -> ParseResult<&str, ()> {
    value(
        (),
        many0(alt((map(comment, |_| ()), value((), multispace1)))),
    )
    .parse(input)
}

/// match contiguous comments or whitespaces, which is only one line
pub fn span0_inline(input: &str) -> ParseResult<&str, ()> {
    value((), many0(alt((map(comment, |_| ()), value((), space1))))).parse(input)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_comment() {
        assert_eq!(comment("// comment"), Ok(("", " comment")));
        assert_eq!(comment("// comment\n"), Ok(("\n", " comment")));
        assert_eq!(comment("// comment\nnext"), Ok(("\nnext", " comment")));
        assert_eq!(comment("// comment\nnext\n"), Ok(("\nnext\n", " comment")));
    }

    #[test]
    fn test_marker_directive_comment() {
        assert_eq!(
            marker_directive_comment("//#marker id=Labc123\nnext"),
            Ok((
                "next",
                LineMarker {
                    id: "Labc123".to_string(),
                },
            ))
        );
        assert!(marker_directive_comment("// #marker id=Labc123\n").is_err());
        assert!(marker_directive_comment("//#marker id=\n").is_err());
    }

    #[test]
    fn test_comment_multi() {
        assert_eq!(comment_multi("/* comment */"), Ok(("", " comment ")));
        assert_eq!(comment_multi("/** comment */"), Ok(("", "* comment ")));
        assert_eq!(
            comment_multi("/* comment \n next */"),
            Ok(("", " comment \n next "))
        );
        assert_eq!(
            comment_multi("/* comment \n * next */"),
            Ok(("", " comment \n * next "))
        );
        assert_eq!(comment_multi("/* \n */"), Ok(("", " \n ")));
    }

    #[test]
    fn test_comment_or_multispace0() {
        assert_eq!(span0("// comment"), Ok(("", ())));
        assert_eq!(span0("// comment\n"), Ok(("", ())));
        assert_eq!(span0("// comment\n// comment"), Ok(("", ())));
        assert_eq!(span0("// comment\n// comment\n"), Ok(("", ())));
        assert_eq!(span0("// comment\nnext"), Ok(("next", ())));
        assert_eq!(span0("// comment\nnext\n"), Ok(("next\n", ())));
        assert_eq!(span0(""), Ok(("", ())));
        assert_eq!(span0(" "), Ok(("", ())));
        assert_eq!(span0("\n"), Ok(("", ())));
        assert_eq!(span0("\t"), Ok(("", ())));
        assert_eq!(span0("\r"), Ok(("", ())));
        assert_eq!(span0("  "), Ok(("", ())));
        assert_eq!(span0("\n\n"), Ok(("", ())));
        assert_eq!(span0("\t\t"), Ok(("", ())));
        assert_eq!(span0("\r\r"), Ok(("", ())));
        assert_eq!(span0("  \n"), Ok(("", ())));
        assert_eq!(span0("\n  "), Ok(("", ())));
        assert_eq!(span0("\t\n"), Ok(("", ())));
        assert_eq!(span0("\n\t"), Ok(("", ())));
        assert_eq!(span0("\r\n"), Ok(("", ())));
        assert_eq!(span0("\n\r"), Ok(("", ())));
        assert_eq!(span0("  \n  "), Ok(("", ())));
        assert_eq!(span0("\n\n\n"), Ok(("", ())));
        assert_eq!(span0("\t\t\t"), Ok(("", ())));
        assert_eq!(span0("\r\r\r"), Ok(("", ())));
        assert_eq!(span0("  \n  \n  "), Ok(("", ())));
    }
}
