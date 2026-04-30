#[cfg(feature = "serde")]
use serde::de::{self, Visitor};
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use twox_hash::XxHash3_128;

use crate::format::{
    Argument, Attribute, Block, Child, ChildContent, CommandLine, LeadingText, Literal, RValue,
    SystemCallLine, TailingText, TemplateLiteral, TemplateLiteralPart, Text, Variable,
};

const VERSION_PREFIX: &str = "sixu:block-fingerprint:v1";

#[derive(Clone, Copy)]
#[repr(u8)]
enum Tag {
    Block = 0x01,
    Child = 0x02,
    Attribute = 0x03,
    Argument = 0x04,
    Variable = 0x05,
    TemplateLiteral = 0x06,

    OptionNone = 0x10,
    OptionSome = 0x11,

    ChildContentBlock = 0x20,
    ChildContentTextLine = 0x21,
    ChildContentCommandLine = 0x22,
    ChildContentSystemCallLine = 0x23,
    ChildContentEmbeddedCode = 0x24,

    LeadingTextNone = 0x30,
    LeadingTextText = 0x31,
    LeadingTextTemplateLiteral = 0x32,

    TextNone = 0x40,
    TextText = 0x41,
    TextTemplateLiteral = 0x42,

    TailingTextNone = 0x50,
    TailingTextText = 0x51,

    TemplateLiteralPartText = 0x60,
    TemplateLiteralPartValue = 0x61,

    RValueLiteral = 0x70,
    RValueVariable = 0x71,

    LiteralNull = 0x80,
    LiteralString = 0x81,
    LiteralInteger = 0x82,
    LiteralFloat = 0x83,
    LiteralBoolean = 0x84,
    LiteralArray = 0x85,
    LiteralObject = 0x86,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockFingerprint([u8; 16]);

impl BlockFingerprint {
    pub const VERSION: &'static str = VERSION_PREFIX;

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(self.0.len() * 2);

        for byte in self.0 {
            hex.push(hex_digit(byte >> 4));
            hex.push(hex_digit(byte & 0x0f));
        }

        hex
    }

    fn from_hex(value: &str) -> Result<Self, &'static str> {
        if value.len() != 32 {
            return Err("block fingerprint hex must be 32 characters long");
        }

        let bytes = value.as_bytes();
        let mut output = [0_u8; 16];

        for (index, chunk) in bytes.chunks_exact(2).enumerate() {
            let high = decode_hex_digit(chunk[0])?;
            let low = decode_hex_digit(chunk[1])?;
            output[index] = (high << 4) | low;
        }

        Ok(Self(output))
    }
}

#[cfg(feature = "serde")]
impl Serialize for BlockFingerprint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for BlockFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BlockFingerprintVisitor;

        impl<'de> Visitor<'de> for BlockFingerprintVisitor {
            type Value = BlockFingerprint;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a 32-character lowercase hexadecimal fingerprint")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                BlockFingerprint::from_hex(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(BlockFingerprintVisitor)
    }
}

impl Block {
    pub fn fingerprint(&self) -> BlockFingerprint {
        let mut writer = FingerprintWriter::new();
        writer.write_bytes(BlockFingerprint::VERSION.as_bytes());
        self.encode(&mut writer);
        writer.finish()
    }
}

struct FingerprintWriter {
    hasher: XxHash3_128,
}

impl FingerprintWriter {
    fn new() -> Self {
        Self {
            hasher: XxHash3_128::new(),
        }
    }

    fn finish(self) -> BlockFingerprint {
        BlockFingerprint(self.hasher.finish_128().to_be_bytes())
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.hasher.write(bytes);
    }

    fn write_tag(&mut self, tag: Tag) {
        self.write_u8(tag as u8);
    }

    fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    fn write_len(&mut self, value: usize) {
        self.write_u32(u32::try_from(value).expect("fingerprint length exceeds u32"));
    }

    fn write_str(&mut self, value: &str) {
        self.write_len(value.len());
        self.write_bytes(value.as_bytes());
    }

