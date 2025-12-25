use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{alpha1, alphanumeric1, char, space0, space1},
    combinator::{opt, recognize, value},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, pair, preceded, tuple},
    IResult, Parser,
};
use nom_language::error::VerboseError;
use ropey::Rope;
use tower_lsp_server::ls_types::{Position, Range};

#[derive(Debug)]
pub struct ScannedCommand {
    pub name: String,
    pub name_range: Range,
    pub args: Vec<ScannedArgument>,
    pub range: Range,
}

#[derive(Debug)]
pub struct ScannedSystemCall {
    pub name: String,
    pub name_range: Range,
    pub args: Vec<ScannedArgument>,
    pub range: Range,
}

#[derive(Debug)]
pub struct ScannedParagraph {
    pub name: String,
    pub name_range: Range,
    pub range: Range,
}

#[derive(Debug)]
pub struct ScannedArgument {
    pub name: String,
    pub name_range: Range,
    pub value_kind: ArgValueKind,
    pub value_range: Option<Range>, // Added to track value location
    pub value: Option<String>, // Added to track value content
}

#[derive(Debug, PartialEq, Clone)]
pub enum ArgValueKind {
    String,
    Number,
    Boolean,
    Variable,
    Other,
}

type Res<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

pub fn scan_commands(text: &str, rope: &Rope) -> Vec<ScannedCommand> {
    let mut commands = Vec::new();
    let mut input = text;

    while !input.is_empty() {
        // Try to parse a command
        if let Ok((rest, cmd)) = parse_command_with_span(input, text, rope) {
            commands.push(cmd);
            input = rest;
            continue;
        }

        // Try to skip comments
        if let Ok((rest, _)) = parse_comment(input) {
            input = rest;
            continue;
        }

        // Try to skip strings (to avoid parsing @ inside strings)
        if let Ok((rest, _)) = parse_string(input) {
            input = rest;
            continue;
        }

        // Skip one char
        let mut chars = input.chars();
        chars.next();
        input = chars.as_str();
    }

    commands
}

pub fn scan_system_calls(text: &str, rope: &Rope) -> Vec<ScannedSystemCall> {
    let mut system_calls = Vec::new();
    let mut input = text;

    while !input.is_empty() {
        if let Ok((rest, cmd)) = parse_system_call_with_span(input, text, rope) {
            system_calls.push(cmd);
            input = rest;
            continue;
        }
        if let Ok((rest, _)) = parse_comment(input) {
            input = rest;
            continue;
        }
        if let Ok((rest, _)) = parse_string(input) {
            input = rest;
            continue;
        }
        let mut chars = input.chars();
        chars.next();
        input = chars.as_str();
    }
    system_calls
}

pub fn scan_paragraphs(text: &str, rope: &Rope) -> Vec<ScannedParagraph> {
    let mut paragraphs = Vec::new();
    let mut input = text;

    while !input.is_empty() {
        if let Ok((rest, p)) = parse_paragraph_with_span(input, text, rope) {
            paragraphs.push(p);
            input = rest;
            continue;
        }
        if let Ok((rest, _)) = parse_comment(input) {
            input = rest;
            continue;
        }
        if let Ok((rest, _)) = parse_string(input) {
            input = rest;
            continue;
        }
        let mut chars = input.chars();
        chars.next();
        input = chars.as_str();
    }
    paragraphs
}


fn parse_comment(input: &str) -> Res<()> {
    alt((
        value((), pair(tag("//"), take_until("\n"))),
        value((), tuple((tag("/*"), take_until("*/"), tag("*/")))),
    ))
    .parse(input)
}

fn parse_string(input: &str) -> Res<()> {
    alt((
        value((), delimited(char('"'), take_until("\""), char('"'))),
        value((), delimited(char('\''), take_until("'"), char('\''))),
        value((), delimited(char('`'), take_until("`"), char('`'))),
    ))
    .parse(input)
}

fn parse_command_with_span<'a>(
    input: &'a str,
    full_text: &'a str,
    rope: &Rope,
) -> Res<'a, ScannedCommand> {
    let (input, _) = tag("@").parse(input)?;
    let start_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize - 1; // -1 for @

    let (input, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;

    let name_end_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let name_range = range_from_offsets(start_ptr + 1, name_end_ptr, rope); // +1 to skip @

    let (input, args) = alt((
        // Type A: ( ... )
        preceded(
            space0,
            delimited(
                char('('),
                delimited(
                    space0,
                    separated_list0(delimited(space0, char(','), space0), |i| {
                        parse_argument(i, full_text, rope)
                    }),
                    space0,
                ),
                char(')'),
            ),
        ),
        // Type B: space separated
        many0(preceded(space1, |i| parse_argument(i, full_text, rope))),
    ))
    .parse(input)?;

    let end_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let range = range_from_offsets(start_ptr, end_ptr, rope);

    Ok((input, ScannedCommand {
        name: name.to_string(),
        name_range,
        args,
        range,
    }))
}

