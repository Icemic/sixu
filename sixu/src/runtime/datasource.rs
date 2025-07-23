use crate::format::{Literal, Story};

use super::ExecutionState;

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
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self {
            stories: Vec::new(),
            stack: Vec::new(),
            archive_variables: Literal::Object(Default::default()),
            global_variables: Literal::Object(Default::default()),
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
}
