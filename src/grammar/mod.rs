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

macro_rules! match_token {
    ($($variant:ident$(($pattern:pat))?)|+) => {
        |token| matches!(token.val, $($crate::scanner::token::value::Value::$variant$(($pattern))?)|+)
    };
}

macro_rules! binary_op_seq {
    ($( $outer:ident -> $lhs:ident ( ( $($op:ident)|+ ) $rhs:ident )* ; )+) => {$(
        #[doc = concat!("`", stringify!($outer -> $lhs ( ( $($op)|+ ) $rhs )* ;), "`")]
        fn $outer(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            let mut expr = self.$lhs()?;

            while let Some(op) = self.tokens.next_if(match_token!(Punctuation($(Punctuation::$op)|+))) {
                let rhs = self.$rhs()?;
                expr = Expr::binary(Binary { lhs: expr, op, rhs });
            }

            Ok(expr)
        }
    )+};
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary<'src> {
    pub lhs: Expr<'src>,
    pub op: Token<'src>,
    pub rhs: Expr<'src>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<'src> {
    pub op: Token<'src>,
    pub rhs: Expr<'src>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Binary(Box<Binary<'src>>),
    Unary(Box<Unary<'src>>),
    Literal(Token<'src>),
    Grouping(Box<Self>),
}

impl<'src> Expr<'src> {
    pub fn binary(inner: Binary<'src>) -> Self {
        Self::Binary(Box::new(inner))
    }

    pub fn unary(inner: Unary<'src>) -> Self {
        Self::Unary(Box::new(inner))
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

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    fn new(source: &'src str, tokens: I) -> Self {
        Self {
            source,
            tokens: tokens.peekable(),
        }
    }
}

impl<'src, I> Iterator for Parser<'src, I>
where
    I: Iterator<Item = Token<'src>>,
{
    type Item = Result<Expr<'src>, ContextError<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens
            .peek()
            .is_some()
            .then(|| self.statement().inspect_err(|_| self.synchronize()))
    }
}

pub fn parse<'src, A>(
    source: &'src str,
    tokens: A,
) -> Parser<'src, std::iter::Filter<<A as IntoIterator>::IntoIter, impl FnMut(&Token<'src>) -> bool>>
where
    A: IntoIterator<IntoIter: 'src, Item = Token<'src>>,
{
    Parser::new(
        source,
        tokens
            .into_iter()
            .filter(|token| !matches!(token.val, Value::Whitespace | Value::Comment)),
    )
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    fn pull_if<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.tokens.next_if(p)
    }

    fn try_pull<P>(&mut self, p: P, expected: Expecting) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.pull_if(p).ok_or_else(|| {
            ContextError::missing_or_unexpected(self.tokens.peek().copied(), self.source, expected)
        })
    }

    fn statement(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let expr = self.expression()?;
        self.try_pull(
            match_token!(Punctuation(Punctuation::Semi)),
            Expecting::a("`;`"),
        )?;
        Ok(expr)
    }

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
            Ok(Expr::unary(Unary {
                op,
                rhs: self.unary()?,
            }))
        } else {
            self.primary()
        }
    }

    fn literal(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.try_pull(
            match_token!(
                BoolLiteral(_)
                    | Keyword(Keyword::None)
                    | UIntLiteral(_)
                    | SIntLiteral(_)
                    | FltLiteral(_)
                    | CharLiteral(_)
                    | StringLiteral(_)
            ),
            Expecting::a("literal"),
        )
        .map(Expr::literal)
    }

    fn group(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let lparen = self.try_pull(
            match_token!(Punctuation(Punctuation::LParen)),
            Expecting::an("parenthesized expression"),
        )?;
        let expr = self.expression()?;
        self.try_pull(
                match_token!(Punctuation(Punctuation::RParen)),
                Expecting::a("`)`"),
            )
            .map(move |_| Expr::grouping(expr))
            .map_err(|e| ContextError {
                source: e.source,
                range: e.range,
                err: match e.err {
                    ErrorType::MissingToken { .. } => ErrorType::MissingCloseBracket {
                        expect: (Bracket::Paren, lparen.lex_range(self.source)),
                    },

                    ErrorType::UnexpectedToken {
                        actual:
                            Token {
                                val:
                                    Value::Punctuation(
                                        punc @ (Punctuation::RBrace | Punctuation::RBrack),
                                    ),
                                ..
                            },
                        ..
                    } => ErrorType::IncorrectCloseBracket {
                        expect: (Bracket::Paren, lparen.lex_range(self.source)),
                        actual: match punc {
                            Punctuation::RBrace => Bracket::Brace,
                            Punctuation::RBrack => Bracket::Brack,
                            _ => unreachable!("guarded by outer match arm"),
                        },
                    },

                    _ => e.err,
                },
            })
    }

    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.literal().or_else(|_| self.group()).map_err(|mut e| {
            if let ErrorType::MissingToken { expect } | ErrorType::UnexpectedToken { expect, .. } =
                &mut e.err
                && expect.expect == "parenthesized expression"
            {
                *expect = Expecting::an("expression");
            }
            e
        })
    }

    fn synchronize(&mut self) {
        while let Some(token) = self.tokens.next() {
            // end of current statement
            if matches!(token.val, Value::Punctuation(Punctuation::Semi)) {
                break;
            }

            // start of new statement/definition
            if self.tokens.peek().is_some_and(|token| {
                matches!(
                    token.val,
                    Value::Keyword(
                        Keyword::Rec
                            | Keyword::Sup
                            | Keyword::Cat
                            | Keyword::Alt
                            | Keyword::Sub
                            | Keyword::Def
                            | Keyword::Fn
                            | Keyword::Mem
                            | Keyword::Let
                            | Keyword::Uni
                            | Keyword::Pvt
                            | Keyword::If
                            | Keyword::Or
                            | Keyword::Match
                            | Keyword::Rep
                            | Keyword::For
                            | Keyword::Loop
                            | Keyword::Cord
                            | Keyword::Halt
                            | Keyword::Skip
                            | Keyword::Give
                            | Keyword::Fail
                            | Keyword::Emit
                    )
                )
            }) {
                break;
            }
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
        let expr = parse(SOURCE, tokens)
            .collect::<Result<Vec<_>, _>>()
            .expect("should be a valid expression");
        assert_eq!(
            expr.as_slice(),
            &[Expr::binary(Binary {
                lhs: Expr::literal(Token {
                    lex: "5",
                    val: Value::UIntLiteral(5)
                }),
                op: Token {
                    lex: "+",
                    val: Value::Punctuation(Punctuation::Add)
                },
                rhs: Expr::binary(Binary {
                    lhs: Expr::unary(Unary {
                        op: Token {
                            lex: "-",
                            val: Value::Punctuation(Punctuation::Sub)
                        },
                        rhs: Expr::grouping(Expr::binary(Binary {
                            lhs: Expr::literal(Token {
                                lex: "7",
                                val: Value::UIntLiteral(7)
                            }),
                            op: Token {
                                lex: "/",
                                val: Value::Punctuation(Punctuation::Div)
                            },
                            rhs: Expr::literal(Token {
                                lex: "8",
                                val: Value::UIntLiteral(8)
                            })
                        }))
                    }),
                    op: Token {
                        lex: "*",
                        val: Value::Punctuation(Punctuation::Mul)
                    },
                    rhs: Expr::Literal(Token {
                        lex: "3",
                        val: Value::UIntLiteral(3)
                    })
                })
            })]
        );
    }
}
