use nom::branch::alt;
use nom::bytes::complete::*;
use nom::multi::many0;
use nom::sequence::*;
use nom::IResult;

use super::comment::span0;
use super::line::line;
use super::Block;

pub fn block(input: &str) -> IResult<&str, Block> {
    let (input, _) = tag("{")(input)?;
    let (input, children) = many0(preceded(span0, alt((line, line))))(input)?;
    let (input, _) = span0(input)?;
    let (input, _) = tag("}")(input)?;
    Ok((
        input,
        Block {
            attributes: vec![],
            children,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::format::{Argument, Child, CommandLine, Primitive};

    use super::*;

    #[test]
    fn test_block() {
        assert_eq!(
            block("{}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![],
                }
            ))
        );
        assert_eq!(
            block("{\n}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![],
                }
            ))
        );
        assert_eq!(
            block("{\n@command foo=false}"),
            Ok((
                "",
                Block {
                    attributes: vec![],
                    children: vec![Child::CommandLine(CommandLine {
                        command: "command".to_string(),
                        flags: vec![],
                        arguments: vec![Argument {
                            name: "foo".to_string(),
                            value: Some(Primitive::Boolean(false)),
                        }],
                    })],
                }
            ))
        );
    }
}
