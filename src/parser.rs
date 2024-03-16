mod argument;
mod block;
mod command_line;
mod comment;
mod identifier;
mod parameter;
mod primitive;
mod rvalue;
mod scene;
mod systemcall_line;
mod variable;

use nom::combinator::all_consuming;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;

use crate::format::*;

use self::comment::span0;
use self::scene::scene;

/// parse a story file which is a sequence of scenes
pub fn parse(input: &str) -> IResult<&str, Story> {
    let (input, scenes) = all_consuming(many0(preceded(span0, scene)))(input)?;

    Ok((
        input,
        Story {
            filename: "unknown".to_string(),
            scenes,
        },
    ))
}
