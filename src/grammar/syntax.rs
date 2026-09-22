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
    Bracket,
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
    pub bracket: T,
    pub bracket_pairs: &'a [T],
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
            Syntax::Bracket => &self.bracket,
            Syntax::Invalid => &self.invalid,
        }
    }
}

impl<T> SyntaxStyle<'_, T> {
    pub fn bracket_pair(&self, index: usize) -> &T {
        index
            .checked_rem(self.bracket_pairs.len())
            .map(|idx| {
                self.bracket_pairs
                    .get(idx)
                    .expect("list[n % len(list)] should always be valid")
            })
            .expect("BracketPair list should be non-empty")
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
                TokenType::Punctuation
                    if matches!(token.src, "[" | "]" | "(" | ")" | "{" | "}") =>
                {
                    Syntax::Bracket
                }

                TokenType::Whitespace | TokenType::Punctuation => Syntax::Normal,
            },
            value,
        ),
        Err(e) => (&e.source[e.range], Syntax::Invalid, &TokenValue::Ignore),
    }
}
