use crate::{
    error::OpSide,
    scanner::token::{ExpansionData, Token},
};
use std::range::Range;

/// Allows the AST to combine the full range of its tokens, for use in errors.
///
/// Implementors must define either
/// - [`Self::get_range`]
/// - both [`Self::get_range_start`] and [`Self::get_range_end`]
/// - or all three
///
/// Consider implementing [`AstNodeFinal`] if the conversion cannot fail.
pub trait AstNode<'src> {
    /// For when there is a less expensive implementation for ranges that are one-sided
    ///
    /// Returns [`None`] if the range is empty
    ///
    /// If this method returns [`Some`], [`Self::get_range_end`] must also return [`Some`].
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.get_range(source).map(|range| range.start)
    }

    /// For when there is a less expensive implementation for ranges that are one-sided
    ///
    /// Returns [`None`] if the range is empty
    ///
    /// If this method returns [`Some`], [`Self::get_range_start`] must also return [`Some`].
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.get_range(source).map(|range| range.end)
    }

    /// Returns [`None`] if the range is empty
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.get_range_start(source).map(|start| {
            let end = self
                .get_range_end(source)
                .expect("Some-ness of get_range_end should match that of get_range_start");
            Range { start, end }
        })
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    // TBD: what if part of it is from a nested macro?
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>>;
}

pub trait AstNodeFinal<'src>: AstNode<'src> {
    /// # Panics
    /// This method may panic if [`Self::get_range`] returns [`None`]
    fn range(&self, source: &'src str) -> Range<usize> {
        self.get_range(source)
            .expect("final range should not be empty")
    }
}

macro_rules! final_ast_node {
    derive() {
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident<$lt:lifetime> $def:tt
    } => {
        impl<$lt> $crate::grammar::ast::AstNodeFinal<$lt> for $Name<$lt> {}
    };

    derive() {
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident<$lt:lifetime> $def:tt
    } => {
        impl<$lt> $crate::grammar::ast::AstNodeFinal<$lt> for $Name<$lt> {}
    };
}
pub(crate) use final_ast_node as AstNodeF;

/// Passthrough references
impl<'src, T: ?Sized> AstNode<'src> for &T
where
    T: AstNode<'src>,
{
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        (*self).get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (*self).get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (*self).get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (*self).expansion(source)
    }
}

/// Between two nodes, takes the start of the first and the end of the second.
/// All nodes between can be omitted, which is why only tuples of 2 are defined.
impl<'src, T, U> AstNode<'src> for (T, U)
where
    T: AstNode<'src>,
    U: AstNode<'src>,
{
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.0
            .get_range_start(source)
            .or_else(|| self.1.get_range_start(source))
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.1
            .get_range_end(source)
            .or_else(|| self.0.get_range_end(source))
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match (self.0.get_range_start(source), self.1.get_range_end(source)) {
            (None, None) => None,
            (Some(start), None) => {
                let end = self
                    .0
                    .get_range_end(source)
                    .expect("Some-ness of get_range_end should match that of get_range_start");
                Some(Range { start, end })
            }
            (None, Some(end)) => {
                let start = self
                    .1
                    .get_range_start(source)
                    .expect("Some-ness of get_range_start should match that of get_range_end");
                Some(Range { start, end })
            }
            (Some(start), Some(end)) => Some(Range { start, end }),
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

impl<'src, T, U> AstNodeFinal<'src> for (T, U)
where
    T: AstNodeFinal<'src>,
    U: AstNodeFinal<'src>,
{
}

impl<'src, T> AstNode<'src> for Option<T>
where
    T: AstNode<'src>,
{
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.as_ref().and_then(|x| x.get_range_start(source))
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.as_ref().and_then(|x| x.get_range_end(source))
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.as_ref().and_then(|x| x.get_range(source))
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.as_ref().and_then(|x| x.expansion(source))
    }
}

impl<'src, T> AstNode<'src> for [T]
where
    T: AstNode<'src>,
{
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.first().and_then(|x| x.get_range_start(source))
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.last().and_then(|x| x.get_range_end(source))
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            [] => None,
            [only] => only.get_range(source),
            [first, .., last] => (first, last).get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self {
            [] => None,
            [only] => only.expansion(source),
            [first, .., last] => (first, last).expansion(source),
        }
    }
}

