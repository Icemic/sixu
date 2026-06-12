use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CommandSchema {
    pub commands: Vec<CommandDefinition>,
}

impl<'de> Deserialize<'de> for CommandSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = SchemaNode::deserialize(deserializer)?;
        let mut commands = Vec::new();
        raw.collect_command_definitions(&mut commands);
        Ok(CommandSchema { commands })
    }
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

#[derive(Debug, Deserialize)]
struct SchemaNode {
    pub description: Option<String>,
    pub properties: Option<HashMap<String, Property>>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "oneOf", default)]
    pub one_of: Vec<SchemaNode>,
    #[serde(rename = "anyOf", default)]
    pub any_of: Vec<SchemaNode>,
    #[serde(rename = "allOf", default)]
    pub all_of: Vec<SchemaNode>,
}

impl SchemaNode {
    fn collect_command_definitions(self, commands: &mut Vec<CommandDefinition>) {
        if let Some(properties) = self.properties
            && properties
                .get("command")
                .and_then(|property| property.const_value.as_ref())
                .is_some()
        {
            commands.push(CommandDefinition {
                description: self.description,
                properties,
                required: self.required,
            });
        }

        for child in self.one_of {
            child.collect_command_definitions(commands);
        }

        for child in self.any_of {
            child.collect_command_definitions(commands);
        }

        for child in self.all_of {
            child.collect_command_definitions(commands);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_one_of_command_schema() {
        let schema = serde_json::from_str::<CommandSchema>(include_str!(
            "../tests/fixtures/nested-oneof.json"
        ))
        .expect("nested oneOf schema should parse");

        let command_names: Vec<_> = schema
            .commands
            .iter()
            .filter_map(CommandDefinition::get_command_name)
            .collect();

        assert_eq!(
            command_names,
            vec![
                "transPrepare".to_string(),
                "transPerform".to_string(),
                "transPerform".to_string(),
                "transPerform".to_string(),
            ]
        );
    }
}
