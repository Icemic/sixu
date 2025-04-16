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

pub fn attribute(input: &str) -> ParseResult<&str, Attribute> {
    let (input, _) = span0.parse(input)?;

    let (input, _) = tag("#[").parse(input)?;
    let (input, _) = span0_inline.parse(input)?;

    // 解析属性名
    let (input, keyword) = identifier.parse(input)?;
    let (input, _) = span0_inline.parse(input)?;

    // 处理有条件的情况
    let (input, condition) =
        if let Ok((input, _)) = tag::<&str, &str, VerboseError<&str>>("(").parse(input) {
            // 解析嵌套括号内的内容
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
                    // 如果前一个字符是转义符，忽略当前字符的特殊意义
                    escape_next = false;
                } else if ch == '\\' {
                    // 标记下一个字符被转义
                    escape_next = true;
                } else if !in_single_quote && !in_double_quote && !in_backtick {
                    // 只有不在引号内时，才处理括号计数
                    match ch {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        '\'' => in_single_quote = true,
                        '"' => in_double_quote = true,
                        '`' => in_backtick = true,
                        _ => {}
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

            // 提取条件内容 (减1是因为我们不包括匹配的右括号)
            let condition = &input[..end - 1];
            let input = &input[end..];

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
