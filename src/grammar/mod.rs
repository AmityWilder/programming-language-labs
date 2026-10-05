//! Context-free grammar

use crate::{
    error::{ContextError, ErrorType, ExpectedToken},
    highlight::{Highlighted, style::StyleWrapper, syntax::Syntax},
    scanner::{
        BadBracketCombo, Bracket,
        token::{Token, keyword::Keyword, punc::Punctuation, value::LexValue},
    },
};
use std::range::Range;

/// Means of displaying content with lisp style
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

/// Means of displaying content with Polish notation
pub trait PolishDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

/// Adapter to display contents as Polish
#[derive(Debug)]
#[repr(transparent)]
pub struct Polish<T: ?Sized + PolishDisplay>(T);

impl<T: ?Sized + PolishDisplay> Polish<T> {
    pub const fn new(value: &T) -> &Self {
        // SAFETY: Math is a transparent wrapper for `T`.
        unsafe { std::mem::transmute(value) }
    }
}

impl<T: ?Sized + PolishDisplay> std::fmt::Display for Polish<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        PolishDisplay::fmt(&self.0, f) // calls MathDisplay::fmt, since T isn't proven to implement any other fmt
    }
}

macro_rules! match_token {
    ($($variant:ident$(($pattern:pat))?)|+) => {
        |token| matches!(token.val, $($crate::scanner::token::value::LexValue::$variant$(($pattern))?)|+)
    };
}
pub(crate) use match_token;

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
        if f.alternate() {
            write!(
                f,
                "({} {:#} {:#})",
                crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(self.op.lex),
                Lisp::new(&self.lhs),
                Lisp::new(&self.rhs)
            )
        } else {
            write!(
                f,
                "({} {} {})",
                self.op.lex,
                Lisp::new(&self.lhs),
                Lisp::new(&self.rhs)
            )
        }
    }
}

impl PolishDisplay for Binary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.op.lex,
            Polish::new(&self.lhs),
            Polish::new(&self.rhs)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<'src> {
    /// Promises to use [`Punctuation::Neg`] instead of [`Punctuation::Sub`]
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
        if f.alternate() {
            write!(
                f,
                "({} {:#})",
                crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(self.op.lex),
                Lisp::new(&self.rhs)
            )
        } else {
            write!(f, "({} {})", self.op.lex, Lisp::new(&self.rhs))
        }
    }
}

impl PolishDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self.op.val {
            // TODO: how does polish notation represent unary negative?
            LexValue::Punctuation(Punctuation::SubNeg) => "- 0",
            _ => self.op.lex,
        };
        write!(f, "{op} {}", Polish::new(&self.rhs))
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
        if f.alternate() {
            let kw = crate::SYNTAX_STYLE_ANSI[Syntax::Keyword];
            write!(f, "({}group{} {:#})", kw.begin(), kw.end(), Lisp::new(expr))
        } else {
            write!(f, "(group {})", Lisp::new(expr))
        }
    }
}

impl PolishDisplay for Grouping<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { expr, .. } = self;
        write!(f, "{}", Polish::new(expr))
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
            Self::Literal(tkn @ Token { lex, .. }) => {
                if f.alternate() {
                    std::fmt::Display::fmt(&Highlighted(std::iter::once(&Ok(*tkn))), f)
                } else {
                    f.write_str(lex)
                }
            } // TODO: should this use val instead of lex?
            Self::Grouping(inner) => LispDisplay::fmt(&**inner, f),
        }
    }
}

