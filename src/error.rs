use thiserror::Error;

pub type Result<T, E = RuntimeError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("No story found")]
    NoStory,
    #[error("Story not started")]
    StoryNotStarted,
    #[error("Story has started")]
    StoryStarted,
    #[error("Story {0} not found")]
    StoryNotFound(String),
    #[error("Scene {0} not found")]
    SceneNotFound(String),
    #[error("Wrong argument(s) provided to system call line")]
    WrongArgumentSystemCallLine,
    #[error("Wrong argument(s) provided to command line")]
    WrongArgumentCommandLine,
}
