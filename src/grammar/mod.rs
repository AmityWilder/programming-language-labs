//! Context-free grammar

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::{ContextError, ErrorType},
    scanner::{
        Bracket,
        token::{Token, keyword::Keyword, punc::Punctuation, value::Value},
    },
};
use std::range::Range;

// Means of displaying content with lisp style
pub trait LispDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

/// Adapter to display contents as lisp
#[derive(Debug)]
#[repr(transparent)]
pub struct Lisp<T: ?Sized + LispDisplay>(T);

impl<T: ?Sized + LispDisplay> Lisp<T> {
    pub const fn new(value: &T) -> &Self {
        // SAFETY: Lisp is a transparent wrapper for `T`.
        unsafe { std::mem::transmute(value) }
    }
}

impl<T: ?Sized + LispDisplay> std::fmt::Display for Lisp<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LispDisplay::fmt(&self.0, f) // calls LispDisplay::fmt, since T isn't proven to implement any other fmt
    }
}

// Means of displaying content with math style
pub trait MathDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

/// Adapter to display contents as math
#[derive(Debug)]
#[repr(transparent)]
pub struct Math<T: ?Sized + MathDisplay>(T);

impl<T: ?Sized + MathDisplay> Math<T> {
    pub const fn new(value: &T) -> &Self {
        // SAFETY: Math is a transparent wrapper for `T`.
        unsafe { std::mem::transmute(value) }
    }
}

impl<T: ?Sized + MathDisplay> std::fmt::Display for Math<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MathDisplay::fmt(&self.0, f) // calls MathDisplay::fmt, since T isn't proven to implement any other fmt
    }
}

macro_rules! match_token {
    ($($variant:ident$(($pattern:pat))?)|+) => {
        |token| matches!(token.val, $($crate::scanner::token::value::Value::$variant$(($pattern))?)|+)
    };
}
pub(crate) use match_token;

macro_rules! bnf_notation {
    // direct
    ($head:ident -> $inner:ident) => {
        /// Direct
        ///
        #[doc = concat!("`", stringify!($head -> $inner ;), "`")]
        fn $head(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            self.$inner()
        }
    };

    // binary
    ($head:ident -> $lhs:ident ( ( $($op:ident)|+ ) $rhs:ident )*) => {
        /// Binary
        ///
        #[doc = concat!("`", stringify!($head -> $lhs ( ( $($op)|+ ) $rhs )* ;), "`")]
        fn $head(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            let mut expr = self.$lhs()?;
            // left associative
            while let Some(op) = self.tokens.next_if(match_token!(Punctuation($(Punctuation::$op)|+))) {
                let rhs = self.$rhs()?;
                expr = Expr::binary(Binary { lhs: expr, op, rhs });
            }
            Ok(expr)
        }
    };

    // unary
    ($head:ident -> ( ( $($op:ident)|+ ) $rhs:ident )*) => {
        /// Unary
        ///
        #[doc = concat!("`", stringify!($head -> ( ( $($op)|+ ) $rhs )* ;), "`")]
        fn $head(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            // right associative
            if let Some(op) = self.tokens.next_if(match_token!(Punctuation($(Punctuation::$op)|+))) {
                let rhs = self.$head()?;
                Ok(Expr::unary(Unary { op, rhs }))
            } else {
                self.$rhs()
            }
        }
    };

    // literal
    ($head:ident -> ( ( $($variant:ident$(( $pattern:pat ))?)|+ ) ) $(*)?) => {
        /// Literal
        ///
        #[doc = concat!("`", stringify!($head -> $($variant$(($pattern))?)|+ ;), "`")]
        fn $head(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
            self.try_pull(match_token!($($variant$(($pattern))?)|+), "a literal").map(Expr::literal)
        }
    };
}

macro_rules! bnf_notation_multi {
    ($( $head:ident -> $($lhs_or_inner:ident)? $(( ( $($variant_or_op:ident$(($pattern:pat))?)|+ ) $($rhs:ident)? )$(*)?)? ; )*) => {$(
        bnf_notation! { $head -> $($lhs_or_inner)? $(( ( $($variant_or_op$(($pattern))?)|+ ) $($rhs)? )*)? }
    )*};
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary<'src> {
    pub lhs: Expr<'src>,
    pub op: Token<'src>,
    pub rhs: Expr<'src>,
}

impl<'src> Binary<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        Range {
            start: self.lhs.range(source).start,
            end: self.rhs.range(source).end,
        }
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    pub fn macro_range(&self, source: &'src str) -> Option<Range<usize>> {
        let lhs_mac = self.lhs.macro_range(source)?;
        // don't need to check op because it's between them, so it must be in the same expansion if the other two are
        let rhs_mac = self.rhs.macro_range(source)?;
        (lhs_mac == rhs_mac).then_some(lhs_mac)
    }
}

