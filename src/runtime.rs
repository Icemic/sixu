mod callback;
mod state;

use std::sync::{Arc, Mutex};

use arc_swap::ArcSwapOption;

pub use self::callback::*;
use self::state::SceneState;

use crate::error::{Result, RuntimeError};
use crate::format::{Child, CommandLine, Primitive, RValue, Scene, Story, SystemCallLine};

/// Sixu scripting language runtime
pub struct Runtime {
    stories: Vec<Story>,
    stack: Vec<SceneState>,
    command_handler: Arc<ArcSwapOption<Mutex<OnCommandHandler>>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            stories: Vec::new(),
            stack: Vec::new(),
            command_handler: Arc::new(ArcSwapOption::new(None)),
        }
    }

    pub fn add_story(&mut self, story: Story) {
        self.stories.push(story);
    }

    pub fn set_command_handler(&self, handler: OnCommandHandler) {
        self.command_handler
            .store(Some(Arc::new(Mutex::new(handler))));
    }

    pub fn get_story(&self, name: &str) -> Result<&Story> {
        self.stories
            .iter()
            .find(|s| s.filename == name)
            .ok_or(RuntimeError::StoryNotFound(name.to_string()))
    }

    pub fn get_scene(&self, story_name: &str, name: &str) -> Result<&Scene> {
        let story = self.get_story(story_name)?;
        story
            .scenes
            .iter()
            .find(|s| s.name == name)
            .ok_or(RuntimeError::SceneNotFound(name.to_string()))
    }

    pub fn start(&mut self, story_name: &str) -> Result<()> {
        if self.stories.is_empty() {
            return Err(RuntimeError::NoStory);
        }

        if self.stack.is_empty() {
            self.stack
                .push(SceneState::new(story_name.to_string(), "entry".to_string()));
        } else {
            return Err(RuntimeError::StoryStarted);
        }

        Ok(())
    }

    pub fn get_current_state(&self) -> Result<&SceneState> {
        self.stack.last().ok_or(RuntimeError::StoryNotStarted)
    }

    pub fn get_current_state_mut(&mut self) -> Result<&mut SceneState> {
        self.stack.last_mut().ok_or(RuntimeError::StoryNotStarted)
    }

    pub fn next(&mut self) -> Result<()> {
        let current_state = self.get_current_state()?;
        let scene = self.get_scene(&current_state.story, &current_state.scene)?;

        if let Some(child) = scene.block.children.get(current_state.index).cloned() {
            match child {
                Child::Block(_) => todo!(),
                Child::TextLine(_) => todo!(),
                Child::CommandLine(command) => {
                    self.handle_command(&command)?;
                }
                Child::SystemCallLine(systemcall) => {
                    self.handle_system_call(&systemcall)?;
                }
            }
        } else {
            // end of scene
            self.stack.pop();
        }

        self.stack.last_mut().unwrap().index += 1;

        Ok(())
    }

    pub fn handle_command(&self, command_line: &CommandLine) -> Result<()> {
        if let Some(handler) = self.command_handler.load().as_ref() {
            let mut handler = handler.lock().unwrap();
            let future = handler(command_line);
            pollster::block_on(future);
        } else {
            log::warn!("No command handler set");
        }

        Ok(())
    }

    pub fn handle_system_call(&mut self, systemcall_line: &SystemCallLine) -> Result<()> {
        match systemcall_line.command.as_str() {
            "goto" => {
                if let Some(scene_name) = systemcall_line
                    .arguments
                    .iter()
                    .find(|arg| arg.name == "scene")
                {
                    let scene_name = scene_name
                        .value
                        .as_ref()
                        .ok_or(RuntimeError::WrongArgumentSystemCallLine)?;
                    let scene_name = self.get_rvalue(scene_name)?.to_owned();

                    let current = self.get_current_state_mut()?;
                    current.scene = scene_name.to_string();
                    current.index = 0;
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine);
                }
            }
            "call" => {
                if let Some(scene_name) = systemcall_line
                    .arguments
                    .iter()
                    .find(|arg| arg.name == "scene")
                {
                    let scene_name = scene_name
                        .value
                        .as_ref()
                        .ok_or(RuntimeError::WrongArgumentSystemCallLine)?;
                    let scene_name = self.get_rvalue(&scene_name)?;
                    let current = self.get_current_state()?;
                    self.stack.push(SceneState::new(
                        current.story.clone(),
                        scene_name.to_string(),
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine);
                }
            }
            _ => todo!(),
        }

        Ok(())
    }

    pub fn get_rvalue<'a>(&self, value: &'a RValue) -> Result<&'a Primitive> {
        match value {
            RValue::Primitive(s) => Ok(s),
            RValue::Variable(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Block, Scene};

    #[test]
    fn test_runtime() {
        let mut runtime = Runtime::new();
        let story = Story {
            filename: "test".to_string(),
            scenes: vec![Scene {
                name: "entry".to_string(),
                parameters: vec![],
                block: Block {
                    attributes: vec![],
                    children: vec![Child::CommandLine(CommandLine {
                        command: "print".to_string(),
                        flags: vec![],
                        arguments: vec![],
                    })],
                },
            }],
        };

        runtime.set_command_handler(Box::new(|command_line| {
            assert_eq!(command_line.command, "print");
            Box::pin(async {})
        }));

        runtime.add_story(story);
        runtime.start("test").unwrap();
        runtime.next().unwrap();
    }
}
