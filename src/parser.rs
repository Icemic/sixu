mod comment;
mod identifier;
mod parameter;
mod primitive;
mod scene;

use nom::multi::*;
use nom::sequence::*;
use nom::IResult;

use crate::format::*;

use self::comment::span0;
use self::scene::scene;

pub fn parse(input: &str) -> IResult<&str, Story> {
    let (input, scenes) = many0(delimited(
        span0,
        scene,
        span0,
    ))(input)?;

    Ok((
        input,
        Story {
            filename: "unknown".to_string(),
            scenes,
        },
    ))
}
