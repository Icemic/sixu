mod callback;
mod datasource;
mod executor;
mod state;

pub use self::callback::*;
pub use self::datasource::RuntimeDataSource;
pub use self::executor::RuntimeExecutor;
pub use self::state::ExecutionState;

use crate::error::{Result, RuntimeError};
use crate::format::*;

pub trait Runtime: RuntimeDataSource + RuntimeExecutor {
    fn add_story(&mut self, story: Story) {
        self.get_stories_mut().push(story);
    }

    fn get_story(&self, name: &str) -> Result<&Story> {
        self.get_stories()
            .iter()
            .find(|s| s.filename == name)
            .ok_or(RuntimeError::StoryNotFound(name.to_string()))
    }

    fn get_paragraph(&self, story_name: &str, name: &str) -> Result<&Paragraph> {
        let story = self.get_story(story_name)?;
        story
            .paragraphs
            .iter()
            .find(|s| s.name == name)
            .ok_or(RuntimeError::ParagraphNotFound(name.to_string()))
    }

    fn save(&self) -> Result<Vec<ExecutionState>> {
        let stack = self.get_stack().clone();
        Ok(stack)
    }

    fn restore(&mut self, states: Vec<ExecutionState>) -> Result<()> {
        *self.get_stack_mut() = states;
        Ok(())
    }

    fn start(&mut self, story_name: &str) -> Result<()> {
        if self.get_stories().is_empty() {
            return Err(RuntimeError::NoStory);
        }

        let is_empty = self.get_stack().is_empty();
        if is_empty {
            let paragraph = self.get_paragraph(story_name, "entry")?;
            let block = paragraph.block.clone();
            self.get_stack_mut().push(ExecutionState::new(
                story_name.to_string(),
                "entry".to_string(),
                block,
            ));
        } else {
            return Err(RuntimeError::StoryStarted);
        }

        Ok(())
    }

    fn terminate(&mut self) -> Result<()> {
        if self.get_stack().is_empty() {
            return Err(RuntimeError::StoryNotStarted);
        }

        self.get_stack_mut().clear();
        self.finished();

        Ok(())
    }

    fn get_current_state(&self) -> Result<&ExecutionState> {
        self.get_stack().last().ok_or(RuntimeError::StoryNotStarted)
    }

    fn get_current_state_mut(&mut self) -> Result<&mut ExecutionState> {
        self.get_stack_mut()
            .last_mut()
            .ok_or(RuntimeError::StoryNotStarted)
    }

    fn break_current_block(&mut self) -> Result<()> {
        if let Some(state) = self.get_stack_mut().pop() {
            // if the stack is empty, try to load the next paragraph of the current story
            if self.get_stack().is_empty() {
                if let Some(next_paragraph) = {
                    let story = self.get_story(&state.story)?;
                    let mut paragraph_iter = story.paragraphs.iter();
                    paragraph_iter.position(|s| s.name == state.paragraph);

                    paragraph_iter.next().cloned()
                } {
                    self.get_stack_mut().push(ExecutionState::new(
                        state.story.clone(),
                        next_paragraph.name,
                        next_paragraph.block,
                    ));
                } else {
                    self.finished();
                }
            }

            Ok(())
        } else {
            // Use this error to tell the user that the story is finished, who should
            // break the loop or stop the execution
            Err(RuntimeError::StoryFinished)
        }
    }

    fn next(&mut self) -> Result<()> {
        let current_state = self.get_current_state_mut()?;

        if let Some(child) = current_state.next_line() {
            let content = child.content;
            match content {
                ChildContent::Block(block) => {
                    let current_state = self.get_current_state()?.clone();
                    self.get_stack_mut().push(ExecutionState::new(
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
                            let text = self.calculate_template_literal(&template_literal)?;
                            Some(text)
                        }
                    };
                    let text = match text {
                        Text::None => None,
                        Text::Text(t) => Some(t),
                        Text::TemplateLiteral(template_literal) => {
                            let text = self.calculate_template_literal(&template_literal)?;
                            Some(text)
                        }
                    };
                    self.handle_text(leading.as_deref(), text.as_deref())?;
                }
                ChildContent::CommandLine(command) => {
                    self.handle_command(&command)?;
                }
                ChildContent::SystemCallLine(systemcall) => {
                    self.handle_system_call(&systemcall)?;
                }
                ChildContent::EmbeddedCode(script) => {
                    self.eval_script(&script)?;
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
                        let v = self.get_rvalue(v)?;
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
                    let paragraph_name = self.get_rvalue(paragraph_name)?.to_owned();
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    self.get_stack_mut().clear();

                    let paragraph = self
                        .get_paragraph(&paragraph_name, &paragraph_name)?
                        .clone();

                    self.get_stack_mut().push(ExecutionState::new(
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
                        let v = self.get_rvalue(v)?;
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
                    let paragraph_name = self.get_rvalue(paragraph_name)?.to_owned();
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    let current_paragraph = self
                        .get_stack_mut()
                        .pop()
                        .expect("No paragraph in stack to replace, this should not happen.");

                    loop {
                        if self.get_stack().is_empty() {
                            break;
                        }

                        // pop the stack until the last state is not the same on story and paragraph
                        // to remove all sub-blocks on the same paragraph
                        let last_state = self.get_stack().last().unwrap();
                        if last_state.story == current_paragraph.story
                            && last_state.paragraph == current_paragraph.paragraph
                        {
                            self.get_stack_mut().pop();
                        } else {
                            break;
                        }
                    }

                    let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();

                    self.get_stack_mut().push(ExecutionState::new(
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
                        let v = self.get_rvalue(v)?;
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
                    let paragraph_name = self.get_rvalue(paragraph_name)?.to_owned();
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();

                    self.get_stack_mut().push(ExecutionState::new(
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
                self.get_stack_mut().clear();
                self.finished();
            }
            _ => {
                self.handle_extra_system_call(systemcall_line)?;
            }
        }

        Ok(())
    }
}

impl<T: RuntimeDataSource + RuntimeExecutor> Runtime for T {}
