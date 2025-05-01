use crate::format::Story;

use super::ParagraphState;

pub trait RuntimeDataSource {
    fn get_stories(&self) -> &Vec<Story>;
    fn get_stories_mut(&mut self) -> &mut Vec<Story>;
    fn get_stack(&self) -> &Vec<ParagraphState>;
    fn get_stack_mut(&mut self) -> &mut Vec<ParagraphState>;
}
