use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct CommandSchema {
    #[serde(rename = "oneOf")]
    pub commands: Vec<CommandDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CommandDefinition {
    pub description: Option<String>,
    pub properties: HashMap<String, Property>,
    pub required: Option<Vec<String>>,
}

impl CommandDefinition {
    pub fn get_command_name(&self) -> Option<String> {
        self.properties
            .get("command")
            .and_then(|p| p.const_value.clone())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Property {
    #[serde(rename = "type")]
    pub type_: Option<StringOrArray>,
    pub description: Option<String>,
    #[serde(rename = "const")]
    pub const_value: Option<String>,
    #[serde(rename = "enum")]
    #[allow(dead_code)]
    pub enum_values: Option<Vec<String>>,
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum StringOrArray {
    String(String),
    Array(Vec<String>),
}
