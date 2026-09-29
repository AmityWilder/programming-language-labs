//! Context-free grammar

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::{ContextError, ErrorType, Expecting},
    scanner::{
        Bracket,
        token::{Keyword, Punctuation, Token, TokenValue},
    },
};
use std::range::Range;

macro_rules! match_token {
    ($pattern:pat) => {
        |token| matches!(token.val, $pattern)
    };
}

macro_rules! binary_op_seq {
    ($outer:ident ::= $lhs:ident ($op:pat) $rhs:ident) => {
        fn $outer(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            use Punctuation::*;
            self.binary_op(
                match_token!(TokenValue::Punctuation($op)),
                Self::$lhs,
                Self::$rhs,
            )
        }
    };
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Binary(Box<(Self, Token<'src>, Self)>),
    Unary(Box<(Token<'src>, Self)>),
    Literal(Token<'src>),
    Grouping(Box<Self>),
}

impl<'src> Expr<'src> {
    pub fn binary(lhs: Self, op: Token<'src>, rhs: Self) -> Self {
        Self::Binary(Box::new((lhs, op, rhs)))
    }

    pub fn unary(op: Token<'src>, rhs: Self) -> Self {
        Self::Unary(Box::new((op, rhs)))
    }

    pub const fn literal(literal: Token<'src>) -> Self {
        Self::Literal(literal)
    }

    pub fn grouping(inner: Self) -> Self {
        Self::Grouping(Box::new(inner))
    }
}

#[derive(Debug, Clone)]
pub struct Parser<'src, I: Iterator<Item = Token<'src>>> {
    source: &'src str,
    tokens: std::iter::Peekable<I>,
}

fn parse<'src, A>(
    source: &'src str,
    tokens: A,
) -> Parser<'src, std::iter::Filter<A::IntoIter, impl FnMut(&Token<'src>) -> bool>>
where
    A: IntoIterator<Item = Token<'src>>,
{
    Parser {
        source,
        tokens: tokens
            .into_iter()
            .filter(|token| !matches!(token.val, TokenValue::Whitespace | TokenValue::Comment))
            .peekable(),
    }
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    fn expression(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.equality()
    }

    fn binary_op(
        &mut self,
        sep: fn(&Token<'src>) -> bool,
        lhs: fn(&mut Self) -> Result<Expr<'src>, ContextError<'src>>,
        rhs: fn(&mut Self) -> Result<Expr<'src>, ContextError<'src>>,
    ) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = lhs(self)?;

        while let Some(op) = self.tokens.next_if(sep) {
            let rhs = rhs(self)?;
            expr = Expr::binary(expr, op, rhs);
        }

        Ok(expr)
    }

    binary_op_seq! { equality ::= comparison (Neq | Eq) comparison }
    binary_op_seq! { comparison ::= term (Gt | Ge | Lt | Le) term }
    binary_op_seq! { term ::= factor (Add | Sub) factor }
    binary_op_seq! { factor ::= exponent (Mul | Div) exponent }
    binary_op_seq! { exponent ::= unary (Exponent) unary }

    fn unary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        use Punctuation::*;
        if let Some(op) = self
            .tokens
            .next_if(match_token!(TokenValue::Punctuation(Not | Sub)))
        {
            Ok(Expr::unary(op, self.unary()?))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(token) = self.tokens.next_if(match_token!(
            TokenValue::BoolLiteral(_)
                | TokenValue::Keyword(Keyword::None)
                | TokenValue::UIntLiteral(_)
                | TokenValue::SIntLiteral(_)
                | TokenValue::FltLiteral(_)
                | TokenValue::CharLiteral(_)
                | TokenValue::StringLiteral(_)
        )) {
            Ok(Expr::literal(token))
        } else if let Some(lparen) = self
            .tokens
            .next_if(match_token!(TokenValue::Punctuation(Punctuation::LParen)))
        {
            let expr = self.expression()?;
            if let Some(_rparen) = self
                .tokens
                .next_if(match_token!(TokenValue::Punctuation(Punctuation::RParen)))
            {
                Ok(Expr::grouping(expr))
            } else {
                let peeked = self.tokens.peek();
                Err(ContextError {
                    source: self.source,
                    range: peeked.map_or(
                        Range::from(self.source.len()..self.source.len()),
                        |token| {
                            self.source
                                .substr_range(token.lex)
                                .expect("lexeme should be a substr of source code")
                        },
                    ),
                    err: match peeked {
                        Some(Token {
                            val:
                                TokenValue::Punctuation(
                                    punc @ (Punctuation::RBrace | Punctuation::RBrack),
                                ),
                            ..
                        }) => ErrorType::IncorrectCloseBracket {
                            expect: (
                                Bracket::Paren,
                                self.source
                                    .substr_range(lparen.lex)
                                    .expect("lexeme should be a substr of source code"),
                            ),
                            actual: match punc {
                                Punctuation::RBrace => Bracket::Brace,
                                Punctuation::RBrack => Bracket::Brack,
                                _ => unreachable!("guarded by outer match arm"),
                            },
                        },

                        Some(token) => ErrorType::UnexpectedToken {
                            expect: Expecting::a("')'"),
                            actual: *token,
                        },

                        None => ErrorType::MissingCloseBracket {
                            expect: (
                                Bracket::Paren,
                                self.source
                                    .substr_range(lparen.lex)
                                    .expect("lexeme should be a substr of source code"),
                            ),
                        },
                    },
                })
            }
        } else {
            todo!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::tokenize;

    #[test]
    fn test_parse0() {
        const SOURCE: &str = "5 + -(7 / 8) * 3";
        let tokens = tokenize(SOURCE)
            .collect::<Result<Vec<_>, _>>()
            .expect("should not have a token error");
        let mut parser = parse(SOURCE, tokens);
        let expr = parser.expression().expect("should be a valid expression");
        assert_eq!(
            expr,
            Expr::Binary(Box::new((
                Expr::Literal(Token {
                    lex: "5",
                    val: TokenValue::UIntLiteral(5)
                }),
                Token {
                    lex: "+",
                    val: TokenValue::Punctuation(Punctuation::Add)
                },
                Expr::Binary(Box::new((
                    Expr::Unary(Box::new((
                        Token {
                            lex: "-",
                            val: TokenValue::Punctuation(Punctuation::Sub)
                        },
                        Expr::Grouping(Box::new(Expr::Binary(Box::new((
                            Expr::Literal(Token {
                                lex: "7",
                                val: TokenValue::UIntLiteral(7)
                            }),
                            Token {
                                lex: "/",
                                val: TokenValue::Punctuation(Punctuation::Div)
                            },
                            Expr::Literal(Token {
                                lex: "8",
                                val: TokenValue::UIntLiteral(8)
                            })
                        )))))
                    ))),
                    Token {
                        lex: "*",
                        val: TokenValue::Punctuation(Punctuation::Mul)
                    },
                    Expr::Literal(Token {
                        lex: "3",
                        val: TokenValue::UIntLiteral(3)
                    })
                )))
            )))
        );
    }
}
