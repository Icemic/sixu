use crate::error::Result;
use crate::format::*;

pub trait Executor: Send + Sync {
    fn handle_command(&mut self, command_line: &CommandLine) -> Result<()>;
    fn handle_system_call(&mut self, systemcall_line: &SystemCallLine) -> Result<()>;
    fn handle_text(&mut self, leading: Option<&str>, text: Option<&str>) -> Result<()>;
    fn get_variable<'a>(&self, value: &'a Variable) -> Result<&'a Primitive>;
    fn eval_script(&mut self, script: &String) -> Result<()>;
    fn finished(&mut self);

    fn calculate_template_literal<'a>(
        &self,
        template: &'a crate::format::TemplateLiteral,
    ) -> Result<String> {
        let text = template
            .parts
            .iter()
            .map(|part| match part {
                crate::format::TemplateLiteralPart::Text(text) => text.to_owned(),
                crate::format::TemplateLiteralPart::Value(value) => match self.get_rvalue(&value) {
                    Ok(v) => v.to_string(),
                    Err(err) => {
                        log::error!(
                            "Failed to get rvalue from template literal: {:?}.\
                                             Error: {:?}",
                            value,
                            err
                        );
                        return "[Error]".to_string();
                    }
                },
            })
            .collect::<String>();
        Ok(text)
    }

    fn get_rvalue<'a>(&self, value: &'a RValue) -> Result<&'a Primitive> {
        match value {
            RValue::Primitive(s) => Ok(s),
            RValue::Variable(v) => self.get_variable(v),
        }
    }
}
