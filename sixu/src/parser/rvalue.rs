use nom::branch::alt;
use nom::combinator::cut;
use nom::error::context;
use nom::Parser;

use crate::result::SixuResult;

use super::primitive::primitive;
use super::variable::variable;
use super::RValue;

pub fn rvalue(input: &str) -> SixuResult<&str, RValue> {
    context("rvalue", alt((primitive_value, cut(variable_value)))).parse(input)
}

pub fn primitive_value(input: &str) -> SixuResult<&str, RValue> {
    let (input, p) = primitive.parse(input)?;
    Ok((input, RValue::Primitive(p)))
}

pub fn variable_value(input: &str) -> SixuResult<&str, RValue> {
    let (input, variable) = variable.parse(input)?;
    Ok((input, RValue::Variable(variable)))
}

#[cfg(test)]
mod tests {
    use crate::format::{Primitive, RValue, Variable};

    use super::*;

    #[test]
    fn test_rvalue() {
        assert_eq!(
            rvalue("1"),
            Ok(("", RValue::Primitive(Primitive::Integer(1))))
        );
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
