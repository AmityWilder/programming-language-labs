//! Context-free grammar

use crate::{
    error::{ContextError, ErrorType, ExpectedToken, OpSide},
    scanner::{
        Bracket,
        token::{Token, value::LexValue},
    },
};
use ast::{Binary, Expr, Grouping, OrType, TypeExpr, Unary};

pub mod ast;
pub mod ast_iter;
pub mod fmt;

#[cfg(test)]
pub mod tests;

macro_rules! token_matches {
    ($token:expr, $($Variant:ident$(($pattern:pat))?$(::$SubVariant:ident)?$(::{$($SubVariants:ident)|*})?)|+) => {{
        #[allow(unused_imports, reason = "only here to allow proper semantic highlighting")] {
            $($({ use $crate::scanner::token::value::$Variant::$SubVariant; })?)*
            $($({ use $crate::scanner::token::value::$Variant::{$($SubVariants),*}; })?)*
        }
        matches!($token.val, $($crate::scanner::token::value::LexValue::$Variant
            $(($pattern))?
            $(($crate::scanner::token::value::$Variant::$SubVariant))?
            $(($($crate::scanner::token::value::$Variant::$SubVariants)|*))?
        )|+)
    }};
}
pub(crate) use token_matches;

macro_rules! match_token {
    ($($Variant:ident$(($pattern:pat))?$(::$SubVariant:ident)?$(::{$($SubVariants:ident)|*})?)|+) => {
        |token| -> bool { $crate::grammar::token_matches!(token, $($Variant$(($pattern))?$(::$SubVariant)?$(::{$($SubVariants)|*})?)|+) }
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
            .filter(|token| !token_matches!(token, Whitespace | Comment)),
    )
}

impl<'src, I: Iterator<Item = Token<'src>>> Parser<'src, I> {
    /// Optional token
    fn pull_if<P>(&mut self, p: P) -> Option<Token<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.tokens.next_if(p)
    }

    /// Required token
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

    /// `expression -> or`
    pub fn expression(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.or()
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.xor()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Or | Nor})) {
            let rhs = self.xor()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.and()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Xor | Xnor})) {
            let rhs = self.and()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.equality()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{And | Nand})) {
            let rhs = self.equality()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.comparison()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Ne | Eq})) {
            let rhs = self.comparison()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.shift()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Gt | Ge | Lt | Le})) {
            let rhs = self.shift()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.term()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Shl | Shr | Rotl | Rotr})) {
            let rhs = self.term()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.factor()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Add | SubNeg})) {
            let rhs = self.factor()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `factor -> unary_postfix ( ( "*" | "/" | "%" ) unary_postfix )*`
    fn factor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_postfix()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Mul | Div | Rem})) {
            let rhs = self.unary_postfix()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `unary_postfix -> unary_prefix ( "?" )*`
    fn unary_postfix(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary_prefix()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::Coalesce)) {
            expr = Expr::unary(Unary {
                operand: expr,
                op,
                side: OpSide::Left,
            });
        }
        Ok(expr)
    }

    /// `unary_prefix -> ( ( ( "!" | "!!" | "-" ) | "szof" ( type_expression )? ) )* exponent`
    fn unary_prefix(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(op) = self.pull_if(match_token!(Keyword::Szof)) {
            let operand = self
                .type_expression()
                .map(Expr::type_expr)
                .or_else(|_| self.unary_prefix())?;
            Ok(Expr::unary(Unary {
                op,
                operand,
                side: OpSide::Right,
            }))
        } else if let Some(op) = self.pull_if(match_token!(Punctuation::{Not | Exists | SubNeg})) {
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

    /// `exponent -> conversion ( "**" conversion )*`
    fn exponent(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.conversion()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::Pow)) {
            let rhs = self.conversion()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `conversion -> primary ( "-:>" | "=:>" ) type_expression`
    fn conversion(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.primary()?;
        while let Some(op) = self.pull_if(match_token!(Punctuation::{Convert | Transmute})) {
            let rhs = Expr::type_expr(self.type_expression()?);
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `primary -> literal | group`
    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.literal()
            .or_else(|_| self.group().map(Expr::grouping))
            .map_err(|mut e| {
                if let ErrorType::MissingToken { expect }
                | ErrorType::UnexpectedToken { expect, .. } = &mut e.err
                    && *expect == ExpectedToken::ParenExpr
                {
                    *expect = ExpectedToken::Expr;
                }
                e
            })
    }

    /// `literal -> "true" | "fals" | "none" | UINT | SINT | FRAC | CHAR | TEXT | MACRO`
    fn literal(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.try_pull(
            match_token!(
                BoolLiteral(_)
                    | Keyword::None
                    | UIntLiteral(_)
                    | SIntLiteral(_)
                    | FracLiteral(_)
                    | CharLiteral(_)
                    | TextLiteral(_)
                    | Macro
            ),
            ExpectedToken::Literal,
        )
        .map(Expr::literal)
    }

    /// `group -> "(" expression ")"`
    fn group(&mut self) -> Result<Grouping<'src>, ContextError<'src>> {
        let open = self.try_pull(match_token!(Punctuation::LParen), ExpectedToken::ParenExpr)?;
        let expr = self.expression()?;
        self.try_pull(
            match_token!(Punctuation::RParen),
            ExpectedToken::ExprOrRParen,
        )
        .map(move |close| Grouping { open, expr, close })
        .map_err(|e| e.to_bracket_mismatch(Bracket::Paren, open.lex_range(self.source)))
    }

    /// `type_expression -> ( "nevr" | "bool" | "uint" | "sint" | "frac" | "char" | "text" | "fail" | IDENTIFIER ) ( "|" ( "fail" | "none" | "nevr" ) )?`
    fn type_expression(&mut self) -> Result<TypeExpr<'src>, ContextError<'src>> {
        let name = self.try_pull(
            |token| match token.val {
                LexValue::Identifier => true,
                LexValue::Keyword(kw) => kw.is_type(),
                _ => false,
            },
            ExpectedToken::TypeExpr,
        )?;
        let or_ty = self.pull_if(match_token!(Punctuation::Or)).map(|pipe| {
            Ok(OrType {
                pipe,
                ty: self.try_pull(
                    match_token!(Keyword::{Fail | None | Nevr}),
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
            if token_matches!(token, Punctuation::Semi) {
                break;
            }

            // start of new statement/definition
            if self.tokens.peek().is_some_and(match_token!(Keyword::{
                Rec | Sup | Cat | Alt | Sub | Def | Fun | Mem | Let | Uni | Pvt | If |
                Or | Pick | Rep | For | Loop | Cord | Halt | Skip | Give | Fail | Emit
            })) {
                break;
            }
        }
    }
}
