use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, tag};
use nom::character::complete::{char, none_of};
use nom::combinator::{cut, map_res, value};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::Parser;

use crate::format::{ChildContent, TemplateLiteral, TemplateLiteralPart};
use crate::result::ParseResult;

use super::comment::span0;
use super::rvalue::rvalue;
use super::text::parse_unicode;

pub fn template_line(input: &str) -> ParseResult<&str, ChildContent> {
    let (input, template) = preceded(span0, template_literal).parse(input)?;

    Ok((input, ChildContent::TemplateLiteral(template)))
}

/// parse template literals like the same as JS, but only support primitive types or variable reference,
/// expression is not supported yet.
pub fn template_literal(input: &str) -> ParseResult<&str, TemplateLiteral> {
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
                Ok::<TemplateLiteralPart, nom::error::Error<&str>>(TemplateLiteralPart::Text(s))
            },
        ),
    );

    let value = context(
        "expression",
        map_res(delimited(tag("${"), cut(rvalue), char('}')), |v| {
            Ok::<TemplateLiteralPart, nom::error::Error<&str>>(TemplateLiteralPart::Value(v))
        }),
    );

    let (input, parts) = context(
        "template_literal",
        delimited(char('`'), cut(many0(alt((escaped_text, value)))), char('`')),
    )
    .parse(input)?;

    Ok((input, TemplateLiteral { parts }))
}

#[cfg(test)]
mod tests {
    use crate::format::{Primitive, RValue, Variable};

    use super::*;

    #[test]
    fn test_template_literal() {
        let input = "`hello \n${world} ${123} world`";
        let (remaining, result) = template_literal.parse(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(result.get_strings(), vec!["hello \n", " ", " world"]);
        assert_eq!(
            result.get_values(),
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
        let (remaining, result) = template_line.parse(input).unwrap();
        assert_eq!(remaining, " \n");
        assert_eq!(
            result,
            ChildContent::TemplateLiteral(TemplateLiteral {
                parts: vec![
                    TemplateLiteralPart::Text("hello \n".to_string()),
                    TemplateLiteralPart::Value(RValue::Variable(Variable {
                        chain: vec!["world".to_string()],
                    })),
                    TemplateLiteralPart::Text(" ".to_string()),
                    TemplateLiteralPart::Value(RValue::Primitive(Primitive::Integer(123))),
                    TemplateLiteralPart::Text(" world".to_string()),
                ],
            })
        );
    }
}
