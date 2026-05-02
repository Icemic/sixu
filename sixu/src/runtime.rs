mod callback;
mod datasource;
mod executor;
mod state;

pub use self::callback::*;
pub use self::datasource::{LoopControl, RuntimeContext};
pub use self::executor::RuntimeExecutor;
pub use self::state::ExecutionState;

use crate::error::{Result, RuntimeError};
use crate::format::*;

/// Result of a single step of runtime execution
#[derive(Debug)]
pub enum StepResult {
    /// Execution paused normally (e.g. awaiting user input)
    Done,
    /// The runtime needs a condition to be evaluated externally.
    /// Call `resume_condition()` with the result, then call `step()` again.
    NeedsCondition(String),
    /// The runtime needs a script to be evaluated externally.
    /// Call `resume_script()` with the result, then call `step()` again.
    NeedsScript(String),
    /// The runtime needs a story file to be loaded.
    /// Call `provide_story_data()` with the file contents, then call `step()` again.
    NeedsStoryFile(String),
}

/// Internal state tracking for step/resume execution
enum StepPhase {
    /// Ready for normal execution
    Ready,
    /// Yielded for condition evaluation; child is saved for resumption
    AwaitingCondition { child: Child },
    /// Yielded for script evaluation
    AwaitingScript,
    /// Yielded for story file loading; paragraph target saved
    AwaitingStoryFile {
        story_name: String,
        paragraph_name: String,
    },
}

impl Default for StepPhase {
    fn default() -> Self {
        StepPhase::Ready
    }
}

/// Runtime manages the execution context and executor together
pub struct Runtime<E: RuntimeExecutor> {
    context: RuntimeContext,
    executor: E,
    /// Internal phase for step/resume execution
    phase: StepPhase,
    /// Condition result provided by the caller after NeedsCondition
    condition_result: Option<bool>,
    /// Script result provided by the caller after NeedsScript
    script_result: Option<(Option<RValue>, bool)>,
}

impl<E: RuntimeExecutor> Runtime<E> {
    pub fn new(executor: E) -> Self {
        Self {
            context: RuntimeContext::new(),
            executor,
            phase: StepPhase::default(),
            condition_result: None,
            script_result: None,
        }
    }

