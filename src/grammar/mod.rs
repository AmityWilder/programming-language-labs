//! Context-free grammar

#![allow(clippy::missing_docs_in_private_items, reason = "under construction")]

use crate::{
    error::{ContextError, ErrorType},
    scanner::token::{Keyword, Punctuation, StrLiteral, Token, TokenType, TokenValue},
};

pub trait TerminatingRule<'a, T: StrLiteral>: Sized {
    type Output;

    fn try_pull_matching<'b>(
        self,
        source: &'a str,
        tokens: &'b [Token<'a, T>],
        expecting: &'static str,
    ) -> Result<(Self::Output, &'b [Token<'a, T>]), ContextError<'a>>;
}

macro_rules! rule {
    (($source:expr, $tokens:expr) $expecting:literal: $pattern:pat => $res:expr) => {
        (|token| {
            if let $pattern = token {
                Some($res)
            } else {
                None
            }
        })
        .try_pull_matching($source, $tokens, $expecting)
    };
}

impl<'a, T, U, F> TerminatingRule<'a, T> for F
where
    Token<'a, T>: Into<Token<'a, &'a str>>,
    T: StrLiteral + Clone,
    F: FnOnce(Token<'a, T>) -> Option<U>,
{
    type Output = U;

    fn try_pull_matching<'b>(
        self,
        source: &'a str,
        mut tokens: &'b [Token<'a, T>],
        expect: &'static str,
    ) -> Result<(Self::Output, &'b [Token<'a, T>]), ContextError<'a>> {
        tokens
            .split_off_first()
            .ok_or(ContextError {
                source,
                range: (source.len()..source.len()).into(),
                err: ErrorType::MissingToken { expect },
            })
            .and_then(|token| {
                if let Some(x) = self(token.clone()) {
                    Ok((x, tokens))
                } else {
                    Err(ContextError {
                        source,
                        range: source
                            .substr_range(token.src)
                            .expect("token src should be a substring of the source code"),
                        err: ErrorType::UnexpectedToken {
                            expect,
                            actual: token.clone().into(),
                        },
                    })
                }
            })
    }
}

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
        tokens: &'b [Token<'a, T>],
    ) -> Result<(Self, &'b [Token<'a, T>]), ContextError<'a>> {
        let (let_kw, tokens) = rule!((source, tokens) "`let`": Token { src, val: TokenValue::Keyword(Keyword::Let), .. } => src)?;

        let (binding, tokens) = Binding::try_pull(source, tokens)?;

        let (assign_kw, tokens) = rule!((source, tokens) "`=`": Token { src, val: TokenValue::Punctuation(Punctuation::Assign), .. } => src)?;

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
        tokens: &'b [Token<'a, T>],
    ) -> Result<(Self, &'b [Token<'a, T>]), ContextError<'a>> {
        let (name, tokens) = rule!((source, tokens) "identifier": Token { src, ty: TokenType::Identifier, .. } => src)?;

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
