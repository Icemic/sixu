use nom::branch::alt;
use nom::bytes::complete::*;
use nom::combinator::*;
use nom::multi::{many0, separated_list0};
use nom::sequence::*;
use nom::Parser;

use crate::result::SixuResult;

use super::comment::{span0, span0_inline};
use super::identifier::identifier;
use super::rvalue::rvalue;
use super::Argument;

pub fn arguments(input: &str) -> SixuResult<&str, Vec<Argument>> {
    let (input, _) = span0.parse(input)?;
    let (input, arguments) = cut(alt((arguments_type_a, arguments_type_b))).parse(input)?;
    Ok((input, arguments))
}

pub fn arguments_type_a(input: &str) -> SixuResult<&str, Vec<Argument>> {
    let (input, _) = tag("(").parse(input)?;
    let (input, _) = span0.parse(input)?;
    let (input, arguments) =
        separated_list0(delimited(span0, tag(","), span0), argument).parse(input)?;
    let (input, _) = span0.parse(input)?;
    let (input, _) = tag(")").parse(input)?;
    Ok((input, arguments))
}

pub fn arguments_type_b(input: &str) -> SixuResult<&str, Vec<Argument>> {
    many0(delimited(span0_inline, argument, span0_inline)).parse(input)
}

pub fn argument(input: &str) -> SixuResult<&str, Argument> {
    let (input, name) = identifier.parse(input)?;
    let (input, _) = span0.parse(input)?;
    let (input, value) = cut(opt(preceded(tag("="), preceded(span0, cut(rvalue))))).parse(input)?;
    Ok((
        input,
        Argument {
            name: name.to_string(),
            value,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::{Primitive, RValue, Variable};

    use super::*;

    #[test]
    fn test_argument() {
        assert_eq!(
            argument("a"),
            Ok((
                "",
                Argument {
                    name: "a".to_string(),
                    value: None,
                }
            ))
        );
        assert_eq!(
            argument("a = 1"),
            Ok((
                "",
                Argument {
                    name: "a".to_string(),
                    value: Some(RValue::Primitive(Primitive::Integer(1))),
                }
            ))
        );
        assert_eq!(
            argument("a = 1 "),
            Ok((
                " ",
                Argument {
                    name: "a".to_string(),
                    value: Some(RValue::Primitive(Primitive::Integer(1))),
                }
            ))
        );
        assert_eq!(
            argument(r#"foo = "bar" "#),
            Ok((
                " ",
                Argument {
                    name: "foo".to_string(),
                    value: Some(RValue::Primitive(Primitive::String("bar".to_string()))),
                }
            ))
        );
        assert_eq!(
            argument(r#"foo = foo.bar "#),
            Ok((
                " ",
                Argument {
                    name: "foo".to_string(),
                    value: Some(RValue::Variable(Variable {
                        chain: vec!["foo".to_string(), "bar".to_string()],
                    })),
                }
            ))
        );

        // type a
        assert_eq!(arguments("()"), Ok(("", vec![])));
        assert_eq!(
            arguments("(a=1)"),
            Ok((
                "",
                vec![Argument {
                    name: "a".to_string(),
                    value: Some(RValue::Primitive(Primitive::Integer(1))),
                }]
            ))
        );
        assert_eq!(
            arguments("(a=1, b='aa')"),
            Ok((
                "",
                vec![
                    Argument {
                        name: "a".to_string(),
                        value: Some(RValue::Primitive(Primitive::Integer(1))),
                    },
                    Argument {
                        name: "b".to_string(),
                        value: Some(RValue::Primitive(Primitive::String("aa".to_string()))),
                    }
                ]
            ))
        );

        // type b
        assert_eq!(arguments(""), Ok(("", vec![])));
        assert_eq!(
            arguments("a=1"),
            Ok((
                "",
                vec![Argument {
                    name: "a".to_string(),
                    value: Some(RValue::Primitive(Primitive::Integer(1))),
                }]
            ))
        );
        assert_eq!(
            arguments("a=1 b='aa'"),
            Ok((
                "",
                vec![
                    Argument {
                        name: "a".to_string(),
                        value: Some(RValue::Primitive(Primitive::Integer(1))),
                    },
                    Argument {
                        name: "b".to_string(),
                        value: Some(RValue::Primitive(Primitive::String("aa".to_string()))),
                    }
                ]
            ))
        );
    }
}