    fn write_optional_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.write_tag(Tag::OptionSome);
                self.write_str(value);
            }
            None => self.write_tag(Tag::OptionNone),
        }
    }

    fn write_f64(&mut self, value: f64) {
        self.write_bytes(&normalize_f64_bits(value).to_le_bytes());
    }
}

trait FingerprintEncode {
    fn encode(&self, writer: &mut FingerprintWriter);
}

impl FingerprintEncode for Block {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_tag(Tag::Block);
        writer.write_len(self.children.len());

        for child in &self.children {
            child.encode(writer);
        }
    }
}

impl FingerprintEncode for Child {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_tag(Tag::Child);

        let mut attributes = self.attributes.iter().collect::<Vec<_>>();
        attributes.sort_by(|left, right| {
            left.keyword
                .cmp(&right.keyword)
                .then_with(|| left.condition.cmp(&right.condition))
        });

        writer.write_len(attributes.len());
        for attribute in attributes {
            attribute.encode(writer);
        }

        self.content.encode(writer);
    }
}

impl FingerprintEncode for Attribute {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_tag(Tag::Attribute);
        writer.write_str(&self.keyword);
        writer.write_optional_str(self.condition.as_deref());
    }
}

impl FingerprintEncode for ChildContent {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::Block(block) => {
                writer.write_tag(Tag::ChildContentBlock);
                block.encode(writer);
            }
            Self::TextLine(leading, text, tailing) => {
                writer.write_tag(Tag::ChildContentTextLine);
                leading.encode(writer);
                text.encode(writer);
                tailing.encode(writer);
            }
            Self::CommandLine(command_line) => {
                writer.write_tag(Tag::ChildContentCommandLine);
                command_line.encode(writer);
            }
            Self::SystemCallLine(system_call_line) => {
                writer.write_tag(Tag::ChildContentSystemCallLine);
                system_call_line.encode(writer);
            }
            Self::EmbeddedCode(code) => {
                writer.write_tag(Tag::ChildContentEmbeddedCode);
                writer.write_str(&normalize_embedded_code(code));
            }
        }
    }
}

impl FingerprintEncode for LeadingText {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::None => writer.write_tag(Tag::LeadingTextNone),
            Self::Text(text) => {
                writer.write_tag(Tag::LeadingTextText);
                writer.write_str(text);
            }
            Self::TemplateLiteral(template) => {
                writer.write_tag(Tag::LeadingTextTemplateLiteral);
                template.encode(writer);
            }
        }
    }
}

impl FingerprintEncode for Text {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::None => writer.write_tag(Tag::TextNone),
            Self::Text(text) => {
                writer.write_tag(Tag::TextText);
                writer.write_str(text);
            }
            Self::TemplateLiteral(template) => {
                writer.write_tag(Tag::TextTemplateLiteral);
                template.encode(writer);
            }
        }
    }
}

impl FingerprintEncode for TailingText {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::None => writer.write_tag(Tag::TailingTextNone),
            Self::Text(text) => {
                writer.write_tag(Tag::TailingTextText);
                writer.write_str(text);
            }
        }
    }
}

impl FingerprintEncode for TemplateLiteral {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_tag(Tag::TemplateLiteral);
        writer.write_len(self.parts.len());

        for part in &self.parts {
            part.encode(writer);
        }
    }
}

impl FingerprintEncode for TemplateLiteralPart {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::Text(text) => {
                writer.write_tag(Tag::TemplateLiteralPartText);
                writer.write_str(text);
            }
            Self::Value(value) => {
                writer.write_tag(Tag::TemplateLiteralPartValue);
                value.encode(writer);
            }
        }
    }
}

impl FingerprintEncode for CommandLine {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_str(&self.command);

        let mut arguments = self.arguments.iter().collect::<Vec<_>>();
        arguments.sort_by(|left, right| left.name.cmp(&right.name));

        writer.write_len(arguments.len());
        for argument in arguments {
            argument.encode(writer);
        }
    }
}