impl PolishDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => PolishDisplay::fmt(&**inner, f),
            Self::Unary(inner) => PolishDisplay::fmt(&**inner, f),
            Self::Literal(Token { lex, .. }) => f.write_str(lex), // TODO: should this use val instead of lex?
            Self::Grouping(inner) => PolishDisplay::fmt(&**inner, f),
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

    /// `factor -> unary ( ( "*" | "/" | "%" ) unary )* ;`
    fn factor(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.unary()?;
        while let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Mul | Punctuation::Div | Punctuation::Rem
        ))) {
            let rhs = self.unary()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

    /// `unary -> ( ( "!" | "-" ) exponent )* ;`
    fn unary(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        if let Some(op) = self.tokens.next_if(match_token!(Punctuation(
            Punctuation::Not | Punctuation::SubNeg
        ))) {
            let rhs = self.unary()?;
            Ok(Expr::unary(Unary { op, rhs }))
        } else {
            self.exponent()
        }
    }

    /// `exponent -> primary ( "**" primary )* ;`
    fn exponent(&mut self) -> Result<Expr<'src>, ContextError<'src>> {
        let mut expr = self.primary()?;
        while let Some(op) = self
            .tokens
            .next_if(match_token!(Punctuation(Punctuation::Pow)))
        {
            let rhs = self.primary()?;
            expr = Expr::binary(Binary { lhs: expr, op, rhs });
        }
        Ok(expr)
    }

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

    /// `literal -> "true" | "fals" | "none" | UINT | SINT | FRAC | CHAR | STRING ;`
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

pub enum ExprOrToken<'src, 'expr> {
    Token(&'expr Token<'src>),
    Expr(&'expr Expr<'src>),
}

pub enum ExprIter<'src, 'expr> {
    Binary {
        lhs: Option<&'expr Expr<'src>>,
        op: Option<&'expr Token<'src>>,
        rhs: Option<&'expr Expr<'src>>,
    },
    Unary {
        op: Option<&'expr Token<'src>>,
        rhs: Option<&'expr Expr<'src>>,
    },
    Literal {
        token: Option<&'expr Token<'src>>,
    },
    Grouping {
        open: Option<&'expr Token<'src>>,
        expr: Option<&'expr Expr<'src>>,
        close: Option<&'expr Token<'src>>,
    },
}

impl<'src, 'expr> Iterator for ExprIter<'src, 'expr> {
    type Item = ExprOrToken<'src, 'expr>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ExprIter::Binary { lhs, op, rhs } => lhs
                .take()
                .map(ExprOrToken::Expr)
                .or_else(|| op.take().map(ExprOrToken::Token))
                .or_else(|| rhs.take().map(ExprOrToken::Expr)),

            ExprIter::Unary { op, rhs } => op
                .take()
                .map(ExprOrToken::Token)
                .or_else(|| rhs.take().map(ExprOrToken::Expr)),

            ExprIter::Literal { token } => token.take().map(ExprOrToken::Token),

            ExprIter::Grouping { open, expr, close } => open
                .take()
                .map(ExprOrToken::Token)
                .or_else(|| expr.take().map(ExprOrToken::Expr))
                .or_else(|| close.take().map(ExprOrToken::Token)),
        }
    }
}

impl<'src> Expr<'src> {
    fn iter(&self) -> ExprIter<'src, '_> {
        match self {
            Expr::Binary(Binary { lhs, op, rhs }) => ExprIter::Binary {
                lhs: Some(lhs),
                op: Some(op),
                rhs: Some(rhs),
            },

            Expr::Unary(Unary { op, rhs }) => ExprIter::Unary {
                op: Some(op),
                rhs: Some(rhs),
            },

            Expr::Literal(token) => ExprIter::Literal { token: Some(token) },

            Expr::Grouping(Grouping { open, expr, close }) => ExprIter::Grouping {
                open: Some(open),
                expr: Some(expr),
                close: Some(close),
            },
        }
    }
}

/// Traverse the AST using DFS
pub struct AstIter<'src, 'expr> {
    stack: Vec<ExprIter<'src, 'expr>>,
}

impl<'src, 'expr> AstIter<'src, 'expr> {
    pub fn new(ast: &'expr Expr<'src>) -> Self {
        Self {
            stack: vec![ast.iter()],
        }
    }
}

impl<'src, 'expr> Iterator for AstIter<'src, 'expr> {
    type Item = &'expr Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.last_mut() {
            if let Some(item) = top.next() {
                match item {
                    ExprOrToken::Token(token) => return Some(token),
                    ExprOrToken::Expr(expr) => self.stack.push(expr.iter()),
                }
            } else {
                self.stack.pop();
            }
        }
        None
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
                    val: LexValue::UIntLiteral(5),
                    mac: None
                }),
                op: Token {
                    lex: "+",
                    val: LexValue::Punctuation(Punctuation::Add),
                    mac: None
                },
                rhs: Expr::binary(Binary {
                    lhs: Expr::unary(Unary {
                        op: Token {
                            lex: "-",
                            val: LexValue::Punctuation(Punctuation::SubNeg),
                            mac: None
                        },
                        rhs: Expr::grouping(Grouping {
                            open: Token {
                                lex: "(",
                                val: LexValue::Punctuation(Punctuation::LParen),
                                mac: None
                            },
                            expr: Expr::binary(Binary {
                                lhs: Expr::literal(Token {
                                    lex: "7",
                                    val: LexValue::UIntLiteral(7),
                                    mac: None
                                }),
                                op: Token {
                                    lex: "/",
                                    val: LexValue::Punctuation(Punctuation::Div),
                                    mac: None
                                },
                                rhs: Expr::literal(Token {
                                    lex: "8",
                                    val: LexValue::UIntLiteral(8),
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
                        val: LexValue::UIntLiteral(3),
                        mac: None
                    })
                })
            })]
        );
    }
}
