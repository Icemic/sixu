use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;

use crate::result::SixuResult;

/// parse comment like `// C++/EOL-style comments`
pub fn comment(input: &str) -> SixuResult<&str, &str> {
    preceded(tag("//"), cut(is_not("\r\n")))(input)
}

/// match contiguous comments or whitespaces, which can be multiple lines
pub fn span0(input: &str) -> SixuResult<&str, ()> {
    value(
        (),
        many0(alt((map(comment, |_| ()), value((), multispace1)))),
    )(input)
}

/// match contiguous comments or whitespaces, which is only one line
pub fn span0_inline(input: &str) -> SixuResult<&str, ()> {
    value(
        (),
        many0(alt((map(comment, |_| ()), value((), space1)))),
    )(input)
}

/// match contiguous comments or whitespaces, which can be multiple lines
pub fn span1(input: &str) -> SixuResult<&str, ()> {
    value(
        (),
        many1(alt((map(comment, |_| ()), value((), multispace1)))),
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::error::{VerboseError, VerboseErrorKind};
    use nom::Err;

    use super::*;

    #[test]
    fn test_comment() {
        assert_eq!(comment("// comment"), Ok(("", " comment")));
        assert_eq!(comment("// comment\n"), Ok(("\n", " comment")));
        assert_eq!(comment("// comment\nnext"), Ok(("\nnext", " comment")));
        assert_eq!(comment("// comment\nnext\n"), Ok(("\nnext\n", " comment")));
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

    #[test]
    fn test_comment_or_multispace1() {
        assert_eq!(
            span1(""),
            Err(Err::Error(VerboseError {
                errors: vec![
                    ("", VerboseErrorKind::Nom(nom::error::ErrorKind::MultiSpace)),
                    ("", VerboseErrorKind::Nom(nom::error::ErrorKind::Alt)),
                    ("", VerboseErrorKind::Nom(nom::error::ErrorKind::Many1))
                ]
            }))
        );
        assert_eq!(span1(" "), Ok(("", ())));
        assert_eq!(span1("\n"), Ok(("", ())));
    }
}
