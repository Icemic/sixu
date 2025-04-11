use nom::bytes::complete::*;
use nom::combinator::*;
use nom::sequence::*;
use nom::Parser;

use crate::result::SixuResult;

use super::block::block;
use super::comment::span0;
use super::identifier::identifier;
use super::parameter::parameters;
use super::Scene;

pub fn scene(input: &str) -> SixuResult<&str, Scene> {
    let (input, _) = tag("::").parse(input)?;
    let (input, name) = cut(identifier).parse(input)?;
    let (input, parameters) = delimited(span0, opt(parameters), span0).parse(input)?;
    let (input, block) = preceded(span0, cut(block)).parse(input)?;
    Ok((
        input,
        Scene {
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
    fn test_scene() {
        assert_eq!(
            scene("::a {}"),
            Ok((
                "",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            scene("::a { }"),
            Ok((
                "",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            scene("::a { } "),
            Ok((
                " ",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            scene("::a { } // comment"),
            Ok((
                " // comment",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            scene("::a \n// comment\n { } // comment"),
            Ok((
                " // comment",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
        assert_eq!(
            scene("::a {\n@command\n}"),
            Ok((
                "",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Block {
                        children: vec![Child {
                            attributes: vec![],
                            content: ChildContent::CommandLine(CommandLine {
                                command: "command".to_string(),
                                flags: vec![],
                                arguments: vec![],
                            }),
                        }]
                    },
                }
            ))
        );
    }
}