impl FingerprintEncode for SystemCallLine {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_str(&self.command);

        let mut arguments = self.arguments.iter().collect::<Vec<_>>();
        arguments.sort_by(|left, right| left.name.cmp(&right.name));

        writer.write_len(arguments.len());
        for argument in arguments {
            argument.encode(writer);
        }
    }
}

impl FingerprintEncode for Argument {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_tag(Tag::Argument);
        writer.write_str(&self.name);
        self.value.encode(writer);
    }
}

impl FingerprintEncode for RValue {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::Literal(literal) => {
                writer.write_tag(Tag::RValueLiteral);
                literal.encode(writer);
            }
            Self::Variable(variable) => {
                writer.write_tag(Tag::RValueVariable);
                variable.encode(writer);
            }
        }
    }
}

impl FingerprintEncode for Variable {
    fn encode(&self, writer: &mut FingerprintWriter) {
        writer.write_tag(Tag::Variable);
        writer.write_len(self.chain.len());

        for segment in &self.chain {
            writer.write_str(segment);
        }
    }
}

impl FingerprintEncode for Literal {
    fn encode(&self, writer: &mut FingerprintWriter) {
        match self {
            Self::Null => writer.write_tag(Tag::LiteralNull),
            Self::String(value) => {
                writer.write_tag(Tag::LiteralString);
                writer.write_str(value);
            }
            Self::Integer(value) => {
                writer.write_tag(Tag::LiteralInteger);
                writer.write_i64(*value);
            }
            Self::Float(value) => {
                writer.write_tag(Tag::LiteralFloat);
                writer.write_f64(*value);
            }
            Self::Boolean(value) => {
                writer.write_tag(Tag::LiteralBoolean);
                writer.write_bool(*value);
            }
            Self::Array(values) => {
                writer.write_tag(Tag::LiteralArray);
                writer.write_len(values.len());
                for value in values {
                    value.encode(writer);
                }
            }
            Self::Object(entries) => {
                writer.write_tag(Tag::LiteralObject);

                let mut entries = entries.iter().collect::<Vec<_>>();
                entries.sort_by(|left, right| left.0.cmp(right.0));

                writer.write_len(entries.len());
                for (key, value) in entries {
                    writer.write_str(key);
                    value.encode(writer);
                }
            }
        }
    }
}

fn normalize_embedded_code(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n").trim().to_string()
}

fn normalize_f64_bits(value: f64) -> u64 {
    if value == 0.0 {
        0.0f64.to_bits()
    } else if value.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        value.to_bits()
    }
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => unreachable!("hex digit out of range"),
    }
}

