mod callback;
mod state;

pub use self::callback::*;
use self::state::SceneState;

use crate::error::{Result, RuntimeError};
use crate::executor::Executor;
use crate::format::*;

/// Sixu scripting language runtime
pub struct Runtime<T: Executor> {
    stories: Vec<Story>,
    stack: Vec<SceneState>,
    executor: T,
}

impl<T: Executor> Runtime<T> {
    pub fn new(executor: T) -> Self {
        Self {
            stories: Vec::new(),
            stack: Vec::new(),
            executor,
        }
    }

    pub fn add_story(&mut self, story: Story) {
        self.stories.push(story);
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
            let scene = self.get_scene(story_name, "entry")?;
            self.stack.push(SceneState::new(
                story_name.to_string(),
                "entry".to_string(),
                scene.block.clone(),
            ));
        } else {
            return Err(RuntimeError::StoryStarted);
        }

        Ok(())
    }

    fn get_current_state(&self) -> Result<&SceneState> {
        self.stack.last().ok_or(RuntimeError::StoryNotStarted)
    }

    pub fn get_current_state_mut(&mut self) -> Result<&mut SceneState> {
        self.stack.last_mut().ok_or(RuntimeError::StoryNotStarted)
    }

    pub fn next(&mut self) -> Result<()> {
        let current_state = self.get_current_state_mut()?;

        if let Some(child) = current_state.next_line() {
            let content = child.content;
            match content {
                ChildContent::Block(block) => {
                    let current_state = self.get_current_state()?;
                    self.stack.push(SceneState::new(
                        current_state.story.clone(),
                        current_state.scene.clone(),
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
                                .calculate_template_literal(&template_literal)?;
                            Some(text)
                        }
                    };
                    let text = match text {
                        Text::None => None,
                        Text::Text(t) => Some(t),
                        Text::TemplateLiteral(template_literal) => {
                            let text = self
                                .executor
                                .calculate_template_literal(&template_literal)?;
                            Some(text)
                        }
                    };
                    self.executor
                        .handle_text(leading.as_deref(), text.as_deref())?;
                }
                ChildContent::CommandLine(command) => {
                    self.executor.handle_command(&command)?;
                }
                ChildContent::SystemCallLine(systemcall) => {
                    self.handle_system_call(&systemcall)?;
                }
                ChildContent::EmbeddedCode(script) => {
                    self.executor.eval_script(&script)?;
                }
            }
        } else {
            if let Some(state) = self.stack.pop() {
                // if the stack is empty, try to load the next scene of the current story
                if self.stack.is_empty() {
                    if let Some(next_scene) = {
                        let story = self.get_story(&state.story)?;
                        let mut scene_iter = story.scenes.iter();
                        scene_iter.position(|s| s.name == state.scene);

                        scene_iter.next()
                    } {
                        self.stack.push(SceneState::new(
                            state.story.clone(),
                            next_scene.name.clone(),
                            next_scene.block.clone(),
                        ));
                    } else {
                        self.executor.finished();
                    }
                }
            } else {
                // Use this error to tell the user that the story is finished, who should
                // break the loop or stop the execution
                return Err(RuntimeError::StoryFinished);
            }
        }

        Ok(())
    }

    pub fn handle_system_call(&mut self, systemcall_line: &SystemCallLine) -> Result<()> {
        match systemcall_line.command.as_str() {
            // This method will clear the stack and push a new state with the story and scene name
            "goto" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
                        let v = self.executor.get_rvalue(v)?;
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

                if let Some(scene_name) = systemcall_line.get_argument("scene") {
                    let scene_name = self.executor.get_rvalue(scene_name)?.to_owned();
                    let scene_name = if scene_name.is_string() {
                        scene_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    self.stack.clear();

                    let scene = self.get_scene(&scene_name, &scene_name)?;

                    self.stack.push(SceneState::new(
                        story_name,
                        scene_name.to_string(),
                        scene.block.clone(),
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Scene name not provided".to_string(),
                    ));
                }
            }
            // This method will replace the current state with a new state with the story and scene name
            // once this new state is ended, it will return to the previous state
            "replace" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
                        let v = self.executor.get_rvalue(v)?;
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

                if let Some(scene_name) = systemcall_line.get_argument("scene") {
                    let scene_name = self.executor.get_rvalue(scene_name)?.to_owned();
                    let scene_name = if scene_name.is_string() {
                        scene_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    let current_scene = self
                        .stack
                        .pop()
                        .expect("No scene in stack to replace, this should not happen.");

                    loop {
                        if self.stack.is_empty() {
                            break;
                        }

                        // pop the stack until the last state is not the same on story and scene
                        // to remove all sub-blocks on the same scene
                        let last_state = self.stack.last().unwrap();
                        if last_state.story == current_scene.story
                            && last_state.scene == current_scene.scene
                        {
                            self.stack.pop();
                        } else {
                            break;
                        }
                    }

                    let scene = self.get_scene(&story_name, &scene_name)?;

                    self.stack.push(SceneState::new(
                        story_name,
                        scene_name.to_string(),
                        scene.block.clone(),
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Scene name not provided".to_string(),
                    ));
                }
            }
            // This method will push a new state with the story and scene name,
            // once this new state is ended, it will return to the previous state
            "call" => {
                let story_name = match systemcall_line.get_argument("story") {
                    Some(v) => {
                        let v = self.executor.get_rvalue(v)?;
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

                if let Some(scene_name) = systemcall_line.get_argument("scene") {
                    let scene_name = self.executor.get_rvalue(scene_name)?.to_owned();
                    let scene_name = if scene_name.is_string() {
                        scene_name.to_string()
                    } else {
                        return Err(RuntimeError::WrongArgumentSystemCallLine(
                            "Expected a string argument".to_string(),
                        ));
                    };

                    let scene = self.get_scene(&story_name, &scene_name)?;

                    self.stack.push(SceneState::new(
                        story_name,
                        scene_name.to_string(),
                        scene.block.clone(),
                    ));
                } else {
                    return Err(RuntimeError::WrongArgumentSystemCallLine(
                        "Scene name not provided".to_string(),
                    ));
                }
            }
            "finish" => {
                self.stack.clear();
                self.executor.finished();
            }
            _ => {
                self.executor.handle_system_call(systemcall_line)?;
            }
        }

        Ok(())
    }
}