    pub fn new_with_context(executor: E, context: RuntimeContext) -> Self {
        Self {
            context,
            executor,
            phase: StepPhase::default(),
            condition_result: None,
            script_result: None,
        }
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

    /// Resolve all variables in the argument list to literal values
    pub fn resolve_arguments(&mut self, args: Vec<Argument>) -> Result<Vec<ResolvedArgument>> {
        let mut resolved_args = Vec::new();
        for arg in args {
            let resolved_value = self
                .executor
                .get_rvalue(&self.context, &arg.value)?
                .to_owned();
            resolved_args.push(ResolvedArgument {
                name: arg.name.clone(),
                value: resolved_value,
            });
        }
        Ok(resolved_args)
    }

    /// Execute steps synchronously until paused or an external async operation is needed.
    ///
    /// Returns `StepResult::Done` when execution pauses (e.g. awaiting user input).
    /// Returns `StepResult::NeedsCondition`, `NeedsScript`, or `NeedsStoryFile` when
    /// an external async operation is required. The caller should perform the operation,
    /// call the corresponding resume method, then call `step()` again.
    pub fn step(&mut self) -> Result<StepResult> {
        loop {
            if let Some(result) = self.step_one()? {
                return Ok(result);
            }
        }
    }

    /// Process one iteration of the execution loop.
    /// Returns `None` if the loop should continue, or `Some(StepResult)` to yield.
    fn step_one(&mut self) -> Result<Option<StepResult>> {
        // Handle resume from pending phase
        match std::mem::replace(&mut self.phase, StepPhase::Ready) {
            StepPhase::Ready => {} // normal path
            StepPhase::AwaitingCondition { child } => {
                // Resuming after condition evaluation
                return self.process_child(child);
            }
            StepPhase::AwaitingScript => {
                // Resuming after script evaluation
                let (_, is_continue) = self
                    .script_result
                    .take()
                    .expect("resumed from AwaitingScript without script result");
                return Ok(if is_continue {
                    None
                } else {
                    Some(StepResult::Done)
                });
            }
            StepPhase::AwaitingStoryFile {
                story_name,
                paragraph_name,
            } => {
                // Story should now be loaded, look up the paragraph and push state
                let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();
                self.context.stack_mut().push(ExecutionState::new(
                    story_name,
                    paragraph_name,
                    paragraph.block,
                ));
                return Ok(None); // continue execution
            }
        }

        // Check loop control signal from #break / #continue
        if let Some(control) = self.context.take_loop_control() {
            // Pop states until we find the loop body state
            let found = self.pop_to_loop_body();
            if found {
                match control {
                    LoopControl::Break => {
                        // Advance parent index past the loop child (undo the decrement)
                        if let Ok(parent_state) = self.get_current_state_mut() {
                            parent_state.index += 1;
                        }
                    }
                    LoopControl::Continue => {
                        // Parent index is already at the loop child (decremented),
                        // so the next iteration will re-evaluate the condition
                    }
                }
            } else {
                log::warn!("Loop control signal received but no loop body found in stack");
            }
            return Ok(None); // continue
        }

        let current_state = self.get_current_state_mut()?;
        if let Some(child) = current_state.next_line() {
            self.process_child(child)
        } else {
            self.break_current_block()?;
            Ok(None) // continue
        }
    }

    /// Process a single child (attributes + content).
    /// Called both for fresh children and when resuming after condition evaluation.
    fn process_child(&mut self, child: Child) -> Result<Option<StepResult>> {
        let mut is_loop = false;
        let marker = child.marker.clone();

        // Extract attribute info before potentially moving child
        let (keyword, condition) = if !child.attributes.is_empty() {
            if child.attributes.len() > 1 {
                log::warn!("Multiple attributes on same child, only last one is used");
            }
            let attr = child.attributes.last().unwrap();
            (attr.keyword.clone(), attr.condition.clone())
        } else {
            (String::new(), None)
        };

        // Process attributes
        if !keyword.is_empty() {
            match keyword.as_str() {
                "cond" | "if" => {
                    if let Some(ref cond_str) = condition {
                        let result = match self.condition_result.take() {
                            Some(r) => r,
                            None => {
                                let cond_str = cond_str.clone();
                                self.phase = StepPhase::AwaitingCondition { child };
                                return Ok(Some(StepResult::NeedsCondition(cond_str)));
                            }
                        };
                        if !result {
                            if let Some(marker) = marker.as_ref() {
                                self.executor.handle_marker(&mut self.context, marker)?;
                            }
                            return Ok(None); // condition not met, skip this child
                        }
                    }
                }
                "while" => {
                    if let Some(ref cond_str) = condition {
                        let result = match self.condition_result.take() {
                            Some(r) => r,
                            None => {
                                let cond_str = cond_str.clone();
                                self.phase = StepPhase::AwaitingCondition { child };
                                return Ok(Some(StepResult::NeedsCondition(cond_str)));
                            }
                        };
                        if !result {
                            if let Some(marker) = marker.as_ref() {
                                self.executor.handle_marker(&mut self.context, marker)?;
                            }
                            return Ok(None); // condition not met, skip this child
                        }
                        self.get_current_state_mut()?.index -= 1;
                        is_loop = true;
                    }
                }
                "loop" => {
                    self.get_current_state_mut()?.index -= 1;
                    is_loop = true;
                }
                _ => {
                    log::warn!("Unknown attribute keyword: {}", keyword);
                }
            }
        }

        // Process content
        let is_continue = match child.content {
            ChildContent::Block(block) => {
                let current_state = self.get_current_state()?.clone();
                if is_loop {
                    self.context.stack_mut().push(ExecutionState::new_loop_body(
                        current_state.story,
                        current_state.paragraph,
                        block.clone(),
                    ));
                } else {
                    self.context.stack_mut().push(ExecutionState::new(
                        current_state.story,
                        current_state.paragraph,
                        block.clone(),
                    ));
                }
                true
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
                self.executor.handle_text(
                    &mut self.context,
                    leading.as_deref(),
                    text.as_deref(),
                    tailing.as_deref(),
                )?
            }
            ChildContent::CommandLine(command) => {
                let command = ResolvedCommandLine {
                    command: command.command,
                    arguments: self.resolve_arguments(command.arguments)?,
                };
                self.executor.handle_command(&mut self.context, &command)?
            }
            ChildContent::SystemCallLine(systemcall) => {
                let systemcall = ResolvedSystemCallLine {
                    command: systemcall.command,
                    arguments: self.resolve_arguments(systemcall.arguments)?,
                };
                match self.handle_system_call(&systemcall)? {
                    Some(v) => v,
                    None => {
                        // Phase was set to AwaitingStoryFile by handle_system_call
                        let story_name = match &self.phase {
                            StepPhase::AwaitingStoryFile { story_name, .. } => story_name.clone(),
                            _ => unreachable!(),
                        };
                        return Ok(Some(StepResult::NeedsStoryFile(story_name)));
                    }
                }
            }
            ChildContent::EmbeddedCode(script) => {
                if let Some((_, is_continue)) = self.script_result.take() {
                    is_continue
                } else {
                    self.phase = StepPhase::AwaitingScript;
                    return Ok(Some(StepResult::NeedsScript(script)));
                }
            }
        };

        if let Some(marker) = marker.as_ref() {
            self.executor.handle_marker(&mut self.context, marker)?;
        }

        Ok(if is_continue {
            None
        } else {
            Some(StepResult::Done)
        })
    }

    /// Provide the result of a condition evaluation after `step()` returned `NeedsCondition`.
    /// Call `step()` again after this to continue execution.
    pub fn resume_condition(&mut self, result: bool) {
        self.condition_result = Some(result);
    }

    /// Provide the result of a script evaluation after `step()` returned `NeedsScript`.
    /// `result` is the evaluated value (or None), `is_continue` indicates whether
    /// execution should continue immediately after this script.
    /// Call `step()` again after this to continue execution.
    pub fn resume_script(&mut self, result: Option<RValue>, is_continue: bool) {
        self.script_result = Some((result, is_continue));
    }

    /// Provide story file data after `step()` returned `NeedsStoryFile`.
    /// The data will be parsed and added to the story list.
    /// Call `step()` again after this to continue execution.
    pub fn provide_story_data(&mut self, story_name: &str, data: Vec<u8>) -> Result<()> {
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

    /// Pop states from the stack until a loop body state is found and popped.
    /// Returns true if a loop body was found, false otherwise.
    fn pop_to_loop_body(&mut self) -> bool {
        while let Some(state) = self.context.stack_mut().pop() {
            if state.is_loop_body {
                return true;
            }
        }
        false
    }

    /// Handle system call line synchronously.
    /// Returns `Ok(Some(is_continue))` for normal completion, or `Ok(None)` when
    /// a story file needs to be loaded (phase set to `AwaitingStoryFile`).
    fn handle_system_call(
        &mut self,
        systemcall_line: &ResolvedSystemCallLine,
    ) -> Result<Option<bool>> {
        match systemcall_line.command.as_str() {
            "goto" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
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
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    self.context.stack_mut().clear();

                    if self.has_story(&story_name) {
                        let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();
                        self.context.stack_mut().push(ExecutionState::new(
                            story_name,
                            paragraph_name,
                            paragraph.block,
                        ));
                    } else {
                        self.phase = StepPhase::AwaitingStoryFile {
                            story_name,
                            paragraph_name,
                        };
                        return Ok(None);
                    }
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Paragraph name not provided".to_string(),
                    ));
                }

                Ok(Some(true))
            }
            "replace" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
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

