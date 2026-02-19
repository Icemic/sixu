use nom::branch::alt;
use nom::bytes::complete::*;
use nom::character::complete::char;
use nom::error::{ErrorKind, ParseError};
use nom::sequence::delimited;
use nom::Err;
use nom::Parser;
use nom_language::error::VerboseError;

use crate::format::Attribute;
use crate::result::ParseResult;

use super::comment::{span0, span0_inline};
use super::identifier::identifier;

/// Parse content with balanced delimiters, handling nested delimiters and quoted content
///
/// # Parameters
/// * `open_delim` - The opening delimiter character
/// * `close_delim` - The closing delimiter character
///
/// # Returns
/// Returns a closure that takes an input string and returns a tuple `(remaining_input, content)`, where:
/// * `remaining_input` - The remaining input string after parsing
/// * `content` - The content between the two delimiters (excluding the delimiters)
pub fn balanced_delimiters<'a>(
    open_delim: char,
    close_delim: char,
) -> impl FnMut(&'a str) -> Result<(&'a str, &'a str), Err<VerboseError<&'a str>>> {
    move |input: &'a str| {
        let mut depth = 1;
        let mut end = 0;
        let chars: Vec<char> = input.chars().collect();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut in_backtick = false;
        let mut escape_next = false;

        while end < chars.len() && depth > 0 {
            let ch = chars[end];

            if escape_next {
                // If previous character was an escape, ignore special meaning of current character
                escape_next = false;
            } else if ch == '\\' {
                // Mark the next character as being escaped
                escape_next = true;
            } else if !in_single_quote && !in_double_quote && !in_backtick {
                // Only process delimiter counting when not inside quotes
                if ch == open_delim {
                    depth += 1;
                } else if ch == close_delim {
                    depth -= 1;
                } else if ch == '\'' {
                    in_single_quote = true;
                } else if ch == '"' {
                    in_double_quote = true;
                } else if ch == '`' {
                    in_backtick = true;
                }
            } else if ch == '\'' && in_single_quote {
                in_single_quote = false;
            } else if ch == '"' && in_double_quote {
                in_double_quote = false;
            } else if ch == '`' && in_backtick {
                in_backtick = false;
            }

            end += 1;
        }

        if depth != 0 {
            return Err(Err::Error(VerboseError::from_error_kind(
                input,
                ErrorKind::Tag,
            )));
        }

        // Extract content (minus 1 because we don't include the closing delimiter)
        let content = &input[..end - 1];
        let remaining = &input[end..];

        // Return remaining input and content
        Ok((remaining, content))
    }
}

pub fn attribute(input: &str) -> ParseResult<&str, Attribute> {
    let (input, _) = span0.parse(input)?;

    let (input, _) = tag("#[").parse(input)?;
    let (input, _) = span0_inline.parse(input)?;

    // Parse attribute name
    let (input, keyword) = identifier.parse(input)?;
    let (input, _) = span0_inline.parse(input)?;

    // Handle conditional case: condition must be a quoted string inside parentheses
    // e.g. #[cond("x > 10")] or #[cond('counter < 3')]
    let (input, condition) =
        if let Ok((input, _)) = tag::<&str, &str, VerboseError<&str>>("(").parse(input) {
            let (input, _) = span0_inline.parse(input)?;
            // Parse a quoted string (double or single quotes)
            let (input, condition_str) = alt((
                delimited(
                    tag::<&str, &str, VerboseError<&str>>("\""),
                    take_until("\""),
                    tag("\""),
                ),
                delimited(tag("'"), take_until("'"), tag("'")),
            ))
            .parse(input)
            .map_err(|_| Err::Error(VerboseError::from_error_kind(input, ErrorKind::Tag)))?;
            let (input, _) = span0_inline.parse(input)?;
            let (input, _) = tag(")").parse(input)?;
            (input, Some(condition_str.to_string()))
        } else {
            (input, None)
        };

    let (input, _) = span0_inline.parse(input)?;
    let (input, _) = char(']').parse(input)?;

    let attribute = Attribute {
        keyword: keyword.to_string(),
        condition,
    };

    Ok((input, attribute))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute() {
        let input = "#[attribute_name(\"condition\")]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("condition".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_without_condition() {
        let input = "#[attribute_name]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: None,
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_with_double_quotes() {
        let input = "#[attribute_name(\"a > b && (x + 1) < 10\")]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("a > b && (x + 1) < 10".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_with_single_quotes() {
        let input = "#[attribute_name('a > b && (x + 1) < 10')]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("a > b && (x + 1) < 10".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_condition_with_special_chars() {
        // Condition can contain any characters since it's just a string
        let input = "#[attribute_name(\"a == 'hello' && b > (c * d)\")]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("a == 'hello' && b > (c * d)".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_condition_with_spaces_in_parens() {
        // Spaces around the quoted string inside parentheses
        let input = "#[attribute_name( \"condition\" )]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("condition".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    // === Attribute keyword-specific tests ===

    #[test]
    fn test_attribute_cond() {
        let input = "#[cond(\"x > 10\")]";
        let expected = Attribute {
            keyword: "cond".to_string(),
            condition: Some("x > 10".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_if_alias() {
        // `if` is an alias for `cond`
        let input = "#[if(\"save.x = 1\")]";
        let expected = Attribute {
            keyword: "if".to_string(),
            condition: Some("save.x = 1".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_while() {
        let input = "#[while(\"counter < 10\")]";
        let expected = Attribute {
            keyword: "while".to_string(),
            condition: Some("counter < 10".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_loop_without_condition() {
        let input = "#[loop]";
        let expected = Attribute {
            keyword: "loop".to_string(),
            condition: None,
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_if_complex_condition() {
        let input = "#[if(\"a =123 && (b + 1) > '])'.length\")]";
        let expected = Attribute {
            keyword: "if".to_string(),
            condition: Some("a =123 && (b + 1) > '])'.length".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }
}
