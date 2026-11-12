//! Syntax used for highlighting

use crate::{
    highlight::style::Style,
    lex::token::{IdentCase, TokenKind},
    scanner::token::{Token, punc::Punctuation, value::LexValue},
};
use std::marker::Destruct;

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
    /// A jump label
    Label,
    /// A local variable
    Variable,
    /// A structure field
    Field,
    /// A function parameter
    Param,
    /// A value that does not change at runtime
    Constant,
    /// A categorical type discriminant
    Discriminant,
    /// A supertype variant
    Variant,
    /// A function, method, or variable being called
    Callable,
    /// A language keyword that defines items or variables
    Keyword,
    /// A language keyword that resembles assembly labels
    FlowKeyword,
    /// A type name
    Typename,
    /// A module/library name
    Modname,
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
    /// Style for [`Syntax::TextLiteral`]
    pub string_literal: T,
    /// Style for [`Syntax::EscapeSeq`]
    pub escape_seq: T,
    /// Style for [`Syntax::LanguageDefined`]
    pub language_defined: T,
    /// Style for [`Syntax::Label`]
    pub label: T,
    /// Style for [`Syntax::Variable`]
    pub variable: T,
    /// Style for [`Syntax::Field`]
    pub field: T,
    /// Style for [`Syntax::Param`]
    pub param: T,
    /// Style for [`Syntax::Constant`]
    pub constant: T,
    /// Style for [`Syntax::Discriminant`]
    pub discriminant: T,
    /// Style for [`Syntax::Variant`]
    pub variant: T,
    /// Style for [`Syntax::Callable`]
    pub callable: T,
    /// Style for [`Syntax::Keyword`]
    pub keyword: T,
    /// Style for [`Syntax::FlowKeyword`]
    pub flow_keyword: T,
    /// Style for [`Syntax::Typename`]
    pub typename: T,
    /// Style for [`Syntax::Modname`]
    pub modname: T,
    /// Style for [`Syntax::MacroName`]
    pub macro_name: T,
    /// Style for [`Syntax::MacroParam`]
    pub macro_param: T,
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
            Syntax::Label => &self.label,
            Syntax::Variable => &self.variable,
            Syntax::Field => &self.field,
            Syntax::Param => &self.param,
            Syntax::Constant => &self.constant,
            Syntax::Discriminant => &self.discriminant,
            Syntax::Variant => &self.variant,
            Syntax::Callable => &self.callable,
            Syntax::Keyword => &self.keyword,
            Syntax::FlowKeyword => &self.flow_keyword,
            Syntax::Typename => &self.typename,
            Syntax::Modname => &self.modname,
            Syntax::MacroName => &self.macro_name,
            Syntax::MacroParam => &self.macro_param,
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
            label: Style::new(),
            variable: Style::new(),
            field: Style::new(),
            param: Style::new(),
            constant: Style::new(),
            discriminant: Style::new(),
            variant: Style::new(),
            callable: Style::new(),
            keyword: Style::new(),
            flow_keyword: Style::new(),
            typename: Style::new(),
            modname: Style::new(),
            macro_name: Style::new(),
            macro_param: Style::new(),
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
            label: Style::new(),
            variable: Style::new(),
            field: Style::new(),
            param: Style::new(),
            constant: Style::new(),
            discriminant: Style::new(),
            variant: Style::new(),
            callable: Style::new(),
            keyword: Style::new(),
            flow_keyword: Style::new(),
            typename: Style::new(),
            modname: Style::new(),
            macro_name: Style::new(),
            macro_param: Style::new(),
            bracket: const { &[Style::new()] },
            invalid: Style::new(),
        }
    }
}

pub trait GetSyntax {
    fn syntax(&self) -> Syntax;
}

impl GetSyntax for Token<'_> {
    fn syntax(&self) -> Syntax {
        match self.val {
            LexValue::Comment => Syntax::Comment,

            LexValue::Whitespace => Syntax::Dimmed,

            LexValue::ByteLiteral(_)
            | LexValue::UIntLiteral(_)
            | LexValue::SIntLiteral(_)
            | LexValue::FlptLiteral(_) => Syntax::NumberLiteral,

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
                    Syntax::FlowKeyword
                } else if kw.is_type() && /* TODO: differenciate type vs value in semantics */ !kw.is_value()
                {
                    Syntax::Typename
                } else {
                    Syntax::Keyword
                }
            }

            LexValue::Macro => Syntax::MacroName,

            LexValue::MacroParam => Syntax::MacroParam,

            LexValue::Punctuation(
                Punctuation::MacroConcat
                | Punctuation::MacroStringify
                | Punctuation::MacroConst
                | Punctuation::Coalesce,
            ) => Syntax::FlowKeyword,

            LexValue::Punctuation(
                Punctuation::Exists
                | Punctuation::Convert
                | Punctuation::Transmute
                | Punctuation::Incr
                | Punctuation::Decr
                | Punctuation::Iff,
            ) => Syntax::Keyword,

            LexValue::Punctuation(_) => Syntax::Normal,
        }
    }
}

impl GetSyntax for (&str, TokenKind) {
    fn syntax(&self) -> Syntax {
        match self.1 {
            TokenKind::Invalid => Syntax::Invalid,
            TokenKind::Comment => Syntax::Comment,
            TokenKind::WSpace => Syntax::Dimmed,
            TokenKind::NumberLit => Syntax::NumberLiteral,
            TokenKind::CharLit => Syntax::CharLiteral,
            TokenKind::TextLit => Syntax::TextLiteral,
            TokenKind::BoolLit(_) => Syntax::LanguageDefined,
            TokenKind::Macro => Syntax::MacroName,
            TokenKind::MParam => Syntax::MacroParam,

            TokenKind::Ident => {
                let case = IdentCase::from(self.0);
                match case {
                    IdentCase::Other | IdentCase::Snake => Syntax::Variable,
                    IdentCase::Pascal => Syntax::Typename,
                    IdentCase::Screaming => Syntax::Constant,
                }
            }

            TokenKind::Keyword(kw) => {
                if kw.is_flow() {
                    Syntax::FlowKeyword
                } else if kw.is_type() && /* TODO: differenciate type vs value in semantics */ !kw.is_value()
                {
                    Syntax::Typename
                } else {
                    Syntax::Keyword
                }
            }

            TokenKind::Punc(
                Punctuation::MacroConcat
                | Punctuation::MacroStringify
                | Punctuation::MacroConst
                | Punctuation::Coalesce,
            ) => Syntax::FlowKeyword,

            TokenKind::Punc(
                Punctuation::Exists
                | Punctuation::Convert
                | Punctuation::Transmute
                | Punctuation::Incr
                | Punctuation::Decr
                | Punctuation::Iff,
            ) => Syntax::Keyword,

            TokenKind::Punc(_) => Syntax::Normal,
        }
    }
}

impl GetSyntax for (&str, Syntax) {
    fn syntax(&self) -> Syntax {
        self.1
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
        let mut style = $crate::highlight::syntax::SyntaxStyle::<$crate::highlight::style::Style, [_; _]>::new();
        $(
            style.$item = (
                // simple
                $(
                    $crate::highlight::style::Style::new()$(.$key($value))*
                )?
                // bracket
                $(const {
                    [$( $crate::highlight::style::Style::new()$(.$brack_key($brack_value))* ),+]
                })?
            );
        )*
        style
    }};
}
pub(crate) use syntax_style;
