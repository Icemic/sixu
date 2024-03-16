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
    pub attributes: Vec<Attribute>,
    pub children: Vec<Child>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Child {
    Block(Block),
    ScriptBlock(ScriptBlock),
    CommandLine(CommandLine),
    SystemCallLine(SystemCallLine),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ScriptBlock {
    pub attributes: Vec<Attribute>,
    pub lines: Vec<ScriptLine>,
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
pub struct ScriptLine {
    pub _content: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Attribute {
    pub _content: String,
}