impl std::fmt::Display for Binary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            lhs,
            op: Token { lex: op, .. },
            rhs,
        } = self;
        write!(f, "{lhs} {op} {rhs}")
    }
}

impl LispDisplay for Binary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({} {} {})",
            self.op.lex,
            Lisp::new(&self.lhs),
            Lisp::new(&self.rhs)
        )
    }
}

impl MathDisplay for Binary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sep = match self.op.val {
            Value::Punctuation(
                Punctuation::Rem | Punctuation::Mul | Punctuation::Div | Punctuation::Pow,
            ) => "",
            _ => " ",
        };
        write!(
            f,
            "({}{sep}{}{sep}{})",
            Math::new(&self.lhs),
            self.op.lex,
            Math::new(&self.rhs)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<'src> {
    pub op: Token<'src>,
    pub rhs: Expr<'src>,
}

impl<'src> Unary<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        Range {
            start: self.op.lex_range(source).start,
            end: self.rhs.range(source).end,
        }
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    pub fn macro_range(&self, source: &'src str) -> Option<Range<usize>> {
        let op_mac = self.op.mac?;
        let rhs_mac = self.rhs.macro_range(source)?;
        (op_mac == rhs_mac).then_some(op_mac)
    }
}

impl std::fmt::Display for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            op: Token { lex: op, .. },
            rhs,
        } = self;
        write!(f, "{op}{rhs}")
    }
}

impl LispDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {})", self.op.lex, Lisp::new(&self.rhs))
    }
}

impl MathDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sep = match self.op.val {
            Value::Punctuation(Punctuation::Sub /* negate */ | Punctuation::Not | Punctuation::MacroStringify) => "",
            _ => " ",
        };
        write!(f, "({}{sep}{})", self.op.lex, Math::new(&self.rhs))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grouping<'src> {
    pub open: Token<'src>,
    pub expr: Expr<'src>,
    pub close: Token<'src>,
}

impl<'src> Grouping<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        Range {
            start: self.open.lex_range(source).start,
            end: self.close.lex_range(source).end,
        }
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    pub fn macro_range(&self, _: &'src str) -> Option<Range<usize>> {
        let open_mac = self.open.mac?;
        // don't need to check expr because it's between open and close,
        // and therefore must be in the same expansion if the other two are.
        // this is also cheaper, since now we don't have to recursively check the inner expressions :)
        let close_mac = self.close.mac?;
        (open_mac == close_mac).then_some(open_mac)
    }
}

impl std::fmt::Display for Grouping<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            open: Token { lex: open, .. },
            expr,
            close: Token { lex: close, .. },
        } = self;
        write!(f, "{open}{expr}{close}")
    }
}

impl LispDisplay for Grouping<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { expr, .. } = self;
        write!(f, "(group {})", Lisp::new(expr))
    }
}

impl MathDisplay for Grouping<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { expr, .. } = self;
        write!(f, "{}", Math::new(expr)) // contents will be parenthesized anyway
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Binary(Box<Binary<'src>>),
    Unary(Box<Unary<'src>>),
    Literal(Token<'src>),
    Grouping(Box<Grouping<'src>>),
}

impl std::fmt::Display for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => inner.fmt(f),
            Self::Unary(inner) => inner.fmt(f),
            Self::Literal(Token { lex, .. }) => lex.fmt(f),
            Self::Grouping(inner) => inner.fmt(f),
        }
    }
}

impl LispDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => LispDisplay::fmt(&**inner, f),
            Self::Unary(inner) => LispDisplay::fmt(&**inner, f),
            Self::Literal(Token { lex, .. }) => f.write_str(lex), // TODO: should this use val instead of lex?
            Self::Grouping(inner) => LispDisplay::fmt(&**inner, f),
        }
    }
}

impl MathDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => MathDisplay::fmt(&**inner, f),
            Self::Unary(inner) => MathDisplay::fmt(&**inner, f),
            Self::Literal(Token { lex, .. }) => f.write_str(lex), // TODO: should this use val instead of lex?
            Self::Grouping(inner) => MathDisplay::fmt(&**inner, f),
        }
    }
}

