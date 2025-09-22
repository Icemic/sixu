use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::{Result, RuntimeError};

/// The format represents the structure of a `story`, which is commonly came from a single file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Story {
    pub name: String,
    pub paragraphs: Vec<Paragraph>,
}

/// The format represents the structure of a `paragraph` inside a `story`.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Paragraph {
    pub name: String,
    pub parameters: Vec<Parameter>,
    /// root block
    pub block: Block,
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Parameter {
    pub name: String,
    pub default_value: Option<Literal>,
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Argument {
    pub name: String,
    pub value: Option<RValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Literal {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<Literal>),
    Object(HashMap<String, Literal>),
}

impl Literal {
    pub fn is_null(&self) -> bool {
        matches!(self, Literal::Null)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Literal::String(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Literal::Integer(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Literal::Float(_))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Literal::Boolean(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Literal::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Literal::Object(_))
    }

    pub fn as_string(&self) -> Result<&String> {
        if let Literal::String(ref s) = self {
            Ok(s)
        } else {
            Err(RuntimeError::NotAString)
        }
    }

    pub fn as_integer(&self) -> Result<&i64> {
        if let Literal::Integer(ref i) = self {
            Ok(i)
        } else {
            Err(RuntimeError::NotAInteger)
        }
    }

    pub fn as_float(&self) -> Result<&f64> {
        if let Literal::Float(ref f) = self {
            Ok(f)
        } else {
            Err(RuntimeError::NotAFloat)
        }
    }

    pub fn as_number(&self) -> Result<f64> {
        match self {
            Literal::Integer(i) => Ok(*i as f64),
            Literal::Float(f) => Ok(*f),
            _ => Err(RuntimeError::NotANumber),
        }
    }

    pub fn as_boolean(&self) -> Result<&bool> {
        if let Literal::Boolean(ref b) = self {
            Ok(b)
        } else {
            Err(RuntimeError::NotABoolean)
        }
    }

    pub fn as_array(&self) -> Result<&Vec<Literal>> {
        if let Literal::Array(ref a) = self {
            Ok(a)
        } else {
            Err(RuntimeError::NotAArray)
        }
    }

    pub fn as_object(&self) -> Result<&HashMap<String, Literal>> {
        if let Literal::Object(ref o) = self {
            Ok(o)
        } else {
            Err(RuntimeError::NotAObject)
        }
    }
    pub fn as_string_mut(&mut self) -> Result<&mut String> {
        if let Literal::String(ref mut s) = self {
            Ok(s)
        } else {
            Err(RuntimeError::NotAString)
        }
    }

    pub fn as_integer_mut(&mut self) -> Result<&mut i64> {
        if let Literal::Integer(ref mut i) = self {
            Ok(i)
        } else {
            Err(RuntimeError::NotAInteger)
        }
    }

    pub fn as_float_mut(&mut self) -> Result<&mut f64> {
        if let Literal::Float(ref mut f) = self {
            Ok(f)
        } else {
            Err(RuntimeError::NotAFloat)
        }
    }

    pub fn as_boolean_mut(&mut self) -> Result<&mut bool> {
        if let Literal::Boolean(ref mut b) = self {
            Ok(b)
        } else {
            Err(RuntimeError::NotABoolean)
        }
    }

    pub fn as_array_mut(&mut self) -> Result<&mut Vec<Literal>> {
        if let Literal::Array(ref mut a) = self {
            Ok(a)
        } else {
            Err(RuntimeError::NotAArray)
        }
    }

    pub fn as_object_mut(&mut self) -> Result<&mut HashMap<String, Literal>> {
        if let Literal::Object(ref mut o) = self {
            Ok(o)
        } else {
            Err(RuntimeError::NotAObject)
        }
    }
}

impl ToString for Literal {
    fn to_string(&self) -> String {
        match self {
            Literal::Null => "null".to_string(),
            Literal::String(s) => s.clone(),
            Literal::Integer(i) => i.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::Boolean(b) => b.to_string(),
            Literal::Array(a) => {
                let elements: Vec<String> = a.iter().map(|e| e.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            Literal::Object(o) => {
                let entries: Vec<String> = o
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_string()))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
        }
    }
}

#[cfg(feature = "serde")]
impl From<Literal> for serde_json::Value {
    fn from(val: Literal) -> Self {
        match val {
            Literal::Null => serde_json::Value::Null,
            Literal::String(s) => serde_json::Value::String(s),
            Literal::Integer(i) => serde_json::Value::Number(serde_json::Number::from(i)),
            Literal::Float(f) => serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Literal::Boolean(b) => serde_json::Value::Bool(b),
            Literal::Array(a) => serde_json::Value::Array(a.into_iter().map(Into::into).collect()),
            Literal::Object(o) => {
                serde_json::Value::Object(o.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Value> for Literal {
    fn from(val: serde_json::Value) -> Self {
        match val {
            serde_json::Value::Null => Literal::Null,
            serde_json::Value::String(s) => Literal::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Literal::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Literal::Float(f)
                } else {
                    Literal::Null
                }
            }
            serde_json::Value::Bool(b) => Literal::Boolean(b),
            serde_json::Value::Array(a) => Literal::Array(a.into_iter().map(Into::into).collect()),
            serde_json::Value::Object(o) => {
                Literal::Object(o.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Variable {
    pub chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum RValue {
    Literal(Literal),
    Variable(Variable),
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Block {
    pub children: Vec<Child>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Child {
    pub attributes: Vec<Attribute>,
    pub content: ChildContent,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ChildContent {
    Block(Block),
    TextLine(LeadingText, Text),
    CommandLine(CommandLine),
    SystemCallLine(SystemCallLine),
    EmbeddedCode(String),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeadingText {
    None,
    Text(String),
    TemplateLiteral(TemplateLiteral),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Text {
    None,
    Text(String),
    TemplateLiteral(TemplateLiteral),
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TemplateLiteral {
    pub parts: Vec<TemplateLiteralPart>,
}

impl TemplateLiteral {
    pub fn get_strings(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                TemplateLiteralPart::Text(text) => Some(text.clone()),
                TemplateLiteralPart::Value(_) => None,
            })
            .collect()
    }
    pub fn get_values(&self) -> Vec<RValue> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                TemplateLiteralPart::Text(_) => None,
                TemplateLiteralPart::Value(value) => Some(value.clone()),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TemplateLiteralPart {
    Text(String),
    Value(RValue),
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CommandLine {
    pub command: String,
    pub flags: Vec<String>,
    pub arguments: Vec<Argument>,
}

impl CommandLine {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    pub fn get_argument(&self, name: &str) -> Option<&RValue> {
        self.arguments
            .iter()
            .find(|arg| arg.name == name)
            .and_then(|arg| arg.value.as_ref())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SystemCallLine {
    pub command: String,
    pub arguments: Vec<Argument>,
}

impl SystemCallLine {
    pub fn get_argument(&self, name: &str) -> Option<&RValue> {
        self.arguments
            .iter()
            .find(|arg| arg.name == name)
            .and_then(|arg| arg.value.as_ref())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Attribute {
    pub keyword: String,
    pub condition: Option<String>,
}
