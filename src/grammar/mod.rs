//! Context-free grammar

#![allow(clippy::missing_docs_in_private_items, reason = "under construction")]

use crate::{
    error::{ContextError, ErrorType},
    scanner::token::{Keyword, Punctuation, StrLiteral, Token, TokenValue},
};

pub trait Rule<'a, T: StrLiteral>: Sized {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a, T>],
    ) -> Result<(Self, &'b [Token<'a, T>]), ContextError<'a>>;
}

/// `<let_statement> ::= "let" <binding> "=" <expression>`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LetStatement<'a> {
    pub let_kw: &'a str,
    pub binding: Binding<'a>,
    pub assign_kw: &'a str,
    pub expression: Expression,
}

impl<'a, T: StrLiteral + Clone> Rule<'a, T> for LetStatement<'a>
where
    Token<'a, T>: Into<Token<'a, &'a str>>,
{
    fn try_pull<'b>(
        source: &'a str,
        mut tokens: &'b [Token<'a, T>],
    ) -> Result<(Self, &'b [Token<'a, T>]), ContextError<'a>> {
        let let_kw = tokens
            .split_off_first()
            .cloned()
            .ok_or(ContextError {
                source,
                range: (source.len()..source.len()).into(),
                err: ErrorType::MissingToken { expect: "`let`" },
            })
            .and_then(|token| {
                if let Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Let),
                    ..
                } = token
                {
                    Ok(src)
                } else {
                    Err(ContextError {
                        source,
                        range: source
                            .substr_range(token.src)
                            .expect("token src should be a substring of the source code"),
                        err: ErrorType::UnexpectedToken {
                            expect: "`let`",
                            actual: token.into(),
                        },
                    })
                }
            })?;

        let (binding, mut tokens) = Binding::try_pull(source, tokens)?;

        let assign_kw = tokens
            .split_off_first()
            .cloned()
            .ok_or(ContextError {
                source,
                range: (source.len()..source.len()).into(),
                err: ErrorType::MissingToken { expect: "`=`" },
            })
            .and_then(|token| {
                if let Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::Assign),
                    ..
                } = token
                {
                    Ok(src)
                } else {
                    Err(ContextError {
                        source,
                        range: source
                            .substr_range(token.src)
                            .expect("token src should be a substring of the source code"),
                        err: ErrorType::UnexpectedToken {
                            expect: "`=`",
                            actual: token.into(),
                        },
                    })
                }
            })?;

        let (expression, tokens) = Expression::try_pull(source, tokens)?;

        Ok((
            Self {
                let_kw,
                binding,
                assign_kw,
                expression,
            },
            tokens,
        ))
    }
}

/// `<binding> ::= IDENTIFIER`
// TODO: this can be way cooler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Binding<'a> {
    name: &'a str,
}

impl<'a, T: StrLiteral + Clone> Rule<'a, T> for Binding<'a>
where
    Token<'a, T>: Into<Token<'a, &'a str>>,
{
    fn try_pull<'b>(
        source: &'a str,
        mut tokens: &'b [Token<'a, T>],
    ) -> Result<(Self, &'b [Token<'a, T>]), ContextError<'a>> {
        let name = tokens
            .split_off_first()
            .cloned()
            .ok_or(ContextError {
                source,
                range: (source.len()..source.len()).into(),
                err: ErrorType::MissingToken { expect: "`let`" },
            })
            .and_then(|token| {
                if let Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Let),
                    ..
                } = token
                {
                    Ok(src)
                } else {
                    Err(ContextError {
                        source,
                        range: source
                            .substr_range(token.src)
                            .expect("token src should be a substring of the source code"),
                        err: ErrorType::UnexpectedToken {
                            expect: "`let`",
                            actual: token.into(),
                        },
                    })
                }
            })?;

        Ok((Self { name }, tokens))
    }
}

/// `<expression> ::= ` TODO
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Expression {}

impl<'a, T: StrLiteral> Rule<'a, T> for Expression {
    fn try_pull<'b>(
        _source: &'a str,
        _tokens: &'b [Token<'a, T>],
    ) -> Result<(Self, &'b [Token<'a, T>]), ContextError<'a>> {
        todo!()
    }
}