impl<'src> Expr<'src> {
    pub fn range(&self, source: &'src str) -> Range<usize> {
        match self {
            Self::Binary(binary) => binary.range(source),
            Self::Unary(unary) => unary.range(source),
            Self::Literal(token) => token.lex_range(source),
            Self::Grouping(group) => group.range(source),
        }
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    // TODO: what if part of it is from a nested macro?
    pub fn macro_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            Expr::Binary(binary) => binary.macro_range(source),
            Expr::Unary(unary) => unary.macro_range(source),
            Expr::Literal(token) => token.mac,
            Expr::Grouping(grouping) => grouping.macro_range(source),
        }
    }

    pub fn binary(inner: Binary<'src>) -> Self {
        Self::Binary(Box::new(inner))
    }

    pub fn unary(inner: Unary<'src>) -> Self {
        Self::Unary(Box::new(inner))
    }

    pub const fn literal(literal: Token<'src>) -> Self {
        Self::Literal(literal)
    }

    pub fn grouping(inner: Grouping<'src>) -> Self {
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

    fn try_pull<P>(
        &mut self,
        p: P,
        expected: &'static str,
    ) -> Result<Token<'src>, ContextError<'src>>
    where
        P: FnOnce(&Token<'src>) -> bool,
    {
        self.pull_if(p).ok_or_else(|| {
            ContextError::missing_or_unexpected(self.tokens.peek().copied(), self.source, expected)
        })
    }

    bnf_notation_multi! {
        expression -> equality ;
        equality   -> comparison ( (Ne | Eq) comparison )* ;
        comparison -> shift ( (Gt | Ge | Lt | Le) shift )* ;
        shift      -> term ( (Shl | Shr) term )* ;
        term       -> factor ( (Add | Sub) factor )* ;
        factor     -> unary ( (Mul | Div | Rem) unary )* ;
        unary      -> ((Not | Sub) exponent)* ;
        exponent   -> primary ( (Pow) primary )* ; // TODO: exponents should be greater precedence than unary!!
        literal    -> ((
            BoolLiteral(_)
            | Keyword(Keyword::None)
            | UIntLiteral(_)
            | SIntLiteral(_)
            | FltLiteral(_)
            | CharLiteral(_)
            | StringLiteral(_)
        )) ;
    }

    fn group(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let open = self.try_pull(
            match_token!(Punctuation(Punctuation::LParen)),
            "a parenthesized expression",
        )?;
        let expr = self.expression()?;
        self.try_pull(
            match_token!(Punctuation(Punctuation::RParen)),
            "an expression or `)`",
        )
        .map(move |close| Expr::grouping(Grouping { open, expr, close }))
        .map_err(|e| {
            e.map_type(|err| match err {
                ErrorType::MissingToken { .. } => ErrorType::MissingCloseBracket {
                    expect: (Bracket::Paren, open.lex_range(self.source)),
                },

                ErrorType::UnexpectedToken {
                    actual: punc @ ("}" | "]"), .. // Sorry this doesn't use Value anymore, Token is huge now...
                } => ErrorType::IncorrectCloseBracket {
                    expect: (Bracket::Paren, open.lex_range(self.source)),
                    actual: match punc {
                        "}" => Bracket::Brace,
                        "]" => Bracket::Brack,
                        _ => unreachable!("guarded by outer match arm"),
                    },
                },

                err => err,
            })
        })
    }

    fn primary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        self.literal().or_else(|_| self.group()).map_err(|mut e| {
            if let ErrorType::MissingToken { expect } | ErrorType::UnexpectedToken { expect, .. } =
                &mut e.err
                && *expect == "parenthesized expression"
            {
                *expect = "an expression";
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
                    val: Value::UIntLiteral(5),
                    mac: None
                }),
                op: Token {
                    lex: "+",
                    val: Value::Punctuation(Punctuation::Add),
                    mac: None
                },
                rhs: Expr::binary(Binary {
                    lhs: Expr::unary(Unary {
                        op: Token {
                            lex: "-",
                            val: Value::Punctuation(Punctuation::Sub),
                            mac: None
                        },
                        rhs: Expr::grouping(Grouping {
                            open: Token {
                                lex: "(",
                                val: Value::Punctuation(Punctuation::LParen),
                                mac: None
                            },
                            expr: Expr::binary(Binary {
                                lhs: Expr::literal(Token {
                                    lex: "7",
                                    val: Value::UIntLiteral(7),
                                    mac: None
                                }),
                                op: Token {
                                    lex: "/",
                                    val: Value::Punctuation(Punctuation::Div),
                                    mac: None
                                },
                                rhs: Expr::literal(Token {
                                    lex: "8",
                                    val: Value::UIntLiteral(8),
                                    mac: None
                                })
                            }),
                            close: Token {
                                lex: ")",
                                val: Value::Punctuation(Punctuation::RParen),
                                mac: None
                            }
                        })
                    }),
                    op: Token {
                        lex: "*",
                        val: Value::Punctuation(Punctuation::Mul),
                        mac: None
                    },
                    rhs: Expr::Literal(Token {
                        lex: "3",
                        val: Value::UIntLiteral(3),
                        mac: None
                    })
                })
            })]
        );
    }
}
