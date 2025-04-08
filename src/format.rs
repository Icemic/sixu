/// The format represents the structure of a `story`, which is commonly came from a single file.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Story {
    pub filename: String,
    pub scenes: Vec<Scene>,
}

/// The format represents the structure of a `scene` inside a `story`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Scene {
    pub name: String,
    pub parameters: Vec<Parameter>,
    /// root block
    pub block: Block,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub default_value: Option<Primitive>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Argument {
    pub name: String,
    pub value: Option<RValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl ToString for Primitive {
    fn to_string(&self) -> String {
        match self {
            Primitive::String(s) => s.clone(),
            Primitive::Integer(i) => i.to_string(),
            Primitive::Float(f) => f.to_string(),
            Primitive::Boolean(b) => b.to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Variable {
    pub chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RValue {
    Primitive(Primitive),
    Variable(Variable),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Block {
    pub children: Vec<Child>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Child {
    pub attributes: Vec<Attribute>,
    pub content: ChildContent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChildContent {
    Block(Block),
    TextLine(String),
    CommandLine(CommandLine),
    SystemCallLine(SystemCallLine),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CommandLine {
    pub command: String,
    pub flags: Vec<String>,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SystemCallLine {
    pub command: String,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Attribute {
    pub _content: String,
}