fn decode_hex_digit(value: u8) -> Result<u8, &'static str> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("block fingerprint contains a non-hex character"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::format::{CommandLine, RValue};

    fn text_child(value: &str) -> Child {
        Child {
            marker: None,
            attributes: Vec::new(),
            content: ChildContent::TextLine(
                LeadingText::None,
                Text::Text(value.to_string()),
                TailingText::None,
            ),
        }
    }

    fn command_child(command: &str, arguments: Vec<(&str, RValue)>) -> Child {
        Child {
            marker: None,
            attributes: Vec::new(),
            content: ChildContent::CommandLine(CommandLine {
                command: command.to_string(),
                arguments: arguments
                    .into_iter()
                    .map(|(name, value)| Argument {
                        name: name.to_string(),
                        value,
                    })
                    .collect(),
            }),
        }
    }

    #[test]
    fn fingerprint_changes_when_children_order_changes() {
        let first = Block {
            children: vec![text_child("first"), text_child("second")],
        };
        let second = Block {
            children: vec![text_child("second"), text_child("first")],
        };

        assert_ne!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn fingerprint_ignores_attribute_order() {
        let first = Block {
            children: vec![Child {
                marker: None,
                attributes: vec![
                    Attribute {
                        keyword: "if".to_string(),
                        condition: Some("a".to_string()),
                    },
                    Attribute {
                        keyword: "while".to_string(),
                        condition: Some("b".to_string()),
                    },
                ],
                content: ChildContent::Block(Block { children: vec![] }),
            }],
        };
        let second = Block {
            children: vec![Child {
                marker: None,
                attributes: vec![
                    Attribute {
                        keyword: "while".to_string(),
                        condition: Some("b".to_string()),
                    },
                    Attribute {
                        keyword: "if".to_string(),
                        condition: Some("a".to_string()),
                    },
                ],
                content: ChildContent::Block(Block { children: vec![] }),
            }],
        };

        assert_eq!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn fingerprint_ignores_argument_order() {
        let first = Block {
            children: vec![command_child(
                "say",
                vec![
                    ("speaker", RValue::Literal(Literal::String("alice".to_string()))),
                    ("line", RValue::Literal(Literal::String("hello".to_string()))),
                ],
            )],
        };
        let second = Block {
            children: vec![command_child(
                "say",
                vec![
                    ("line", RValue::Literal(Literal::String("hello".to_string()))),
                    ("speaker", RValue::Literal(Literal::String("alice".to_string()))),
                ],
            )],
        };

        assert_eq!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn fingerprint_ignores_object_insertion_order() {
        let first = Block {
            children: vec![command_child(
                "config",
                vec![(
                    "value",
                    RValue::Literal(Literal::Object(HashMap::from([
                        ("foo".to_string(), Literal::Integer(1)),
                        ("bar".to_string(), Literal::Integer(2)),
                    ]))),
                )],
            )],
        };
        let second = Block {
            children: vec![command_child(
                "config",
                vec![(
                    "value",
                    RValue::Literal(Literal::Object(HashMap::from([
                        ("bar".to_string(), Literal::Integer(2)),
                        ("foo".to_string(), Literal::Integer(1)),
                    ]))),
                )],
            )],
        };

        assert_eq!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn fingerprint_normalizes_embedded_code_text() {
        let first = Block {
            children: vec![Child {
                marker: None,
                attributes: Vec::new(),
                content: ChildContent::EmbeddedCode("\r\n  let a = 1;\r\n".to_string()),
            }],
        };
        let second = Block {
            children: vec![Child {
                marker: None,
                attributes: Vec::new(),
                content: ChildContent::EmbeddedCode("let a = 1;\n".to_string()),
            }],
        };

        assert_eq!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn fingerprint_normalizes_negative_zero_and_nan() {
        let zero = Block {
            children: vec![command_child(
                "set",
                vec![("value", RValue::Literal(Literal::Float(0.0)))],
            )],
        };
        let negative_zero = Block {
            children: vec![command_child(
                "set",
                vec![("value", RValue::Literal(Literal::Float(-0.0)))],
            )],
        };
        let nan_a = Block {
            children: vec![command_child(
                "set",
                vec![(
                    "value",
                    RValue::Literal(Literal::Float(f64::from_bits(0x7ff8_0000_0000_0001))),
                )],
            )],
        };
        let nan_b = Block {
            children: vec![command_child(
                "set",
                vec![(
                    "value",
                    RValue::Literal(Literal::Float(f64::from_bits(0x7ff8_0000_0000_0010))),
                )],
            )],
        };

        assert_eq!(zero.fingerprint(), negative_zero.fingerprint());
        assert_eq!(nan_a.fingerprint(), nan_b.fingerprint());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn fingerprint_serde_round_trip_uses_lowercase_hex() {
        let fingerprint = Block {
            children: vec![text_child("hello")],
        }
        .fingerprint();

        let serialized = serde_json::to_string(&fingerprint).unwrap();
        let deserialized: BlockFingerprint = serde_json::from_str(&serialized).unwrap();

        assert_eq!(serialized, format!("\"{}\"", fingerprint.to_hex()));
        assert_eq!(deserialized, fingerprint);
        assert_eq!(fingerprint.to_hex(), fingerprint.to_hex().to_lowercase());
    }
}