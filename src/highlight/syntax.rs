//! Syntax used for highlighting

use crate::{
    error::TokenResult,
    scanner::token::{TokenType, TokenValue, TokenValueSimplicity},
};

/// Syntactic element category for highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Syntax {
    /// Any element not described by other syntax categories
    #[default]
    Normal,
    /// Comments
    Comment,
    /// Any number literal
    NumberLiteral,
    /// A character literal (excluding escape sequences)
    CharLiteral,
    /// A string literal (excluding escape sequences)
    StringLiteral,
    /// The escape sequence of either a character or string literal
    EscapeSeq,
    /// A local variable, field, or function parameter
    Variable,
    /// A value that does not change at runtime
    Constant,
    /// A function, method, or variable being called
    Callable,
    /// A language keyword that defines items or variables
    Keyword,
    /// A language keyword that affects runtime state
    CtrlKeyword,
    /// The name of a macro
    MacroName,
    /// The name of a macro parameter
    MacroParam,
    /// A bracket with depth-based coloring (other punctuation handled with [`Self::Normal`])
    #[expect(dead_code, reason = "reserved for future use")]
    Bracket(usize),
    /// Syntax errors
    Invalid,
}

/// A style table for [`Syntax`] elements.
/// `T`: The type used for styling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SyntaxStyle<'a, T> {
    /// Style for [`Syntax::Normal`]
    pub normal: T,
    /// Style for [`Syntax::Comment`]
    pub comment: T,
    /// Style for [`Syntax::NumberLiteral`]
    pub number_literal: T,
    /// Style for [`Syntax::CharLiteral`]
    pub char_literal: T,
    /// Style for [`Syntax::StringLiteral`]
    pub string_literal: T,
    /// Style for [`Syntax::EscapeSeq`]
    pub escape_seq: T,
    /// Style for [`Syntax::InterpExpr`]
    pub interp_expr: T,
    /// Style for [`Syntax::Variable`]
    pub variable: T,
    /// Style for [`Syntax::Constant`]
    pub constant: T,
    /// Style for [`Syntax::Callable`]
    pub callable: T,
    /// Style for [`Syntax::Keyword`]
    pub keyword: T,
    /// Style for [`Syntax::CtrlKeyword`]
    pub ctrl_keyword: T,
    /// Style for [`Syntax::MacroName`]
    pub macro_name: T,
    /// Style for [`Syntax::MacroArg`]
    pub macro_arg: T,
    /// Style for [`Syntax::Bracket`]
    pub bracket: &'a [T],
    /// Style for [`Syntax::Invalid`]
    pub invalid: T,
}

impl<T> std::ops::Index<Syntax> for SyntaxStyle<'_, T> {
    type Output = T;

    fn index(&self, index: Syntax) -> &Self::Output {
        match index {
            Syntax::Normal => &self.normal,
            Syntax::Comment => &self.comment,
            Syntax::NumberLiteral => &self.number_literal,
            Syntax::CharLiteral => &self.char_literal,
            Syntax::StringLiteral => &self.string_literal,
            Syntax::EscapeSeq => &self.escape_seq,
            Syntax::Variable => &self.variable,
            Syntax::Constant => &self.constant,
            Syntax::Callable => &self.callable,
            Syntax::Keyword => &self.keyword,
            Syntax::CtrlKeyword => &self.ctrl_keyword,
            Syntax::MacroName => &self.macro_name,
            Syntax::MacroParam => &self.macro_arg,
            Syntax::Bracket(depth) => self
                .bracket
                .get(
                    depth
                        .checked_rem(self.bracket.len())
                        .expect("BracketPair list should be non-empty"),
                )
                .expect("arr[n % len(arr)] should always be valid"),

            Syntax::Invalid => &self.invalid,
        }
    }
}

/// Identifies the [`Syntax`] of a [`TokenResult`].
/// All [`Err`]s are [`Syntax::Invalid`].
///
/// # Panics
/// This method may panic if `item` is an error with a malformed [`range`](crate::error::ContextError::range).
pub fn syntax_of<'a, 'b, S>(
    item: &'b TokenResult<'a, S>,
) -> (&'a str, Syntax, &'b TokenValue<'a, S>)
where
    'a: 'b,
    S: TokenValueSimplicity,
{
    match item {
        Ok((token, value)) => (
            token.src,
            match token.ty {
                TokenType::Comment => Syntax::Comment,
                TokenType::NumberLiteral => Syntax::NumberLiteral,
                TokenType::CharLiteral => Syntax::CharLiteral,
                TokenType::StringLiteral => Syntax::StringLiteral,
                TokenType::Identifier => {
                    // constants are all-caps
                    if token.src.chars().any(char::is_uppercase)
                        && !token.src.chars().any(char::is_lowercase)
                    {
                        Syntax::Constant
                    } else {
                        Syntax::Variable
                    }
                }
                TokenType::Callable => Syntax::Callable,
                TokenType::Keyword => Syntax::Keyword,
                TokenType::CtrlKeyword => Syntax::CtrlKeyword,
                TokenType::Macro => Syntax::MacroName,
                TokenType::MacroParam => Syntax::MacroParam,

                TokenType::Whitespace | TokenType::Punctuation => Syntax::Normal,
            },
            value,
        ),
        Err(e) => (
            e.source
                .get(e.range)
                .expect("range should be a range of source"),
            Syntax::Invalid,
            &TokenValue::Ignore,
        ),
    }
}
