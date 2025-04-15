use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::result::ParseResult;

pub fn identifier(input: &str) -> ParseResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        cut(many0(alt((alphanumeric1, tag("_"))))),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::error::ErrorKind;
    use nom::Err;
    use nom_language::error::{VerboseError, VerboseErrorKind};

    use super::*;

    #[test]
    fn test_identifier() {
        assert_eq!(identifier("a"), Ok(("", "a")));
        assert_eq!(identifier("a_"), Ok(("", "a_")));
        assert_eq!(identifier("a0"), Ok(("", "a0")));
        assert_eq!(identifier("a_0"), Ok(("", "a_0")));
        assert_eq!(identifier("_a"), Ok(("", "_a")));
        assert_eq!(identifier("_a0"), Ok(("", "_a0")));
        assert_eq!(identifier("_a_0"), Ok(("", "_a_0")));
        assert_eq!(identifier("_"), Ok(("", "_")));
        assert_eq!(identifier("_0"), Ok(("", "_0")));
        assert_eq!(identifier("_0_"), Ok(("", "_0_")));
        assert_eq!(identifier("_0_1"), Ok(("", "_0_1")));

        assert_eq!(
            identifier("0a"),
            Err(Err::Error(VerboseError {
                errors: vec![
                    ("0a", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("0a", VerboseErrorKind::Nom(ErrorKind::Alt))
                ]
            }))
        );
    }
}