                    if self.has_story(&story_name) {
                        let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();
                        self.context.stack_mut().push(ExecutionState::new(
                            story_name,
                            paragraph_name,
                            paragraph.block,
                        ));
                    } else {
                        self.phase = StepPhase::AwaitingStoryFile {
                            story_name,
                            paragraph_name,
                        };
                        return Ok(None);
                    }
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Paragraph name not provided".to_string(),
                    ));
                }

                Ok(Some(true))
            }
            "call" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
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
                    let paragraph_name = if paragraph_name.is_string() {
                        paragraph_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    if self.has_story(&story_name) {
                        let paragraph = self.get_paragraph(&story_name, &paragraph_name)?.clone();
                        self.context.stack_mut().push(ExecutionState::new(
                            story_name,
                            paragraph_name,
                            paragraph.block,
                        ));
                    } else {
                        self.phase = StepPhase::AwaitingStoryFile {
                            story_name,
                            paragraph_name,
                        };
                        return Ok(None);
                    }
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Paragraph name not provided".to_string(),
                    ));
                }

                Ok(Some(true))
            }
            "leave" => {
                self.break_current_block()?;
                Ok(Some(true))
            }
            "break" => {
                self.context.set_loop_control(LoopControl::Break);
                Ok(Some(true))
            }
            "continue" => {
                self.context.set_loop_control(LoopControl::Continue);
                Ok(Some(true))
            }
            "finish" => {
                self.context.stack_mut().clear();
                self.executor.finished(&mut self.context);
                Ok(Some(false))
            }
            _ => self
                .executor
                .handle_extra_system_call(&mut self.context, systemcall_line)
                .map(Some),
        }
    }
}
