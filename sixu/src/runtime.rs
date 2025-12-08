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

    pub async fn load_story(&mut self, story_name: &str) -> Result<()> {
        let data = self
            .executor
            .read_story_file(&mut self.context, story_name)
            .await?;

        let text = String::from_utf8(data)
            .map_err(|e| anyhow::anyhow!("Failed to parse story file: {}", e))?;

        let (_, story) = crate::parser::parse(story_name, &text).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse story file '{}': {}",
                story_name,
                e.to_string()
            )
        })?;

        self.context.stories_mut().push(story);
        Ok(())
    }

    pub fn has_story(&self, name: &str) -> bool {
        self.context.stories().iter().any(|s| s.name == name)
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

    pub async fn get_paragraph_or_load(
        &mut self,
        story_name: &str,
        name: &str,
    ) -> Result<&Paragraph> {
        if self.has_story(story_name) {
            return self.get_paragraph(story_name, name);
        }

        self.load_story(story_name).await?;
        self.get_paragraph(story_name, name)
    }

    pub fn list_stories(&self) -> Vec<String> {
        self.context
            .stories()
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    pub fn list_paragraphs(&self, story_name: &str) -> Result<Vec<String>> {
        let story = self.get_story(story_name)?;
        Ok(story.paragraphs.iter().map(|p| p.name.clone()).collect())
    }

    pub fn traverse_lines<F>(
        &mut self,
        story_name: &str,
        paragraph_name: &str,
        mut callback: F,
    ) -> Result<()>
    where
        F: FnMut(&ChildContent) -> Result<bool>,
    {
        let paragraph = self.get_paragraph(story_name, paragraph_name)?;

        for child in &paragraph.block.children {
            let is_continue = callback(&child.content)?;
            if !is_continue {
                break;
            }
        }

        Ok(())
    }

    pub fn save(&self) -> Result<Vec<ExecutionState>> {
        let stack = self.context.stack().clone();
        Ok(stack)
    }

    pub fn restore(&mut self, states: Vec<ExecutionState>) -> Result<()> {
        *self.context.stack_mut() = states;
        Ok(())
    }

    pub fn start(&mut self, story_name: &str, entry_name: Option<&str>) -> Result<()> {
        if self.context.stories().is_empty() {
            return Err(RuntimeError::NoStory);
        }

        let is_empty = self.context.stack().is_empty();
        if is_empty {
            let entry_name = entry_name.unwrap_or("entry");
            let paragraph = self.get_paragraph(story_name, entry_name)?;
            let block = paragraph.block.clone();
            self.context.stack_mut().push(ExecutionState::new(
                story_name.to_string(),
                entry_name.to_string(),
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

    pub async fn next(&mut self) -> Result<()> {
        loop {
            let is_continue;
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
                        is_continue = true;
                    }
                    ChildContent::TextLine(leading, text, tailing) => {
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
                        let tailing = match tailing {
                            TailingText::None => None,
                            TailingText::Text(t) => Some(t),
                        };
                        is_continue = self.executor.handle_text(
                            &mut self.context,
                            leading.as_deref(),
                            text.as_deref(),
                            tailing.as_deref(),
                        )?;
                    }
                    ChildContent::CommandLine(command) => {
                        is_continue = self.executor.handle_command(&mut self.context, &command)?;
                    }
                    ChildContent::SystemCallLine(systemcall) => {
                        is_continue = self.handle_system_call(&systemcall).await?;
                    }
                    ChildContent::EmbeddedCode(script) => {
                        is_continue = self.executor.eval_script(&mut self.context, &script)?.1;
                    }
                }
            } else {
                self.break_current_block()?;
                is_continue = true;
            }

            if !is_continue {
                break;
            }
        }

        Ok(())
    }

    /// Handle system call line, returns true if next() should be called again
    async fn handle_system_call(&mut self, systemcall_line: &SystemCallLine) -> Result<bool> {
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

                    let paragraph = self
                        .get_paragraph_or_load(&story_name, &paragraph_name)
                        .await?
                        .clone();

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

                Ok(true)
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

                    let paragraph = self
                        .get_paragraph_or_load(&story_name, &paragraph_name)
                        .await?
                        .clone();

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

                Ok(true)
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

                    let paragraph = self
                        .get_paragraph_or_load(&story_name, &paragraph_name)
                        .await?
                        .clone();

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

                Ok(true)
            }
            // This method will quit the current paragraph and return to the previous one
            "break" => {
                self.break_current_block()?;
                Ok(true)
            }
            "finish" => {
                self.context.stack_mut().clear();
                self.executor.finished(&mut self.context);
                Ok(false)
            }
            _ => self
                .executor
                .handle_extra_system_call(&mut self.context, systemcall_line),
        }
    }
}
