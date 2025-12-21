use nom::bytes::complete::*;
use nom::combinator::*;
use nom::sequence::*;
use nom::Parser;

use crate::result::ParseResult;

use super::block::block;
use super::comment::span0;
use super::identifier::identifier;
use super::parameter::parameters;
use super::Paragraph;

pub fn paragraph(input: &str) -> ParseResult<&str, Paragraph> {
    let (input, _) = tag("::").parse(input)?;
    let (input, name) = cut(identifier).parse(input)?;
    let (input, parameters) = delimited(span0, opt(parameters), span0).parse(input)?;
    let (input, block) = preceded(span0, cut(block)).parse(input)?;
    Ok((
        input,
        Paragraph {
            name: name.to_string(),
            parameters: parameters.unwrap_or_default(),
            block,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::{Block, Child, ChildContent, CommandLine};

    use super::*;

    #[test]
    fn test_paragraph() {
        assert_eq!(
            paragraph("::a {}"),
            Ok((
                "",
                Paragraph {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            paragraph("::a { }"),
            Ok((
                "",
                Paragraph {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            paragraph("::a { } "),
            Ok((
                " ",
                Paragraph {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            paragraph("::a { } // comment"),
            Ok((
                " // comment",
                Paragraph {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            paragraph("::a \n// comment\n { } // comment"),
            Ok((
                " // comment",
                Paragraph {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            paragraph("::a {\n@command\n}"),
            Ok((
                "",
                Paragraph {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Block {
                        children: vec![Child {
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                arguments: vec![],
                            }),
                        }]
                    },
                }
            ))
        );
    }
}
