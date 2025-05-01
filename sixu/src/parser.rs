mod argument;
mod attribute;
mod block;
mod command_line;
mod comment;
mod identifier;
mod paragraph;
mod parameter;
mod primitive;
mod rvalue;
mod systemcall_line;
mod template;
mod text;
mod variable;

use nom::combinator::all_consuming;
use nom::multi::*;
use nom::sequence::*;
use nom::Parser;

use crate::format::*;
use crate::result::ParseResult;

use self::comment::span0;
use self::paragraph::paragraph;

/// parse a story file which is a sequence of paragraphs
pub fn parse<'a>(name: &'a str, input: &'a str) -> ParseResult<&'a str, Story> {
    let (input, paragraphs) =
        all_consuming(terminated(many0(preceded(span0, paragraph)), span0)).parse(input)?;

    Ok((
        input,
        Story {
            filename: name.to_string(),
            paragraphs,
        },
    ))
}
