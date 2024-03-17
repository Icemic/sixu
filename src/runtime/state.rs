#[derive(Debug, Clone)]
pub struct SceneState {
    pub story: String,
    pub scene: String,
    pub index: usize,
}

impl SceneState {
    pub fn new(story: String, scene: String) -> Self {
        Self {
            story,
            scene,
            index: 0,
        }
    }
}
