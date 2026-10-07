//! Syntax used for highlighting

use std::marker::Destruct;

use crate::{
    highlight::style::Style,
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
    TextLiteral,
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
pub struct SyntaxStyle<T, A> {
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
    /// WARNING: Cannot be empty
    pub bracket: A,
    /// Style for [`Syntax::Invalid`]
    pub invalid: T,
}

impl<T, A> std::ops::Index<Syntax> for SyntaxStyle<T, A>
where
    A: AsRef<[T]>,
{
    type Output = T;

    fn index(&self, index: Syntax) -> &Self::Output {
        match index {
            Syntax::Normal => &self.normal,
            Syntax::Dimmed => &self.dimmed,
            Syntax::Comment => &self.comment,
            Syntax::NumberLiteral => &self.number_literal,
            Syntax::CharLiteral => &self.char_literal,
            Syntax::TextLiteral => &self.string_literal,
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
                .as_ref()
                .get(
                    depth
                        .checked_rem(self.bracket.as_ref().len())
                        .expect("BracketPair list should be non-empty"),
                )
                .expect("arr[n % len(arr)] should always be valid"),

            Syntax::Invalid => &self.invalid,
        }
    }
}

impl<const N: usize> SyntaxStyle<Style, [Style; N]> {
    /// Constructs an empty syntax style
    pub const fn new() -> Self {
        Self {
            normal: Style::new(),
            comment: Style::new(),
            dimmed: Style::new(),
            number_literal: Style::new(),
            char_literal: Style::new(),
            string_literal: Style::new(),
            escape_seq: Style::new(),
            language_defined: Style::new(),
            variable: Style::new(),
            constant: Style::new(),
            callable: Style::new(),
            keyword: Style::new(),
            ctrl_keyword: Style::new(),
            typename: Style::new(),
            macro_name: Style::new(),
            macro_arg: Style::new(),
            bracket: [Style::new(); N],
            invalid: Style::new(),
        }
    }
}

impl SyntaxStyle<Style, &[Style]> {
    /// Constructs an empty syntax style
    pub const fn new() -> Self {
        Self {
            normal: Style::new(),
            comment: Style::new(),
            dimmed: Style::new(),
            number_literal: Style::new(),
            char_literal: Style::new(),
            string_literal: Style::new(),
            escape_seq: Style::new(),
            language_defined: Style::new(),
            variable: Style::new(),
            constant: Style::new(),
            callable: Style::new(),
            keyword: Style::new(),
            ctrl_keyword: Style::new(),
            typename: Style::new(),
            macro_name: Style::new(),
            macro_arg: Style::new(),
            bracket: const { &[Style::new()] },
            invalid: Style::new(),
        }
    }
}

impl<T, A> SyntaxStyle<T, A>
where
    A: AsRef<[T]>,
{
    pub const fn normal(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.normal = value;
        self
    }

    pub const fn comment(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.comment = value;
        self
    }

    pub const fn dimmed(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.dimmed = value;
        self
    }

    pub const fn number_literal(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.number_literal = value;
        self
    }

    pub const fn char_literal(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.char_literal = value;
        self
    }

    pub const fn string_literal(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.string_literal = value;
        self
    }

    pub const fn escape_seq(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.escape_seq = value;
        self
    }

    pub const fn language_defined(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.language_defined = value;
        self
    }

    pub const fn variable(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.variable = value;
        self
    }

    pub const fn constant(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.constant = value;
        self
    }

    pub const fn callable(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.callable = value;
        self
    }

    pub const fn keyword(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.keyword = value;
        self
    }

    pub const fn ctrl_keyword(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.ctrl_keyword = value;
        self
    }

    pub const fn typename(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.typename = value;
        self
    }

    pub const fn macro_name(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.macro_name = value;
        self
    }

    pub const fn macro_arg(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.macro_arg = value;
        self
    }

    pub const fn bracket(mut self, value: A) -> Self
    where
        A: [const] Destruct + [const] AsRef<[T]>,
    {
        assert!(
            !value.as_ref().is_empty(),
            "bracket list cannot be empty; it would cause an \"n % 0\" error"
        );
        self.bracket = value;
        self
    }

    pub const fn invalid(mut self, value: T) -> Self
    where
        T: [const] Destruct,
    {
        self.invalid = value;
        self
    }
}

impl Token<'_> {
    pub fn syntax(&self) -> Syntax {
        match self.val {
            LexValue::Comment => Syntax::Comment,
            LexValue::Whitespace => Syntax::Dimmed,
            LexValue::UIntLiteral(_) | LexValue::SIntLiteral(_) | LexValue::FracLiteral(_) => {
                Syntax::NumberLiteral
            }
            LexValue::CharLiteral(_) => Syntax::CharLiteral,
            LexValue::TextLiteral(_) => Syntax::TextLiteral,
            LexValue::BoolLiteral(_) => Syntax::LanguageDefined,
            LexValue::Identifier => {
                // constants are all-caps
                if self.lex.chars().any(char::is_uppercase) {
                    if self.lex.chars().any(char::is_lowercase) {
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
                } else if kw.is_type() && /* TODO: differenciate type vs value in semantics */ !kw.is_value()
                {
                    Syntax::Typename
                } else {
                    Syntax::Keyword
                }
            }
            LexValue::Macro => Syntax::MacroName,
            LexValue::MacroParam => Syntax::MacroParam,
            LexValue::Punctuation(_) => Syntax::Normal,
        }
    }
}

/// Define a syntax style using JSON-style syntax (without quotes)
macro_rules! syntax_style {
    ($(
        $item:ident:
        // simple
        $({ $($key:ident: $value:expr),* })?
        // bracket
        $([$({ $($brack_key:ident: $brack_value:expr),* }),+])?
    ),*) => {{
        #[allow(unused_imports)]
        use $crate::highlight::style::Color::*;
        $crate::highlight::syntax::SyntaxStyle::<$crate::highlight::style::Style, [_; _]>::new()
            $(
                .$item(
                    // simple
                    $(
                        $crate::highlight::style::Style::new()$(.$key($value))*
                    )?
                    // bracket
                    $(const {
                        [$( $crate::highlight::style::Style::new()$(.$brack_key($brack_value))* ),+]
                    })?
                )
            )*
    }};
}
pub(crate) use syntax_style;
