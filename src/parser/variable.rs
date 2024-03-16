use nom::bytes::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::Parser;

use crate::result::SixuResult;

use super::identifier::identifier;
use super::Variable;

/// parse a variable like "foo" or "foo.bar.a.b"
pub fn variable(input: &str) -> SixuResult<&str, Variable> {
    let (input, chain) = map_res(
        separated_list1(tag("."), cut(identifier)),
        |v: Vec<&str>| -> Result<Vec<String>, std::convert::Infallible> {
            Ok(v.iter().map(|s| s.to_string()).collect())
        },
    )
    .parse(input)?;
    Ok((input, Variable { chain }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable() {
        assert_eq!(
            variable("a"),
            Ok((
                "",
                Variable {
                    chain: vec!["a".to_string()]
                }
            ))
        );
        assert_eq!(
            variable("a.b"),
            Ok((
                "",
                Variable {
                    chain: vec!["a".to_string(), "b".to_string()]
                }
            ))
        );
        assert_eq!(
            variable("a.b.c"),
            Ok((
                "",
                Variable {
                    chain: vec!["a".to_string(), "b".to_string(), "c".to_string()]
                }
            ))
        );
    }
}
