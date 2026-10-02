//! Syntax used for highlighting

use crate::{
    error::ContextError,
    scanner::token::{Token, value::LexValue},
};

/// Syntactic element category for highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Syntax {
    /// Any element not described by other syntax categories
    #[default]
    Normal,
    /// Whitespace
    Dimmed,
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
    /// A language-defined constant like true/false
    LanguageDefined,
    /// A local variable, field, or function parameter
    Variable,
    /// A value that does not change at runtime
    Constant,
    /// A function, method, or variable being called
    Callable,
    /// A language keyword that defines items or variables
    Keyword,
    /// A language keyword that resembles assembly labels
    CtrlKeyword,
    /// A type name
    Typename,
    /// The name of a macro
    MacroName,
    /// The name of a macro parameter
    MacroParam,
    /// A bracket with depth-based coloring (other punctuation handled with [`Self::Normal`])
    Bracket(usize),
    /// Syntax errors
    Invalid,
}

/// A style table for [`Syntax`] elements.
/// `T`: The type used for styling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SyntaxStyle<'style, T> {
    /// Style for [`Syntax::Normal`]
    pub normal: T,
    /// Style for [`Syntax::Comment`]
    pub comment: T,
    /// Style for [`Syntax::Dimmed`]
    pub dimmed: T,
    /// Style for [`Syntax::NumberLiteral`]
    pub number_literal: T,
    /// Style for [`Syntax::CharLiteral`]
    pub char_literal: T,
    /// Style for [`Syntax::StringLiteral`]
    pub string_literal: T,
    /// Style for [`Syntax::EscapeSeq`]
    pub escape_seq: T,
    /// Style for [`Syntax::LanguageDefined`]
    pub language_defined: T,
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
    /// Style for [`Syntax::Typename`]
    pub typename: T,
    /// Style for [`Syntax::MacroName`]
    pub macro_name: T,
    /// Style for [`Syntax::MacroParam`]
    pub macro_arg: T,
    /// Style for [`Syntax::Bracket`]
    pub bracket: &'style [T],
    /// Style for [`Syntax::Invalid`]
    pub invalid: T,
}

impl<T> std::ops::Index<Syntax> for SyntaxStyle<'_, T> {
    type Output = T;

    fn index(&self, index: Syntax) -> &Self::Output {
        match index {
            Syntax::Normal => &self.normal,
            Syntax::Dimmed => &self.dimmed,
            Syntax::Comment => &self.comment,
            Syntax::NumberLiteral => &self.number_literal,
            Syntax::CharLiteral => &self.char_literal,
            Syntax::StringLiteral => &self.string_literal,
            Syntax::EscapeSeq => &self.escape_seq,
            Syntax::LanguageDefined => &self.language_defined,
            Syntax::Variable => &self.variable,
            Syntax::Constant => &self.constant,
            Syntax::Callable => &self.callable,
            Syntax::Keyword => &self.keyword,
            Syntax::CtrlKeyword => &self.ctrl_keyword,
            Syntax::Typename => &self.typename,
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

/// Identifies the [`Syntax`] of a token.
/// All [`Err`]s are [`Syntax::Invalid`].
///
/// # Panics
/// This method may panic if `item` is an error with a malformed [`range`](crate::error::ContextError::range).
pub fn syntax_of<'src, 'res>(
    item: &'res Result<Token<'src>, ContextError<'src>>,
) -> (&'src str, Syntax, &'res LexValue<'src>)
where
    'src: 'res,
{
    match item {
        Ok(token) => (
            token.lex,
            match token.val {
                LexValue::Comment => Syntax::Comment,
                LexValue::Whitespace => Syntax::Dimmed,
                LexValue::UIntLiteral(_) | LexValue::SIntLiteral(_) | LexValue::FltLiteral(_) => {
                    Syntax::NumberLiteral
                }
                LexValue::CharLiteral(_) => Syntax::CharLiteral,
                LexValue::StringLiteral(_) => Syntax::StringLiteral,
                LexValue::BoolLiteral(_) => Syntax::LanguageDefined,
                LexValue::Identifier => {
                    // constants are all-caps
                    if token.lex.chars().any(char::is_uppercase) {
                        if token.lex.chars().any(char::is_lowercase) {
                            Syntax::Typename
                        } else {
                            Syntax::Constant
                        }
                    } else {
                        Syntax::Variable
                    }
                }
                LexValue::Callable => Syntax::Callable,
                LexValue::Keyword(kw) => {
                    if kw.is_flow() {
                        Syntax::CtrlKeyword
                    } else if kw.is_type() {
                        Syntax::Typename
                    } else {
                        Syntax::Keyword
                    }
                }
                LexValue::Macro => Syntax::MacroName,
                LexValue::MacroParam => Syntax::MacroParam,

                _ => Syntax::Normal,
            },
            &token.val,
        ),
        Err(e) => (
            e.source
                .get(e.range)
                .expect("range should be a range of source"),
            Syntax::Invalid,
            &LexValue::Comment,
        ),
    }
}
