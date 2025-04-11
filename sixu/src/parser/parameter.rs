use nom::bytes::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::result::SixuResult;

use super::comment::span0;
use super::identifier::identifier;
use super::primitive::primitive;
use super::Parameter;

pub fn parameters(input: &str) -> SixuResult<&str, Vec<Parameter>> {
    let (input, _) = tag("(").parse(input)?;
    let (input, _) = span0.parse(input)?;
    let (input, parameters) = cut(separated_list0(
        delimited(span0, tag(","), span0),
        cut(parameter),
    ))
    .parse(input)?;
    let (input, _) = span0.parse(input)?;
    let (input, _) = tag(")").parse(input)?;
    Ok((input, parameters))
}

pub fn parameter(input: &str) -> SixuResult<&str, Parameter> {
    let (input, name) = identifier.parse(input)?;
    let (input, _) = span0.parse(input)?;
    let (input, default_value) =
        cut(opt(preceded(tag("="), preceded(span0, cut(primitive))))).parse(input)?;
    Ok((
        input,
        Parameter {
            name: name.to_string(),
            default_value,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::Primitive;

    use super::*;

    #[test]
    fn test_parameters() {
        assert_eq!(
            parameters("(a)"),
            Ok((
                "",
                vec![Parameter {
                    name: "a".to_string(),
                    default_value: None,
                }]
            ))
        );
        assert_eq!(
            parameters("(a, b)"),
            Ok((
                "",
                vec![
                    Parameter {
                        name: "a".to_string(),
                        default_value: None,
                    },
                    Parameter {
                        name: "b".to_string(),
                        default_value: None,
                    },
                ]
            ))
        );
        assert_eq!(
            parameters("(a, b, c)"),
            Ok((
                "",
                vec![
                    Parameter {
                        name: "a".to_string(),
                        default_value: None,
                    },
                    Parameter {
                        name: "b".to_string(),
                        default_value: None,
                    },
                    Parameter {
                        name: "c".to_string(),
                        default_value: None,
                    },
                ]
            ))
        );

        assert_eq!(
            parameters("(a=1)"),
            Ok((
                "",
                vec![Parameter {
                    name: "a".to_string(),
                    default_value: Some(Primitive::Integer(1)),
                }]
            ))
        );
        assert_eq!(
            parameters(r#"(a=1, b="2")"#),
            Ok((
                "",
                vec![
                    Parameter {
                        name: "a".to_string(),
                        default_value: Some(Primitive::Integer(1)),
                    },
                    Parameter {
                        name: "b".to_string(),
                        default_value: Some(Primitive::String("2".to_string())),
                    },
                ]
            ))
        );
        assert_eq!(
            parameters(r#"( a=   1, b  =  "2"  )"#),
            Ok((
                "",
                vec![
                    Parameter {
                        name: "a".to_string(),
                        default_value: Some(Primitive::Integer(1)),
                    },
                    Parameter {
                        name: "b".to_string(),
                        default_value: Some(Primitive::String("2".to_string())),
                    },
                ]
            ))
        );
        assert_eq!(
            parameters(r#"( a=   1, _c, b  =  "2"  )"#),
            Ok((
                "",
                vec![
                    Parameter {
                        name: "a".to_string(),
                        default_value: Some(Primitive::Integer(1)),
                    },
                    Parameter {
                        name: "_c".to_string(),
                        default_value: None,
                    },
                    Parameter {
                        name: "b".to_string(),
                        default_value: Some(Primitive::String("2".to_string())),
                    },
                ]
            ))
        );
        assert_eq!(
            parameters(
                "( \n// comment\na=   1, \n// comment\n_c\n// comment\n, b\
              \n// comment\n  = \n// comment\n \"2\" \n// comment\n )"
            ),
            Ok((
                "",
                vec![
                    Parameter {
                        name: "a".to_string(),
                        default_value: Some(Primitive::Integer(1)),
                    },
                    Parameter {
                        name: "_c".to_string(),
                        default_value: None,
                    },
                    Parameter {
                        name: "b".to_string(),
                        default_value: Some(Primitive::String("2".to_string())),
                    },
                ]
            ))
        );
    }
}
