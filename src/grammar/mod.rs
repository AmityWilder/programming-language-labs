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
        token::{Token, keyword::Keyword, punc::Punctuation, value::Value},
    },
};
use std::range::Range;

macro_rules! match_token {
    ($($variant:ident$(($pattern:pat))?)|+) => {
        |token| matches!(token.val, $($crate::scanner::token::value::Value::$variant$(($pattern))?)|+)
    };
}

macro_rules! binary_op_seq {
    ($( $outer:ident -> $lhs:ident ( ( $($op:ident)|+ ) $rhs:ident )* ; )+) => {$(
        fn $outer(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            let mut expr = self.$lhs()?;

            while let Some(op) = self.tokens.next_if(match_token!(Punctuation($(Punctuation::$op)|+))) {
                let rhs = self.$rhs()?;
                expr = Expr::binary(expr, op, rhs);
            }

            Ok(expr)
        }
    )+};
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
            .filter(|token| !matches!(token.val, Value::Whitespace | Value::Comment))
            .peekable(),
    }
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    fn expression(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.equality()
    }

    binary_op_seq! {
        equality   -> comparison ( (Neq | Eq) comparison )* ;
        comparison -> term ( (Gt | Ge | Lt | Le) term )* ;
        term       -> factor ( (Add | Sub) factor )* ;
        factor     -> exponent ( (Mul | Div) exponent )* ;
        exponent   -> unary ( (Exp) unary )* ;
    }

    fn unary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        use Punctuation::*;
        if let Some(op) = self.tokens.next_if(match_token!(Punctuation(Not | Sub))) {
            Ok(Expr::unary(op, self.unary()?))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(token) = self.tokens.next_if(match_token!(
            BoolLiteral(_)
                | Keyword(Keyword::None)
                | UIntLiteral(_)
                | SIntLiteral(_)
                | FltLiteral(_)
                | CharLiteral(_)
                | StringLiteral(_)
        )) {
            Ok(Expr::literal(token))
        } else if let Some(lparen) = self
            .tokens
            .next_if(match_token!(Punctuation(Punctuation::LParen)))
        {
            let expr = self.expression()?;
            if self
                .tokens
                .next_if(match_token!(Punctuation(Punctuation::RParen)))
                .is_some()
            {
                Ok(Expr::grouping(expr))
            } else {
                let peeked = self.tokens.peek();
                Err(ContextError {
                    source: self.source,
                    range: peeked
                        .map_or(Range::from(self.source.len()..self.source.len()), |token| {
                            token.lex_range(self.source)
                        }),
                    err: match peeked {
                        Some(Token {
                            val:
                                Value::Punctuation(punc @ (Punctuation::RBrace | Punctuation::RBrack)),
                            ..
                        }) => ErrorType::IncorrectCloseBracket {
                            expect: (Bracket::Paren, lparen.lex_range(self.source)),
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
                            expect: (Bracket::Paren, lparen.lex_range(self.source)),
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
                    val: Value::UIntLiteral(5)
                }),
                Token {
                    lex: "+",
                    val: Value::Punctuation(Punctuation::Add)
                },
                Expr::Binary(Box::new((
                    Expr::Unary(Box::new((
                        Token {
                            lex: "-",
                            val: Value::Punctuation(Punctuation::Sub)
                        },
                        Expr::Grouping(Box::new(Expr::Binary(Box::new((
                            Expr::Literal(Token {
                                lex: "7",
                                val: Value::UIntLiteral(7)
                            }),
                            Token {
                                lex: "/",
                                val: Value::Punctuation(Punctuation::Div)
                            },
                            Expr::Literal(Token {
                                lex: "8",
                                val: Value::UIntLiteral(8)
                            })
                        )))))
                    ))),
                    Token {
                        lex: "*",
                        val: Value::Punctuation(Punctuation::Mul)
                    },
                    Expr::Literal(Token {
                        lex: "3",
                        val: Value::UIntLiteral(3)
                    })
                )))
            )))
        );
    }
}
