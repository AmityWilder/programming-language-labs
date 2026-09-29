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

    fn equality(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.comparison()?;

        while let Some(op) = self.tokens.next_if(|token| {
            matches!(
                token.val,
                TokenValue::Punctuation(Punctuation::Neq | Punctuation::Eq)
            )
        }) {
            let rhs = self.comparison()?;
            expr = Expr::Binary(Box::new((expr, op, rhs)));
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.term()?;

        while let Some(op) = self.tokens.next_if(|token| {
            matches!(
                token.val,
                TokenValue::Punctuation(
                    Punctuation::Gt | Punctuation::Ge | Punctuation::Lt | Punctuation::Le
                )
            )
        }) {
            let rhs = self.term()?;
            expr = Expr::Binary(Box::new((expr, op, rhs)));
        }

        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.factor()?;

        while let Some(op) = self.tokens.next_if(|token| {
            matches!(
                token.val,
                TokenValue::Punctuation(Punctuation::Add | Punctuation::Sub)
            )
        }) {
            let rhs = self.factor()?;
            expr = Expr::Binary(Box::new((expr, op, rhs)));
        }

        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.exponent()?;

        while let Some(op) = self.tokens.next_if(|token| {
            matches!(
                token.val,
                TokenValue::Punctuation(Punctuation::Mul | Punctuation::Div)
            )
        }) {
            let rhs = self.exponent()?;
            expr = Expr::Binary(Box::new((expr, op, rhs)));
        }

        Ok(expr)
    }

    fn exponent(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary()?;

        while let Some(op) = self
            .tokens
            .next_if(|token| matches!(token.val, TokenValue::Punctuation(Punctuation::Exponent)))
        {
            let rhs = self.unary()?;
            expr = Expr::Binary(Box::new((expr, op, rhs)));
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(op) = self.tokens.next_if(|token| {
            matches!(
                token.val,
                TokenValue::Punctuation(Punctuation::Not | Punctuation::Sub)
            )
        }) {
            let rhs = self.unary()?;
            Ok(Expr::Unary(Box::new((op, rhs))))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(token) = self.tokens.next_if(|token| {
            matches!(
                token.val,
                TokenValue::BoolLiteral(_)
                    | TokenValue::Keyword(Keyword::None)
                    | TokenValue::UIntLiteral(_)
                    | TokenValue::SIntLiteral(_)
                    | TokenValue::FltLiteral(_)
                    | TokenValue::CharLiteral(_)
                    | TokenValue::StringLiteral(_)
            )
        }) {
            Ok(Expr::Literal(token))
        } else if let Some(lparen) = self
            .tokens
            .next_if(|token| matches!(token.val, TokenValue::Punctuation(Punctuation::LParen)))
        {
            let expr = self.expression()?;
            if let Some(_rparen) = self
                .tokens
                .next_if(|token| matches!(token.val, TokenValue::Punctuation(Punctuation::RParen)))
            {
                Ok(Expr::Grouping(Box::new(expr)))
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

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Binary(Box<(Self, Token<'src>, Self)>),
    Unary(Box<(Token<'src>, Self)>),
    Literal(Token<'src>),
    Grouping(Box<Self>),
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
