use crate::format::{Block, Child};

/// Represents a state in the stack of the runtime.
#[derive(Debug, Clone)]
pub struct SceneState {
    /// Story name
    pub story: String,
    /// Scene name
    pub scene: String,
    /// Current block in the scene
    pub block: Block,
    /// line index of the current block in the scene
    pub index: usize,
}

impl SceneState {
    pub fn new(story: String, scene: String, block: Block) -> Self {
        Self {
            story,
            scene,
            block,
            index: 0,
        }
    }
    pub fn next_line(&mut self) -> Option<Child> {
        let line = self.block.children.get(self.index).cloned();
        self.index += 1;
        line
    }
}
