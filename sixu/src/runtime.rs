mod callback;
mod datasource;
mod executor;
mod state;

pub use self::callback::*;
pub use self::datasource::RuntimeContext;
pub use self::executor::RuntimeExecutor;
pub use self::state::ExecutionState;

use crate::error::{Result, RuntimeError};
use crate::format::*;

/// Runtime manages the execution context and executor together
pub struct Runtime<E: RuntimeExecutor> {
    context: RuntimeContext,
    executor: E,
}

impl<E: RuntimeExecutor> Runtime<E> {
    pub fn new(executor: E) -> Self {
        Self {
            context: RuntimeContext::new(),
            executor,
        }
    }

    pub fn new_with_context(executor: E, context: RuntimeContext) -> Self {
        Self { context, executor }
    }

    pub fn context(&self) -> &RuntimeContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut RuntimeContext {
        &mut self.context
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    pub fn add_story(&mut self, story: Story) {
        self.context.stories_mut().push(story);
    }

    pub fn get_story(&self, name: &str) -> Result<&Story> {
        self.context
            .stories()
            .iter()
            .find(|s| s.name == name)
            .ok_or(RuntimeError::StoryNotFound(name.to_string()))
    }

    pub fn get_paragraph(&self, story_name: &str, name: &str) -> Result<&Paragraph> {
        let story = self.get_story(story_name)?;
        story
            .paragraphs
            .iter()
            .find(|s| s.name == name)
            .ok_or(RuntimeError::ParagraphNotFound(name.to_string()))
    }

    pub fn save(&self) -> Result<Vec<ExecutionState>> {
        let stack = self.context.stack().clone();
        Ok(stack)
    }

    pub fn restore(&mut self, states: Vec<ExecutionState>) -> Result<()> {
        *self.context.stack_mut() = states;
        Ok(())
    }

    pub fn start(&mut self, story_name: &str) -> Result<()> {
        if self.context.stories().is_empty() {
            return Err(RuntimeError::NoStory);
        }

        let is_empty = self.context.stack().is_empty();
        if is_empty {
            let paragraph = self.get_paragraph(story_name, "entry")?;
            let block = paragraph.block.clone();
            self.context.stack_mut().push(ExecutionState::new(
                story_name.to_string(),
                "entry".to_string(),
                block,
            ));
        } else {
            return Err(RuntimeError::StoryStarted);
        }

        Ok(())
    }

    pub fn terminate(&mut self) -> Result<()> {
        if self.context.stack().is_empty() {
            return Err(RuntimeError::StoryNotStarted);
        }

        self.context.stack_mut().clear();
        self.context
            .archive_variables_mut()
            .as_object_mut()?
            .clear();
        self.executor.finished(&mut self.context);

        Ok(())
    }

    pub fn get_current_state(&self) -> Result<&ExecutionState> {
        self.context
            .stack()
            .last()
            .ok_or(RuntimeError::StoryNotStarted)
    }

    pub fn get_current_state_mut(&mut self) -> Result<&mut ExecutionState> {
        self.context
            .stack_mut()
            .last_mut()
            .ok_or(RuntimeError::StoryNotStarted)
    }

    pub fn break_current_block(&mut self) -> Result<()> {
        if let Some(state) = self.context.stack_mut().pop() {
            // if the stack is empty, try to load the next paragraph of the current story
            if self.context.stack().is_empty() {
                if let Some(next_paragraph) = {
                    let story = self.get_story(&state.story)?;
                    let mut paragraph_iter = story.paragraphs.iter();
                    paragraph_iter.position(|s| s.name == state.paragraph);

                    paragraph_iter.next().cloned()
                } {
                    self.context.stack_mut().push(ExecutionState::new(
                        state.story.clone(),
                        next_paragraph.name,
                        next_paragraph.block,
                    ));
                } else {
                    self.executor.finished(&mut self.context);
                }
            }

            Ok(())
        } else {
            // Use this error to tell the user that the story is finished, who should
            // break the loop or stop the execution
            Err(RuntimeError::StoryFinished)
        }
    }

    pub fn next(&mut self) -> Result<()> {
        let current_state = self.get_current_state_mut()?;

        if let Some(child) = current_state.next_line() {
            let content = child.content;
            match content {
                ChildContent::Block(block) => {
                    let current_state = self.get_current_state()?.clone();
                    self.context.stack_mut().push(ExecutionState::new(
                        current_state.story,
                        current_state.paragraph,
                        block.clone(),
                    ));
                }
                ChildContent::TextLine(leading, text) => {
                    let leading = match leading {
                        LeadingText::None => None,
                        LeadingText::Text(t) => Some(t),
                        LeadingText::TemplateLiteral(template_literal) => {
                            let text = self
                                .executor
                                .calculate_template_literal(&self.context, &template_literal)?;
                            Some(text)
                        }
                    };
                    let text = match text {
                        Text::None => None,
                        Text::Text(t) => Some(t),
                        Text::TemplateLiteral(template_literal) => {
                            let text = self
                                .executor
                                .calculate_template_literal(&self.context, &template_literal)?;
                            Some(text)
                        }
                    };
                    self.executor.handle_text(
                        &mut self.context,
                        leading.as_deref(),
                        text.as_deref(),
                    )?;
                }
                ChildContent::CommandLine(command) => {
                    self.executor.handle_command(&mut self.context, &command)?;
                }
                ChildContent::SystemCallLine(systemcall) => {
                    self.handle_system_call(&systemcall)?;
                }
                ChildContent::EmbeddedCode(script) => {
                    self.executor.eval_script(&mut self.context, &script)?;
                }
            }
        } else {
            self.break_current_block()?;
        }

        Ok(())
    }

    fn handle_system_call(&mut self, systemcall_line: &SystemCallLine) -> Result<()> {
        match systemcall_line.command.as_str() {
            // This method will clear the stack and push a new state with the story and paragraph name
            "goto" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
                        let v = self.executor.get_rvalue(&self.context, v)?;
                        if v.is_string() {
                            v.to_string()
                        } else {
                            return Err(RuntimeError::WrongArgumentSystemCallLine(
                                "Expected a string argument".to_string(),
                            ));
                        }
                    }
                    None => self.get_current_state().unwrap().story.clone(),
                };

                if let Some(paragraph_name) = systemcall_line.get_argument("paragraph") {
                    let paragraph_name = self
                        .executor
                        .get_rvalue(&self.context, paragraph_name)?
                        .to_owned();
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    self.context.stack_mut().clear();

                    let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();

                    self.context.stack_mut().push(ExecutionState::new(
                        story_name,
                        paragraph_name.to_string(),
                        paragraph.block,
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Paragraph name not provided".to_string(),
                    ));
                }
            }
            // This method will replace the current state with a new state with the story and paragraph name
            // once this new state is ended, it will return to the previous state
            "replace" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
                        let v = self.executor.get_rvalue(&self.context, v)?;
                        if v.is_string() {
                            v.to_string()
                        } else {
                            return Err(RuntimeError::WrongArgumentSystemCallLine(
                                "Expected a string argument".to_string(),
                            ));
                        }
                    }
                    None => self.get_current_state().unwrap().story.clone(),
                };

                if let Some(paragraph_name) = systemcall_line.get_argument("paragraph") {
                    let paragraph_name = self
                        .executor
                        .get_rvalue(&self.context, paragraph_name)?
                        .to_owned();
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    let current_paragraph = self
                        .context
                        .stack_mut()
                        .pop()
                        .expect("No paragraph in stack to replace, this should not happen.");

                    loop {
                        if self.context.stack().is_empty() {
                            break;
                        }

                        // pop the stack until the last state is not the same on story and paragraph
                        // to remove all sub-blocks on the same paragraph
                        let last_state = self.context.stack().last().unwrap();
                        if last_state.story == current_paragraph.story
                            && last_state.paragraph == current_paragraph.paragraph
                        {
                            self.context.stack_mut().pop();
                        } else {
                            break;
                        }
                    }

                    let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();

                    self.context.stack_mut().push(ExecutionState::new(
                        story_name,
                        paragraph_name.to_string(),
                        paragraph.block,
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Paragraph name not provided".to_string(),
                    ));
                }
            }
            // This method will push a new state with the story and paragraph name,
            // once this new state is ended, it will return to the previous state
            "call" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
                        let v = self.executor.get_rvalue(&self.context, v)?;
                        if v.is_string() {
                            v.to_string()
                        } else {
                            return Err(RuntimeError::WrongArgumentSystemCallLine(
                                "Expected a string argument".to_string(),
                            ));
                        }
                    }
                    None => self.get_current_state().unwrap().story.clone(),
                };

                if let Some(paragraph_name) = systemcall_line.get_argument("paragraph") {
                    let paragraph_name = self
                        .executor
                        .get_rvalue(&self.context, paragraph_name)?
                        .to_owned();
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();

                    self.context.stack_mut().push(ExecutionState::new(
                        story_name,
                        paragraph_name.to_string(),
                        paragraph.block,
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Paragraph name not provided".to_string(),
                    ));
                }
            }
            // This method will quit the current paragraph and return to the previous one
            "break" => {
                self.break_current_block()?;
            }
            "finish" => {
                self.context.stack_mut().clear();
                self.executor.finished(&mut self.context);
            }
            _ => {
                self.executor
                    .handle_extra_system_call(&mut self.context, systemcall_line)?;
            }
        }

        Ok(())
    }
}
