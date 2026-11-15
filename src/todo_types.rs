#![allow(clippy::disallowed_types, reason = "definition")]

/// Indicates unfinished code.
///
/// This can be useful if you are prototyping and just
/// want a placeholder to let your code pass type analysis.
pub type Todo = !;

/// Indicates unfinished code.
///
/// This can be useful if you are prototyping and just
/// want a placeholder to let your code pass type analysis.
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Default)]
pub struct Dummy;
