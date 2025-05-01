use nom_language::error::VerboseError;
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
    #[error("Story finished")]
    StoryFinished,
    #[error("Story {0} not found")]
    StoryNotFound(String),
    #[error("Paragraph {0} not found")]
    ParagraphNotFound(String),
    #[error("Wrong argument(s) provided to system call line: {0}")]
    WrongArgumentSystemCallLine(String),
    #[error("Wrong argument(s) provided to command line: {0}")]
    WrongArgumentCommandLine(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] VerboseError<&'static str>),
}
