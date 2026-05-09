#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::format::{Block, Child};

/// Represents a state in the stack of the runtime.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// It is essential for save archive compatibility that old archives can be loaded with new versions of the software,
// so we must ensure that new fields have default values when deserializing old archives.
#[serde(default)]
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
    /// Used by `#break` and `#continue` to find the loop boundary.
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
        while let Some(line) = self.block.children().get(self.index).cloned() {
            self.index += 1;

            if line.content.is_comment() {
                continue;
            }

            return Some(line);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{
        Block, Child, ChildContent, Comment, CommentKind, LeadingText, TailingText, Text,
    };

    use super::ExecutionState;

    #[test]
    fn next_line_skips_comment_children() {
        let block = Block::new(vec![
            Child {
                marker: None,
                attributes: vec![],
                content: ChildContent::Comment(Comment {
                    kind: CommentKind::Line,
                    content: " comment".to_string(),
                }),
            },
            Child {
                marker: None,
                attributes: vec![],
                content: ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("hello".to_string()),
                    TailingText::None,
                ),
            },
        ]);
        let mut state = ExecutionState::new("story".to_string(), "entry".to_string(), block);

        let child = state.next_line().expect("text child should be returned");
        assert!(matches!(
            child.content,
            ChildContent::TextLine(LeadingText::None, Text::Text(ref text), TailingText::None)
                if text == "hello"
        ));
        assert!(state.next_line().is_none());
    }

    #[test]
    fn next_line_skips_interleaved_comments_without_resetting_progress() {
        let block = Block::new(vec![
            Child {
                marker: None,
                attributes: vec![],
                content: ChildContent::Comment(Comment {
                    kind: CommentKind::Line,
                    content: " first".to_string(),
                }),
            },
            Child {
                marker: None,
                attributes: vec![],
                content: ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("a".to_string()),
                    TailingText::None,
                ),
            },
            Child {
                marker: None,
                attributes: vec![],
                content: ChildContent::Comment(Comment {
                    kind: CommentKind::Block,
                    content: " middle ".to_string(),
                }),
            },
            Child {
                marker: None,
                attributes: vec![],
                content: ChildContent::TextLine(
                    LeadingText::None,
                    Text::Text("b".to_string()),
                    TailingText::None,
                ),
            },
        ]);
        let mut state = ExecutionState::new("story".to_string(), "entry".to_string(), block);

        let first = state.next_line().expect("first text child should be returned");
        assert!(matches!(
            first.content,
            ChildContent::TextLine(LeadingText::None, Text::Text(ref text), TailingText::None)
                if text == "a"
        ));

        let second = state.next_line().expect("second text child should be returned");
        assert!(matches!(
            second.content,
            ChildContent::TextLine(LeadingText::None, Text::Text(ref text), TailingText::None)
                if text == "b"
        ));

        assert!(state.next_line().is_none());
    }
}
