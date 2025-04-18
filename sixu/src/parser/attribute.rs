use nom::bytes::complete::*;
use nom::character::complete::char;
use nom::error::{ErrorKind, ParseError};
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

    // Handle conditional case
    let (input, condition) =
        if let Ok((input, _)) = tag::<&str, &str, VerboseError<&str>>("(").parse(input) {
            // Use balanced delimiters function to parse parenthesized content
            let (input, condition) = balanced_delimiters('(', ')').parse(input)?;
            (input, Some(condition.to_string()))
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
        let input = "#[attribute_name(condition)]";
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
    fn test_attribute_complex() {
        let input = "#[attribute_name(a > b && (x + 1) < 10 && c.isValid() && foo='bar)')]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("a > b && (x + 1) < 10 && c.isValid() && foo='bar)'".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_with_quoted_strings() {
        let input = "#[attribute_name(a == 'string with (parens)' && b == \"another (string)\")]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("a == 'string with (parens)' && b == \"another (string)\"".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_with_template_literals() {
        let input = "#[attribute_name(a == `template ${var} with (nested ${obj.method()}) parts`)]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some(
                "a == `template ${var} with (nested ${obj.method()}) parts`".to_string(),
            ),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_attribute_with_regex() {
        let input = "#[attribute_name(text.match(/^\\(hello\\)$/i))]";
        let expected = Attribute {
            keyword: "attribute_name".to_string(),
            condition: Some("text.match(/^\\(hello\\)$/i)".to_string()),
        };
        let result = attribute(input).unwrap().1;
        assert_eq!(result, expected);
    }
}
