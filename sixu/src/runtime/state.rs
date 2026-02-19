#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::format::{Block, Child};

/// Represents a state in the stack of the runtime.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ExecutionState {
    /// Story name
    pub story: String,
    /// Paragraph name
    pub paragraph: String,
    /// Current block in the paragraph
    pub block: Block,
    /// line index of the current block in the paragraph
    pub index: usize,
    /// Whether this state is the body of a loop (while/loop attribute).
    /// Used by `#breakloop` and `#continue` to find the loop boundary.
    pub is_loop_body: bool,
}

impl ExecutionState {
    pub fn new(story: String, paragraph: String, block: Block) -> Self {
        Self {
            story,
            paragraph,
            block,
            index: 0,
            is_loop_body: false,
        }
    }

    pub fn new_loop_body(story: String, paragraph: String, block: Block) -> Self {
        Self {
            story,
            paragraph,
            block,
            index: 0,
            is_loop_body: true,
        }
    }
    pub fn next_line(&mut self) -> Option<Child> {
        let line = self.block.children.get(self.index).cloned();
        self.index += 1;
        line
    }
}
