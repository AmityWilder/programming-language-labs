//! Context-free grammar

use crate::{
    error::{ContextError, ErrorType, ExpectedToken, OpSide},
    scanner::{
        BadBracketCombo, Bracket,
        token::{Token, keyword::Keyword, punc::Punctuation, value::LexValue},
    },
};
use ast::{Binary, Expr, Grouping, OrType, TypeExpr, Unary};

pub mod ast;
pub mod ast_iter;
pub mod fmt;

macro_rules! match_token {
    ($($variant:ident$(($pattern:pat))?)|+) => {
        |token| matches!(token.val, $($crate::scanner::token::value::LexValue::$variant$(($pattern))?)|+)
    };
}
pub(crate) use match_token;

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
            .then(|| self.expression().inspect_err(|_| self.synchronize()))
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
            .filter(|token| !matches!(token.val, LexValue::Whitespace | LexValue::Comment)),
    )
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    fn pull_if<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.tokens.next_if(p)
    }

    fn try_pull<P>(
        &mut self,
        p: P,
        expected: ExpectedToken,
    ) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.pull_if(p).ok_or_else(|| {
            ContextError::missing_or_unexpected(self.tokens.peek().copied(), self.source, expected)
        })
    }

    /// `expression -> or ;`
    fn expression(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.or()
    }

    /// `or -> xor ( ("|" | "!|") xor )* ;`
    fn or(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.xor()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Or | Punctuation::Nor
        ))) {
            let rhs = self.xor()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `xor -> and ( ("^" | "!^") and )* ;`
    fn xor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.and()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Xor | Punctuation::Xnor
        ))) {
            let rhs = self.and()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `and -> equality ( ("&" | "!&") equality )* ;`
    fn and(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.equality()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::And | Punctuation::Nand
        ))) {
            let rhs = self.equality()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )* ;`
    fn equality(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.comparison()?;
        while let Some(op) = self
            .tokens
            .next_if(match_token!(Punctuation(Punctuation::Ne | Punctuation::Eq)))
        {
            let rhs = self.comparison()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )* ;`
    fn comparison(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.shift()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Gt | Punctuation::Ge | Punctuation::Lt | Punctuation::Le
        ))) {
            let rhs = self.shift()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )* ;`
    fn shift(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.term()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Shl | Punctuation::Shr | Punctuation::Rotl | Punctuation::Rotr
        ))) {
            let rhs = self.term()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `term -> factor ( ( "+" | "-" ) factor )* ;`
    fn term(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.factor()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Add | Punctuation::SubNeg
        ))) {
            let rhs = self.factor()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `factor -> unary_postfix ( ( "*" | "/" | "%" ) unary_postfix )* ;`
    fn factor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_postfix()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Mul | Punctuation::Div | Punctuation::Rem
        ))) {
            let rhs = self.unary_postfix()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `unary_postfix -> unary_prefix ( "?" )* ;`
    fn unary_postfix(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_prefix()?;
        while let Some(op) = self
            .tokens
            .next_if(match_token!(Punctuation(Punctuation::Coalesce)))
        {
            expr = Expr::unary(Unary {
                operand: expr,
                op,
                side: OpSide::Left,
            });
        }
        Ok(expr)
    }

    /// `unary_prefix -> ( ( "!" | "!!" | "-" ) exponent )* ;`
    fn unary_prefix(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Not | Punctuation::Exists | Punctuation::SubNeg
        ))) {
            let operand = self.unary_prefix()?;
            Ok(Expr::unary(Unary {
                op,
                operand,
                side: OpSide::Right,
            }))
        } else {
            self.exponent()
        }
    }

    /// `exponent -> conversion ( "**" conversion )* ;`
    fn exponent(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.conversion()?;
        while let Some(op) = self
            .tokens
            .next_if(match_token!(Punctuation(Punctuation::Pow)))
        {
            let rhs = self.conversion()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `conversion -> primary ( "-:>" | "=:>" ) type_expression ;`
    fn conversion(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.primary()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Convert | Punctuation::Transmute
        ))) {
            let rhs = Expr::type_expr(self.type_expression()?);
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `primary -> literal | group ;`
    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.literal().or_else(|_| self.group()).map_err(|mut e| {
            if let ErrorType::MissingToken { expect } | ErrorType::UnexpectedToken { expect, .. } =
                &mut e.err
                && *expect == ExpectedToken::ParenExpr
            {
                *expect = ExpectedToken::Expr;
            }
            e
        })
    }

    /// `literal -> "true" | "fals" | "none" | UINT | SINT | FRAC | CHAR | TEXT ;`
    fn literal(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.try_pull(
            match_token!(
                BoolLiteral(_)
                    | Keyword(Keyword::None)
                    | UIntLiteral(_)
                    | SIntLiteral(_)
                    | FracLiteral(_)
                    | CharLiteral(_)
                    | TextLiteral(_)
            ),
            ExpectedToken::Literal,
        )
        .map(Expr::literal)
    }

    /// `group -> "(" expression ")" ;`
    fn group(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let open = self.try_pull(
            match_token!(Punctuation(Punctuation::LParen)),
            ExpectedToken::ParenExpr,
        )?;
        let expr = self.expression()?;
        self.try_pull(
            match_token!(Punctuation(Punctuation::RParen)),
            ExpectedToken::ExprOrRParen,
        )
        .map(move |close| Expr::grouping(Grouping { open, expr, close }))
        .map_err(|e| {
            e.map_type(|err| match err {
                ErrorType::MissingToken { .. } => ErrorType::MissingCloseBracket {
                    open_range: open.lex_range(self.source),
                    expect: Bracket::Paren,
                },

                ErrorType::UnexpectedToken {
                    actual: punc @ ("}" | "]"), .. // Sorry this doesn't use Value anymore, Token is huge now...
                } => ErrorType::IncorrectCloseBracket {
                    open_range: open.lex_range(self.source),
                    failure: match punc {
                        "}" => BadBracketCombo::ParenBrace,
                        "]" => BadBracketCombo::ParenBrack,
                        _ => unreachable!("guarded by outer match arm"),
                    },
                },

                err => err,
            })
        })
    }

    /// `type_expression -> ( "nevr" | "bool" | "uint" | "sint" | "frac" | "char" | "text" | "fail" | IDENTIFIER ) ( "|" "fail" )?;`
    fn type_expression(&mut self) -> Result<TypeExpr<'src>, ContextError<'src>> {
        let name = self.try_pull(
            |token| match token.val {
                LexValue::Identifier => true,
                LexValue::Keyword(kw) => kw.is_type(),
                _ => false,
            },
            ExpectedToken::TypeExpr,
        )?;
        let or_ty = self
            .tokens
            .next_if(match_token!(Punctuation(Punctuation::Or)))
            .map(|pipe| {
                Ok(OrType {
                    pipe,
                    ty: self.try_pull(
                        match_token!(Keyword(Keyword::Fail | Keyword::None | Keyword::Nevr)),
                        ExpectedToken::OrType,
                    )?,
                })
            });
        Ok(TypeExpr {
            name,
            or_ty: or_ty.transpose()?,
        })
    }

    fn synchronize(&mut self) {
        while let Some(token) = self.tokens.next() {
            // end of current statement
            if matches!(token.val, LexValue::Punctuation(Punctuation::Semi)) {
                break;
            }

            // start of new statement/definition
            if self.tokens.peek().is_some_and(|token| {
                matches!(
                    token.val,
                    LexValue::Keyword(
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
                            | Keyword::Pick
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
        let expr = parse(
            SOURCE,
            tokenize(SOURCE).map(|item| item.expect("this example should not have any lex errors")),
        )
        .collect::<Result<Vec<_>, _>>()
        .expect("this example should be a valid expression");
        assert_eq!(
            expr.as_slice(),
            &[Expr::binary(Binary {
                lhs: Expr::literal(Token {
                    lex: "5",
                    val: LexValue::SIntLiteral(5),
                    mac: None
                }),
                op: Token {
                    lex: "+",
                    val: LexValue::Punctuation(Punctuation::Add),
                    mac: None
                },
                rhs: Expr::binary(Binary {
                    lhs: Expr::unary(Unary {
                        side: OpSide::Right,
                        op: Token {
                            lex: "-",
                            val: LexValue::Punctuation(Punctuation::SubNeg),
                            mac: None
                        },
                        operand: Expr::grouping(Grouping {
                            open: Token {
                                lex: "(",
                                val: LexValue::Punctuation(Punctuation::LParen),
                                mac: None
                            },
                            expr: Expr::binary(Binary {
                                lhs: Expr::literal(Token {
                                    lex: "7",
                                    val: LexValue::SIntLiteral(7),
                                    mac: None
                                }),
                                op: Token {
                                    lex: "/",
                                    val: LexValue::Punctuation(Punctuation::Div),
                                    mac: None
                                },
                                rhs: Expr::literal(Token {
                                    lex: "8",
                                    val: LexValue::SIntLiteral(8),
                                    mac: None
                                })
                            }),
                            close: Token {
                                lex: ")",
                                val: LexValue::Punctuation(Punctuation::RParen),
                                mac: None
                            }
                        })
                    }),
                    op: Token {
                        lex: "*",
                        val: LexValue::Punctuation(Punctuation::Mul),
                        mac: None
                    },
                    rhs: Expr::Literal(Token {
                        lex: "3",
                        val: LexValue::SIntLiteral(3),
                        mac: None
                    })
                })
            })]
        );
    }
}
