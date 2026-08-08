use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

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
    pub properties: Properties,
    pub required: Option<Vec<String>>,
}

impl CommandDefinition {
    pub fn get_command_name(&self) -> Option<String> {
        self.properties
            .get("command")
            .and_then(|p| p.const_value.clone())
    }

    pub fn collect_property_value_options(
        definitions: &[&CommandDefinition],
        property_name: &str,
        default: Option<&serde_json::Value>,
    ) -> Vec<String> {
        let mut value_options = Vec::new();
        for definition in definitions {
            if let Some(property) = definition.properties.get(property_name) {
                if let Some(enum_values) = &property.enum_values {
                    for value in enum_values {
                        if !value_options.contains(value) {
                            value_options.push(value.clone());
                        }
                    }
                }

                if let Some(value) = &property.const_value
                    && !value_options.contains(value)
                {
                    value_options.push(value.clone());
                }
            }
        }

        if let Some(default_value) = default.and_then(|value| value.as_str())
            && let Some(index) = value_options
                .iter()
                .position(|value| value == default_value)
        {
            let default_value = value_options.remove(index);
            value_options.insert(0, default_value);
        }

        value_options
    }
}

#[derive(Debug, Clone, Default)]
pub struct Properties(Vec<(String, Property)>);

impl Properties {
    pub fn get(&self, key: &str) -> Option<&Property> {
        self.0
            .iter()
            .find_map(|(property_key, property)| (property_key == key).then_some(property))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Property)> {
        self.0.iter().map(|(key, property)| (key, property))
    }
}

impl<'de> Deserialize<'de> for Properties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PropertiesVisitor;

        impl<'de> Visitor<'de> for PropertiesVisitor {
            type Value = Properties;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON object containing command properties")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut properties = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Property>()? {
                    properties.push((key, value));
                }
                Ok(Properties(properties))
            }
        }

        deserializer.deserialize_map(PropertiesVisitor)
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
    pub enum_values: Option<Vec<String>>,
    pub default: Option<serde_json::Value>,
    pub format: Option<String>,
}

impl Property {
    pub fn is_string(&self) -> bool {
        self.type_
            .as_ref()
            .map(|type_| match type_ {
                StringOrArray::String(value) => value == "string",
                StringOrArray::Array(values) => values.contains(&"string".to_string()),
            })
            .unwrap_or(false)
    }
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
    pub properties: Option<Properties>,
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
