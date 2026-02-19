use crate::format::{Literal, Story};

use super::ExecutionState;

/// Loop control signal for `#breakloop` and `#continue` system calls
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopControl {
    /// Break out of the current loop
    Break,
    /// Skip the rest of the current iteration and re-evaluate the loop condition
    Continue,
}

/// Runtime context that holds the execution state and data
#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// Loaded stories
    stories: Vec<Story>,
    /// Current execution state stack
    stack: Vec<ExecutionState>,
    /// Game session variables
    archive_variables: Literal,
    /// Permanent variables
    global_variables: Literal,
    /// Pending loop control signal
    loop_control: Option<LoopControl>,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self {
            stories: Vec::new(),
            stack: Vec::new(),
            archive_variables: Literal::Object(Default::default()),
            global_variables: Literal::Object(Default::default()),
            loop_control: None,
        }
    }
}

impl RuntimeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stories(&self) -> &Vec<Story> {
        &self.stories
    }

    pub fn stories_mut(&mut self) -> &mut Vec<Story> {
        &mut self.stories
    }

    pub fn stack(&self) -> &Vec<ExecutionState> {
        &self.stack
    }

    pub fn stack_mut(&mut self) -> &mut Vec<ExecutionState> {
        &mut self.stack
    }

    pub fn archive_variables(&self) -> &Literal {
        &self.archive_variables
    }

    pub fn archive_variables_mut(&mut self) -> &mut Literal {
        &mut self.archive_variables
    }

    pub fn global_variables(&self) -> &Literal {
        &self.global_variables
    }

    pub fn global_variables_mut(&mut self) -> &mut Literal {
        &mut self.global_variables
    }

    /// Set a loop control signal
    pub fn set_loop_control(&mut self, control: LoopControl) {
        self.loop_control = Some(control);
    }

    /// Take the pending loop control signal (if any), clearing it
    pub fn take_loop_control(&mut self) -> Option<LoopControl> {
        self.loop_control.take()
    }
}
