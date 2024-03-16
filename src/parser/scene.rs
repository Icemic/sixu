use nom::bytes::complete::*;
use nom::combinator::*;
use nom::sequence::*;
use nom::IResult;

use super::comment::span0;
use super::comment::span1;
use super::identifier::identifier;
use super::parameter::parameters;
use super::Scene;

pub fn scene(input: &str) -> IResult<&str, Scene> {
    let (input, _) = tag("scene")(input)?;
    let (input, _) = span1(input)?;
    let (input, name) = identifier(input)?;
    let (input, parameters) = delimited(
        span0,
        opt(parameters),
        span0,
    )(input)?;
    let (input, _) = tag("{")(input)?;
    let (input, _) = span0(input)?;
    let (input, _) = tag("}")(input)?;
    Ok((
        input,
        Scene {
            name: name.to_string(),
            parameters: parameters.unwrap_or_default(),
            block: Default::default(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene() {
        assert_eq!(
            scene("scene a {}"),
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
            scene("scene a { }"),
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
            scene("scene a { } "),
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
            scene("scene a { } // comment"),
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
            scene("scene\n// comment\n a \n// comment\n { } // comment"),
            Ok((
                " // comment",
                Scene {
                    name: "a".to_string(),
                    parameters: vec![],
                    block: Default::default(),
                }
            ))
        );
    }
}
