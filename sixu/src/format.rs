#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
pub struct Parameter {
    pub name: String,
    pub default_value: Option<Literal>,
}

#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Argument {
    pub name: String,
    pub value: Option<RValue>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Literal {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
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

    pub fn as_string(&self) -> Option<&String> {
        if let Primitive::String(ref s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_integer(&self) -> Option<&i64> {
        if let Primitive::Integer(ref i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<&f64> {
        if let Primitive::Float(ref f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn as_boolean(&self) -> Option<&bool> {
        if let Primitive::Boolean(ref b) = self {
            Some(b)
        } else {
            None
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
