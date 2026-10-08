use crate::{
    error::OpSide,
    scanner::token::{ExpansionData, Token},
};
use std::range::Range;

pub trait AstNode<'src> {
    fn range(&self, source: &'src str) -> Range<usize>;

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    // TBD: what if part of it is from a nested macro?
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>>;
}

impl<'src, T> AstNode<'src> for &T
where
    T: AstNode<'src>,
{
    fn range(&self, source: &'src str) -> Range<usize> {
        (*self).range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (*self).expansion(source)
    }
}

impl<'src, T, U> AstNode<'src> for (T, U)
where
    T: AstNode<'src>,
    U: AstNode<'src>,
{
    fn range(&self, source: &'src str) -> Range<usize> {
        Range {
            start: self.0.range(source).start,
            end: self.1.range(source).end,
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        let exp = (self.0.expansion(source)?, self.1.expansion(source)?);
        (exp.0 == exp.1).then(|| {
            if exp.0.arg == exp.1.arg {
                exp.0
            } else {
                ExpansionData {
                    range: exp.0.range,
                    arg: None,
                }
            }
        })
    }
}

impl<'src> AstNode<'src> for Token<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        self.lex_range(source)
    }

    fn expansion(&self, _: &'src str) -> Option<ExpansionData<'src>> {
        self.mac
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary<'src> {
    pub lhs: Expr<'src>,
    pub op: Token<'src>,
    pub rhs: Expr<'src>,
}

impl<'src> AstNode<'src> for Binary<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        (&self.lhs, &self.rhs).range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check op because it's between them, so it must be in the same expansion if the other two are
        (&self.lhs, &self.rhs).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<'src> {
    /// Promises to use [`Punctuation::Neg`] instead of [`Punctuation::Sub`]
    pub op: Token<'src>,
    pub operand: Expr<'src>,
    pub side: OpSide,
}

impl<'src> AstNode<'src> for Unary<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        match self.side {
            OpSide::Left => (&self.operand, &self.op).range(source),
            OpSide::Right => (&self.op, &self.operand).range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self.side {
            OpSide::Left => (&self.operand, &self.op).expansion(source),
            OpSide::Right => (&self.op, &self.operand).expansion(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grouping<'src> {
    pub open: Token<'src>,
    pub expr: Expr<'src>,
    pub close: Token<'src>,
}

impl<'src> AstNode<'src> for Grouping<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        (&self.open, &self.close).range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check expr because it's between open and close,
        // and therefore must be in the same expansion if the other two are.
        // this is also cheaper, since now we don't have to recursively check the inner expressions :)
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrType<'src> {
    // `|`
    pub pipe: Token<'src>,
    // `none`, `fail`, or `nevr`
    pub ty: Token<'src>,
}

impl<'src> AstNode<'src> for OrType<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        (&self.pipe, &self.ty).range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.pipe, &self.ty).expansion(source)
    }
}

/// A concrete type
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr<'src> {
    pub name: Token<'src>,
    pub or_ty: Option<OrType<'src>>,
    // TODO: namespace?
    // TODO: generic arguments?
}

impl<'src> AstNode<'src> for TypeExpr<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        if let Some(or_ty) = &self.or_ty {
            (self.name, or_ty).range(source)
        } else {
            self.name.range(source)
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        if let Some(or_ty) = &self.or_ty {
            (self.name, or_ty).expansion(source)
        } else {
            self.name.expansion(source)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FnSource<'src> {
    Ident(Token<'src>),
    Group(Grouping<'src>),
}

impl<'src> AstNode<'src> for FnSource<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        match self {
            Self::Ident(inner) => inner.range(source),
            Self::Group(inner) => inner.range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self {
            Self::Ident(inner) => inner.expansion(source),
            Self::Group(inner) => inner.expansion(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArgList1<'src> {
    pub comma: Token<'src>,
    pub arg: Expr<'src>,
    pub rest: Option<Box<ArgList1<'src>>>,
}

impl<'src> AstNode<'src> for ArgList1<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        if let Some(rest) = &self.rest {
            (&self.comma, &**rest).range(source)
        } else {
            (&self.comma, &self.arg).range(source)
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        if let Some(rest) = &self.rest {
            (&self.comma, &**rest).expansion(source)
        } else {
            (&self.comma, &self.arg).expansion(source)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArgList<'src> {
    pub first: Expr<'src>,
    pub rest: Option<ArgList1<'src>>,
    pub trailing_comma: Option<Token<'src>>,
}

impl<'src> AstNode<'src> for ArgList<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        if let Some(trailing_comma) = &self.trailing_comma {
            (&self.first, trailing_comma).range(source)
        } else if let Some(rest) = &self.rest {
            (&self.first, rest).range(source)
        } else {
            self.first.range(source)
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        if let Some(trailing_comma) = &self.trailing_comma {
            (&self.first, trailing_comma).expansion(source)
        } else if let Some(rest) = &self.rest {
            (&self.first, rest).expansion(source)
        } else {
            self.first.expansion(source)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnCall<'src> {
    pub func: FnSource<'src>,
    pub open: Token<'src>,
    pub args: Option<ArgList<'src>>,
    pub close: Token<'src>,
}

impl<'src> AstNode<'src> for FnCall<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        (&self.func, &self.close).range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.func, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Binary(Box<Binary<'src>>),
    Unary(Box<Unary<'src>>),
    Literal(Token<'src>),
    Grouping(Box<Grouping<'src>>),
    Type(Box<TypeExpr<'src>>),
    FnCall(Box<FnCall<'src>>),
}

impl<'src> AstNode<'src> for Expr<'src> {
    fn range(&self, source: &'src str) -> Range<usize> {
        match self {
            Self::Binary(inner) => inner.range(source),
            Self::Unary(inner) => inner.range(source),
            Self::Literal(token) => token.range(source),
            Self::Grouping(inner) => inner.range(source),
            Self::Type(inner) => inner.range(source),
            Self::FnCall(inner) => inner.range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self {
            Self::Binary(inner) => inner.expansion(source),
            Self::Unary(inner) => inner.expansion(source),
            Self::Literal(token) => token.expansion(source),
            Self::Grouping(inner) => inner.expansion(source),
            Self::Type(inner) => inner.expansion(source),
            Self::FnCall(inner) => inner.expansion(source),
        }
    }
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

    pub fn grouping(inner: Grouping<'src>) -> Self {
        Self::Grouping(Box::new(inner))
    }

    pub fn type_expr(inner: TypeExpr<'src>) -> Self {
        Self::Type(Box::new(inner))
    }

    pub fn fn_call(inner: FnCall<'src>) -> Self {
        Self::FnCall(Box::new(inner))
    }
}
