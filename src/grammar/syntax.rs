use crate::scanner::{
    error::TokenResult,
    token::{TokenType, TokenValue, TokenValueSimplicity},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Syntax {
    #[default]
    Normal,
    Comment,
    NumberLiteral,
    CharLiteral,
    StringLiteral,
    EscapeSeq,
    Variable,
    Constant,
    Callable,
    Keyword,
    CtrlKeyword,
    MacroName,
    MacroParam,
    Bracket(usize),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SyntaxStyle<'a, T> {
    pub normal: T,
    pub comment: T,
    pub number_literal: T,
    pub char_literal: T,
    pub string_literal: T,
    pub interp_str_literal: T,
    pub escape_seq: T,
    pub interp_expr: T,
    pub variable: T,
    pub constant: T,
    pub callable: T,
    pub keyword: T,
    pub ctrl_keyword: T,
    pub macro_name: T,
    pub macro_arg: T,
    pub bracket: &'a [T],
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
                TokenType::Bracket(depth) => Syntax::Bracket(depth),

                TokenType::Whitespace | TokenType::Punctuation => Syntax::Normal,
            },
            value,
        ),
        Err(e) => (&e.source[e.range], Syntax::Invalid, &TokenValue::Ignore),
    }
}
