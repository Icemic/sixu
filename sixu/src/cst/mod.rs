//! Concrete Syntax Tree (CST) module
//!
//! This module provides a concrete syntax tree representation that preserves
//! all source code details including whitespace, comments, and token positions.
//! It is primarily used for LSP features and code formatting.

pub mod formatter;
pub mod node;
pub mod parser;
pub mod span;

pub use formatter::CstFormatter;
pub use node::*;
pub use parser::parse_tolerant;
pub use span::{Span, SpanInfo};
