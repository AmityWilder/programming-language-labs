//! Constants for special characters/character sequences

/// A character literal delimiter - both open and close are identical
pub const CHAR_DELIM: char = '\'';
/// A string literal delimiter - both open and close are identical
pub const STR_DELIM: char = '\"';
/// An escape character
pub const ESCAPE: char = '\\';
/// The prefix of a macro identifier
pub const MACRO_PREFIX: char = '\\';
/// The prefix of a macro parameter identifier
pub const MACRO_PARAM_PREFIX: char = '$';
/// The prefix of a hexadecimal number literal
pub const HEX_PREFIX: &str = "0x";
/// The prefix of an octal number literal
pub const OCT_PREFIX: &str = "0o";
/// The prefix of a binary number literal
pub const BIN_PREFIX: &str = "0b";
/// The open delimiter of a block comment
pub const BLOCK_COMMENT_OPEN: &str = "/*";
/// The close delimiter of a block comment
pub const BLOCK_COMMENT_CLOSE: &str = "*/";
/// The open delimiter of a line comment (the close delimiter is the end of the line)
pub const LINE_COMMENT_OPEN: &str = "//";