fn parse_system_call_with_span<'a>(
    input: &'a str,
    full_text: &'a str,
    rope: &Rope,
) -> Res<'a, ScannedSystemCall> {
    let (input, _) = tag("#").parse(input)?;
    let start_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize - 1; // -1 for #

    let (input, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;

    let name_end_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let name_range = range_from_offsets(start_ptr + 1, name_end_ptr, rope); // +1 to skip #

    let (input, args) = alt((
        // Type A: ( ... )
        preceded(
            space0,
            delimited(
                char('('),
                delimited(
                    space0,
                    separated_list0(
                        delimited(space0, char(','), space0),
                        |i| parse_argument(i, full_text, rope)
                    ),
                    space0
                ),
                char(')')
            )
        ),
        // Type B: space separated
        many0(preceded(space1, |i| parse_argument(i, full_text, rope)))
    )).parse(input)?;

    let end_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let range = range_from_offsets(start_ptr, end_ptr, rope);

    Ok((input, ScannedSystemCall {
        name: name.to_string(),
        name_range,
        args,
        range,
    }))
}

fn parse_paragraph_with_span<'a>(
    input: &'a str,
    full_text: &'a str,
    rope: &Rope,
) -> Res<'a, ScannedParagraph> {
    let (input, _) = tag("::").parse(input)?;
    let start_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize - 2; // -2 for ::

    let (input, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;

    let name_end_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let name_range = range_from_offsets(start_ptr + 2, name_end_ptr, rope); // +2 to skip ::

    let end_ptr = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let range = range_from_offsets(start_ptr, end_ptr, rope);

    Ok((input, ScannedParagraph {
        name: name.to_string(),
        name_range,
        range,
    }))
}

fn parse_argument<'a>(input: &'a str, full_text: &'a str, rope: &Rope) -> Res<'a, ScannedArgument> {
    let (input, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)?;

    let name_start = name.as_ptr() as usize - full_text.as_ptr() as usize;
    let name_end = input.as_ptr() as usize - full_text.as_ptr() as usize;
    let name_range = range_from_offsets(name_start, name_end, rope);

    let (input, value_info) = opt(preceded(
        tuple((space0, tag("="), space0)),
        |i: &'a str| {
            let start_ptr = i.as_ptr() as usize - full_text.as_ptr() as usize;
            let (rest, kind) = parse_value(i)?;
            let end_ptr = rest.as_ptr() as usize - full_text.as_ptr() as usize;
            let range = range_from_offsets(start_ptr, end_ptr, rope);
            let value_str = full_text[start_ptr..end_ptr].to_string();
            Ok((rest, (kind, range, value_str)))
        }
    )).parse(input)?;

    let (value_kind, value_range, value) = match value_info {
        Some((k, r, v)) => (k, Some(r), Some(v)),
        None => (ArgValueKind::Boolean, None, None),
    };

    Ok((
        input,
        ScannedArgument {
            name: name.to_string(),
            name_range,
            value_kind,
            value_range,
            value,
        },
    ))
}

fn parse_value(input: &str) -> Res<ArgValueKind> {
    alt((
        value(ArgValueKind::String, parse_string),
        value(
            ArgValueKind::Number,
            recognize(pair(
                opt(tag("-")),
                take_while1(|c: char| c.is_digit(10) || c == '.'),
            )),
        ),
        value(ArgValueKind::Boolean, alt((tag("true"), tag("false")))),
        value(
            ArgValueKind::Variable,
            recognize(many1(alt((alphanumeric1, tag("."), tag("_"))))),
        ),
    ))
    .parse(input)
}

fn range_from_offsets(start: usize, end: usize, rope: &Rope) -> Range {
    let (start_line, start_col) = offset_to_position(start, rope);
    let (end_line, end_col) = offset_to_position(end, rope);
    Range {
        start: Position {
            line: start_line as u32,
            character: start_col as u32,
        },
        end: Position {
            line: end_line as u32,
            character: end_col as u32,
        },
    }
}

fn offset_to_position(offset: usize, rope: &Rope) -> (usize, usize) {
    let line = rope.byte_to_line(offset);
    let first_char_of_line = rope.line_to_char(line);
    let offset_char = rope.byte_to_char(offset);
    let col = offset_char - first_char_of_line;
    (line, col)
}
