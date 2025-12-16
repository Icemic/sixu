use std::future::Future;

use crate::error::Result;
use crate::format::*;

use super::RuntimeContext;

/// Trait defining the executor behavior for runtime execution
pub trait RuntimeExecutor: Send + Sync {
    /// Handle a command line input, returns true if next line should be executed immediately
    fn handle_command(
        &mut self,
        ctx: &mut RuntimeContext,
        command_line: &ResolvedCommandLine,
    ) -> Result<bool>;
    /// Handle an extra system call line input, returns true if next line should be executed immediately
    fn handle_extra_system_call(
        &mut self,
        ctx: &mut RuntimeContext,
        systemcall_line: &ResolvedSystemCallLine,
    ) -> Result<bool>;
    /// Handle text output, returns true if next line should be executed immediately
    fn handle_text(
        &mut self,
        ctx: &mut RuntimeContext,
        leading: Option<&str>,
        text: Option<&str>,
        tailing: Option<&str>,
    ) -> Result<bool>;
    /// Evaluate a script, returns the result and whether next line should be executed immediately
    fn eval_script(
        &mut self,
        ctx: &mut RuntimeContext,
        script: &String,
    ) -> Result<(Option<RValue>, bool)>;
    /// Called when the scenario execution is finished
    fn finished(&mut self, ctx: &mut RuntimeContext);

    fn read_story_file(
        &mut self,
        ctx: &mut RuntimeContext,
        story_name: &str,
    ) -> impl Future<Output = Result<Vec<u8>>>;

    /// Helper method to get variable value from context
    ///
    /// NOTE: This is a default implementation and should not be overridden in most cases
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

    /// Helper method to calculate template literal from context
    ///
    /// NOTE: This is a default implementation and should not be overridden in most cases
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

    /// Helper method to get RValue from context
    ///
    /// NOTE: This is a default implementation and should not be overridden in most cases
    fn get_rvalue<'a>(&self, ctx: &'a RuntimeContext, value: &'a RValue) -> Result<&'a Literal> {
        match value {
            RValue::Literal(s) => Ok(s),
            RValue::Variable(v) => self.get_variable(ctx, v),
        }
    }
}
