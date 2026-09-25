//! Context-free grammar

#![allow(clippy::missing_docs_in_private_items, reason = "under construction")]

use crate::{
    error::ContextError,
    scanner::token::{Keyword, Punctuation, Token, TokenType, TokenValue},
};

pub trait MatchRule<'a>: Sized
where
    Token<'a>: Into<Token<'a>>,
{
    type Output;

    fn try_pull_matching<'b>(
        self,
        source: &'a str,
        tokens: &'b [Token<'a>],
        expecting: &'static str,
    ) -> Result<(Self::Output, &'b [Token<'a>]), ContextError<'a>>;
}

impl<'a, U, F> MatchRule<'a> for F
where
    F: FnOnce(Token<'a>) -> Option<U>,
{
    type Output = U;

    fn try_pull_matching<'b>(
        self,
        source: &'a str,
        mut tokens: &'b [Token<'a>],
        expecting: &'static str,
    ) -> Result<(Self::Output, &'b [Token<'a>]), ContextError<'a>> {
        tokens
            .split_off_first()
            .ok_or_else(|| ContextError::missing(source, expecting))
            .and_then(|token| {
                self(*token)
                    .map(|x| (x, tokens))
                    .ok_or_else(|| ContextError::unexpected(*token, source, expecting))
            })
    }
}

pub trait Rule<'a>: Sized {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>>;
}

/// `<let-statement> ::= "let" <binding> "=" <expression>`
#[derive(Debug, Clone, PartialEq)]
pub struct LetStatement<'a> {
    pub let_kw: &'a str,
    pub binding: Binding<'a>,
    pub assign_kw: &'a str,
    pub expression: Expression<'a>,
}

impl<'a> Rule<'a> for LetStatement<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (let_kw, tokens) = MatchRule::try_pull_matching(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Let),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            "`let`",
        )?;

        let (binding, tokens) = Binding::try_pull(source, tokens)?;

        let (assign_kw, tokens) = MatchRule::try_pull_matching(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::Assign),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            "`=`",
        )?;

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

/// `<binding> ::= <identifier>`
// TODO: this can be way cooler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Binding<'a> {
    name: &'a str,
}

impl<'a> Rule<'a> for Binding<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (name, tokens) = MatchRule::try_pull_matching(
            |token| match token {
                Token {
                    src,
                    ty: TokenType::Identifier,
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            "identifier",
        )?;

        Ok((Self { name }, tokens))
    }
}

/// `<group> ::= "(" <expression> ")"`
#[derive(Debug, Clone, PartialEq)]
pub struct Group<'a> {
    pub open: &'a str,
    pub inner: Box<Expression<'a>>,
    pub close: &'a str,
}

impl<'a> Rule<'a> for Group<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = MatchRule::try_pull_matching(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::LParen),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            "open parentheses `(`",
        )?;

        let (inner, tokens) = Expression::try_pull(source, tokens)?;

        let (close, tokens) = MatchRule::try_pull_matching(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::RParen),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            "close parentheses`)`",
        )?;

        Ok((
            Self {
                open,
                inner: Box::new(inner),
                close,
            },
            tokens,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Literal<'a> {
    pub token: Token<'a>,
}

impl<'a> Rule<'a> for Literal<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (token, tokens) = MatchRule::try_pull_matching(
            |token| match token {
                Token {
                    ty:
                        TokenType::BoolLiteral
                        | TokenType::NumberLiteral
                        | TokenType::CharLiteral
                        | TokenType::StringLiteral,
                    ..
                } => Some(token),
                _ => None,
            },
            source,
            tokens,
            "literal",
        )?;

        Ok((Self { token }, tokens))
    }
}

/// ```not_code
/// <binary-operation> ::= <expression> <binary-operator> <expression>
/// <binary-operator> ::= "+" | "-" | "*" | "/" | "&" | ...
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOperation<'a> {
    pub lhs: Box<Expression<'a>>,
    pub op: Token<'a>,
    pub rhs: Box<Expression<'a>>,
}

impl<'a> Rule<'a> for BinaryOperation<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        todo!()
    }
}

/// ```not_code
/// <expression> ::= <literal>
///     | <group>
///     | <array>
///     | <binary-operation>
///     | <prefix-operator> <expression> ; TODO
///     | <expression> <postfix-operator> ; TODO
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'a> {
    Literal(Literal<'a>),
    Group(Group<'a>),
    BinOp(BinaryOperation<'a>),
}

impl<'a> Rule<'a> for Expression<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        if let Ok((value, tokens)) = Group::try_pull(source, tokens) {
            Ok((Self::Group(value), tokens))
        } else if let Ok((value, tokens)) = Literal::try_pull(source, tokens) {
            Ok((Self::Literal(value), tokens))
        } else if let Ok((value, tokens)) = BinaryOperation::try_pull(source, tokens) {
            Ok((Self::BinOp(value), tokens))
        }
        // TODO: are there other expression structures?
        else {
            Err(ContextError::missing_or_unexpected(
                tokens.first(),
                source,
                "expression",
            ))
        }
    }
}
