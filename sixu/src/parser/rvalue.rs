use nom::branch::alt;
use nom::combinator::cut;
use nom::error::context;
use nom::Parser;

use crate::result::ParseResult;

use super::primitive::primitive;
use super::variable::variable;
use super::RValue;

pub fn rvalue(input: &str) -> ParseResult<&str, RValue> {
    context("rvalue", alt((primitive_value, cut(variable_value)))).parse(input)
}

pub fn primitive_value(input: &str) -> ParseResult<&str, RValue> {
    let (input, p) = primitive.parse(input)?;
    Ok((input, RValue::Literal(p)))
}

pub fn variable_value(input: &str) -> ParseResult<&str, RValue> {
    let (input, variable) = variable.parse(input)?;
    Ok((input, RValue::Variable(variable)))
}

#[cfg(test)]
mod tests {
    use crate::format::{Literal, RValue, Variable};

    use super::*;

    #[test]
    fn test_rvalue() {
        assert_eq!(rvalue("1"), Ok(("", RValue::Literal(Literal::Integer(1)))));
        assert_eq!(
            rvalue("a"),
            Ok((
                "",
                RValue::Variable(Variable {
                    chain: vec!["a".to_string()]
                })
            ))
        );
        assert_eq!(
            rvalue("foo.bar"),
            Ok((
                "",
                RValue::Variable(Variable {
                    chain: vec!["foo".to_string(), "bar".to_string()]
                })
            ))
        );
    }
}
