use crate::{error::OpSide, scanner::token::Token};
use std::range::Range;

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

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<'src> {
    /// Promises to use [`Punctuation::Neg`] instead of [`Punctuation::Sub`]
    pub op: Token<'src>,
    pub operand: Expr<'src>,
    pub side: OpSide,
}

impl<'src> Unary<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        let op_range = self.op.lex_range(source);
        let operand_range = self.operand.range(source);
        match self.side {
            OpSide::Left => Range {
                start: operand_range.start,
                end: op_range.end,
            },
            OpSide::Right => Range {
                start: op_range.start,
                end: operand_range.end,
            },
        }
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    pub fn macro_range(&self, source: &'src str) -> Option<Range<usize>> {
        let op_mac = self.op.mac?;
        let rhs_mac = self.operand.macro_range(source)?;
        (op_mac == rhs_mac).then_some(op_mac)
    }
}

impl std::fmt::Display for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            side,
            op: Token { lex: op, .. },
            operand,
        } = self;
        match side {
            OpSide::Left => write!(f, "{operand}{op}"),
            OpSide::Right => write!(f, "{op}{operand}"),
        }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrType<'src> {
    // `|`
    pub pipe: Token<'src>,
    // `none`, `fail`, or `nevr`
    pub ty: Token<'src>,
}

/// A concrete type
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeExpr<'src> {
    pub name: Token<'src>,
    pub or_ty: Option<OrType<'src>>,
    // TODO: namespace?
    // TODO: generic arguments?
}

impl<'src> TypeExpr<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        self.name.lex_range(source)
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    pub const fn macro_range(&self, _: &'src str) -> Option<Range<usize>> {
        self.name.mac
    }
}

impl std::fmt::Display for TypeExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            name: Token { lex: name, .. },
            or_ty,
        } = self;
        if let Some(OrType {
            pipe: Token { lex: pipe, .. },
            ty: Token { lex: ty, .. },
        }) = or_ty
        {
            write!(f, "{name} {pipe} {ty}")
        } else {
            write!(f, "{name}")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Binary(Box<Binary<'src>>),
    Unary(Box<Unary<'src>>),
    Literal(Token<'src>),
    Grouping(Box<Grouping<'src>>),
    Type(TypeExpr<'src>),
}

impl std::fmt::Display for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => inner.fmt(f),
            Self::Unary(inner) => inner.fmt(f),
            Self::Literal(Token { lex, .. }) => lex.fmt(f),
            Self::Grouping(inner) => inner.fmt(f),
            Self::Type(inner) => std::fmt::Display::fmt(inner, f),
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
            Self::Type(ty) => ty.range(source),
        }
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    // TBD: what if part of it is from a nested macro?
    pub fn macro_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            Expr::Binary(binary) => binary.macro_range(source),
            Expr::Unary(unary) => unary.macro_range(source),
            Expr::Literal(token) => token.mac,
            Expr::Grouping(grouping) => grouping.macro_range(source),
            Expr::Type(ty) => ty.macro_range(source),
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

    pub const fn type_expr(inner: TypeExpr<'src>) -> Self {
        Self::Type(inner)
    }
}