impl<'src> AstNode<'src> for Token<'src> {
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        Some(self.lex_range(source))
    }

    fn expansion(&self, _: &'src str) -> Option<ExpansionData<'src>> {
        self.mac
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Ternary<'src> {
    pub lhs: Expr<'src>,
    pub lop: Token<'src>,
    pub mhs: Expr<'src>,
    pub rop: Token<'src>,
    pub rhs: Expr<'src>,
}

impl<'src> AstNode<'src> for Ternary<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.lhs.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.rhs.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.lhs, &self.rhs).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check op because it's between them, so it must be in the same expansion if the other two are
        (&self.lhs, &self.rhs).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Binary<'src> {
    pub lhs: Expr<'src>,
    pub op: Token<'src>,
    pub rhs: Expr<'src>,
}

impl<'src> AstNode<'src> for Binary<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.lhs.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.rhs.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.lhs, &self.rhs).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check op because it's between them, so it must be in the same expansion if the other two are
        (&self.lhs, &self.rhs).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Unary<'src> {
    pub op: Token<'src>,
    pub operand: Expr<'src>,
    pub side: OpSide,
}

impl<'src> AstNode<'src> for Unary<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        match self.side {
            OpSide::Left => self.operand.get_range_start(source),
            OpSide::Right => self.op.get_range_start(source),
        }
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        match self.side {
            OpSide::Left => self.op.get_range_end(source),
            OpSide::Right => self.operand.get_range_end(source),
        }
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self.side {
            OpSide::Left => (&self.operand, &self.op).get_range(source),
            OpSide::Right => (&self.op, &self.operand).get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self.side {
            OpSide::Left => (&self.operand, &self.op).expansion(source),
            OpSide::Right => (&self.op, &self.operand).expansion(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Grouping<'src> {
    /// `(`
    pub open: Token<'src>,
    pub expr: Expr<'src>,
    /// `)`
    pub close: Token<'src>,
}

impl<'src> AstNode<'src> for Grouping<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_start(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check expr because it's between open and close,
        // and therefore must be in the same expansion if the other two are.
        // this is also cheaper, since now we don't have to recursively check the inner expressions :)
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct OrType<'src> {
    // `|`
    pub pipe: Token<'src>,
    // `none`, `fail`, or `nevr`
    pub ty: Token<'src>,
}

impl<'src> AstNode<'src> for OrType<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.pipe.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ty.get_range_start(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.pipe, &self.ty).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.pipe, &self.ty).expansion(source)
    }
}

/// A concrete type
#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct TypeExpr<'src> {
    // TODO: namespace?
    pub name: Token<'src>,
    // TODO: generic arguments?
    pub or_ty: Option<OrType<'src>>,
}

impl<'src> AstNode<'src> for TypeExpr<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.name.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (&self.name, self.or_ty.as_ref()).get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.name, self.or_ty.as_ref()).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.name, self.or_ty.as_ref()).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub enum Expr<'src> {
    Ternary(Box<Ternary<'src>>),
    Binary(Box<Binary<'src>>),
    Unary(Box<Unary<'src>>),
    Literal(Token<'src>),
    Grouping(Box<Grouping<'src>>),
    Type(Box<TypeExpr<'src>>),
}

impl<'src> AstNode<'src> for Expr<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        match self {
            Self::Ternary(inner) => inner.get_range_start(source),
            Self::Binary(inner) => inner.get_range_start(source),
            Self::Unary(inner) => inner.get_range_start(source),
            Self::Literal(token) => token.get_range_start(source),
            Self::Grouping(inner) => inner.get_range_start(source),
            Self::Type(inner) => inner.get_range_start(source),
        }
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        match self {
            Self::Ternary(inner) => inner.get_range_end(source),
            Self::Binary(inner) => inner.get_range_end(source),
            Self::Unary(inner) => inner.get_range_end(source),
            Self::Literal(token) => token.get_range_end(source),
            Self::Grouping(inner) => inner.get_range_end(source),
            Self::Type(inner) => inner.get_range_end(source),
        }
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            Self::Ternary(inner) => inner.get_range(source),
            Self::Binary(inner) => inner.get_range(source),
            Self::Unary(inner) => inner.get_range(source),
            Self::Literal(token) => token.get_range(source),
            Self::Grouping(inner) => inner.get_range(source),
            Self::Type(inner) => inner.get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self {
            Self::Ternary(inner) => inner.expansion(source),
            Self::Binary(inner) => inner.expansion(source),
            Self::Unary(inner) => inner.expansion(source),
            Self::Literal(token) => token.expansion(source),
            Self::Grouping(inner) => inner.expansion(source),
            Self::Type(inner) => inner.expansion(source),
        }
    }
}

impl<'src> Expr<'src> {
    pub fn ternary(inner: Ternary<'src>) -> Self {
        Self::Ternary(Box::new(inner))
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

    pub fn type_expr(inner: TypeExpr<'src>) -> Self {
        Self::Type(Box::new(inner))
    }
}
