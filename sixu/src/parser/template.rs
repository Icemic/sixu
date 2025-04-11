use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, tag};
use nom::character::complete::{char, none_of};
use nom::combinator::{cut, map_res, value};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::{delimited, preceded};

use crate::format::{ChildContent, RValue, TemplateLiteral};
use crate::result::SixuResult;

use super::comment::span0;
use super::rvalue::rvalue;
use super::text::parse_unicode;

enum TemplateLiteralPart {
    Text(String),
    Value(RValue),
}

pub fn template_line(input: &str) -> SixuResult<&str, ChildContent> {
    let (input, template) = preceded(span0, template_literal)(input)?;

    Ok((input, ChildContent::TemplateLiteral(template)))
}

/// parse template literals like the same as JS, but only support primitive types or variable reference,
/// expression is not supported yet.
pub fn template_literal(input: &str) -> SixuResult<&str, TemplateLiteral> {
    let escaped_text = context(
        "escaped_text",
        map_res(
            escaped_transform(
                none_of("`$\\"),
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
            ),
            |s: String| {
                Ok::<TemplateLiteralPart, nom::error::VerboseError<&str>>(
                    TemplateLiteralPart::Text(s),
                )
            },
        ),
    );

    let value = context(
        "expression",
        map_res(delimited(tag("${"), cut(rvalue), char('}')), |v| {
            Ok::<TemplateLiteralPart, nom::error::VerboseError<&str>>(TemplateLiteralPart::Value(v))
        }),
    );

    let (input, s) = context(
        "template_literal",
        delimited(char('`'), cut(many0(alt((escaped_text, value)))), char('`')),
    )(input)?;

    let mut strings = vec![];
    let mut values = vec![];

    for part in s {
        match part {
            TemplateLiteralPart::Text(s) => strings.push(s),
            TemplateLiteralPart::Value(v) => values.push(v),
        }
    }

    Ok((input, TemplateLiteral { strings, values }))
}

#[cfg(test)]
mod tests {
    use crate::format::{Primitive, Variable};

    use super::*;

    #[test]
    fn test_template_literal() {
        let input = "`hello \n${world} ${123} world`";
        let (remaining, result) = template_literal(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(result.strings, vec!["hello \n", " ", " world"]);
        assert_eq!(
            result.values,
            vec![
                RValue::Variable(Variable {
                    chain: vec!["world".to_string()],
                }),
                RValue::Primitive(Primitive::Integer(123)),
            ]
        );
    }

    #[test]
    fn test_template_line() {
        let input = "  \n `hello \n${world} ${123} world` \n";
        let (remaining, result) = template_line(input).unwrap();
        assert_eq!(remaining, " \n");
        assert_eq!(
            result,
            ChildContent::TemplateLiteral(TemplateLiteral {
                strings: vec![
                    "hello \n".to_string(),
                    " ".to_string(),
                    " world".to_string()
                ],
                values: vec![
                    RValue::Variable(Variable {
                        chain: vec!["world".to_string()],
                    }),
                    RValue::Primitive(Primitive::Integer(123)),
                ],
            })
        );
    }
}
