use crate::error::Result;
use crate::format::*;

use super::RuntimeContext;

pub trait RuntimeExecutor: Send + Sync {
    fn handle_command(
        &mut self,
        ctx: &mut RuntimeContext,
        command_line: &CommandLine,
    ) -> Result<()>;
    fn handle_extra_system_call(
        &mut self,
        ctx: &mut RuntimeContext,
        systemcall_line: &SystemCallLine,
    ) -> Result<()>;
    fn handle_text(
        &mut self,
        ctx: &mut RuntimeContext,
        leading: Option<&str>,
        text: Option<&str>,
    ) -> Result<()>;
    fn eval_script(&mut self, ctx: &mut RuntimeContext, script: &String) -> Result<Option<RValue>>;
    fn finished(&mut self, ctx: &mut RuntimeContext);

    fn get_variable<'a>(
        &self,
        ctx: &'a RuntimeContext,
        value: &'a Variable,
    ) -> Result<&'a Literal> {
        if value.chain.len() == 1 {
            let name = &value.chain[0];
            let v = ctx
                .archive_variables()
                .as_object()?
                .get(name)
                .or_else(|| {
                    ctx.global_variables()
                        .as_object()
                        .map(|o| o.get(name))
                        .unwrap_or_else(|_| Some(&Literal::Null))
                })
                .unwrap_or(&Literal::Null);
            Ok(v)
        } else {
            log::warn!(
                "Variable chain with more than one element is not supported: {:?}",
                value.chain
            );
            Ok(&Literal::Null)
        }
    }

    fn calculate_template_literal<'a>(
        &self,
        ctx: &'a RuntimeContext,
        template: &'a crate::format::TemplateLiteral,
    ) -> Result<String> {
        let text = template
            .parts
            .iter()
            .map(|part| match part {
                crate::format::TemplateLiteralPart::Text(text) => text.to_owned(),
                crate::format::TemplateLiteralPart::Value(value) => {
                    match self.get_rvalue(ctx, value) {
                        Ok(v) => v.to_string(),
                        Err(err) => {
                            log::error!(
                                "Failed to get rvalue from template literal: {:?}.\
                                             Error: {:?}",
                                value,
                                err
                            );
                            "[Error]".to_string()
                        }
                    }
                }
            })
            .collect::<String>();
        Ok(text)
    }

    fn get_rvalue<'a>(&self, ctx: &'a RuntimeContext, value: &'a RValue) -> Result<&'a Literal> {
        match value {
            RValue::Literal(s) => Ok(s),
            RValue::Variable(v) => self.get_variable(ctx, v),
        }
    }
}
