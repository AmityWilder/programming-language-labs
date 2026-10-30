use crate::{
    SYNTAX_STYLE_ANSI,
    analyzer::{
        Analyze,
        static_env::StaticEnvironment,
        types::{AbstractType, StaticType},
    },
    grammar::ast::{self, ExprPrecedence},
    highlight::{style::StyleWrapper, write_highlight},
    scanner::{token::value::Operator, tokenize},
    types::{byte, sint, uint},
};
use std::fmt::Display;

// TODO: this is all very rigid

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PureOperation<'src> {
    // Top-level
    And(PureExpr<'src>, PureExpr<'src>),
    Or(PureExpr<'src>, PureExpr<'src>),
    Xor(PureExpr<'src>, PureExpr<'src>),
    Nand(PureExpr<'src>, PureExpr<'src>),
    Nor(PureExpr<'src>, PureExpr<'src>),
    Xnor(PureExpr<'src>, PureExpr<'src>),

    Eq(PureExpr<'src>, PureExpr<'src>),
    Ne(PureExpr<'src>, PureExpr<'src>),
    Lt(PureExpr<'src>, PureExpr<'src>),
    Gt(PureExpr<'src>, PureExpr<'src>),
    Le(PureExpr<'src>, PureExpr<'src>),
    Ge(PureExpr<'src>, PureExpr<'src>),

    Not(PureExpr<'src>),
    Exists(PureExpr<'src>),

    // Binary
    BitAnd(PureExpr<'src>, PureExpr<'src>),
    BitOr(PureExpr<'src>, PureExpr<'src>),
    BitXor(PureExpr<'src>, PureExpr<'src>),
    BitNand(PureExpr<'src>, PureExpr<'src>),
    BitNor(PureExpr<'src>, PureExpr<'src>),
    BitXnor(PureExpr<'src>, PureExpr<'src>),

    Add(PureExpr<'src>, PureExpr<'src>),
    Sub(PureExpr<'src>, PureExpr<'src>),
    Mul(PureExpr<'src>, PureExpr<'src>),
    Pow(PureExpr<'src>, PureExpr<'src>),
    Div(PureExpr<'src>, PureExpr<'src>),
    Rem(PureExpr<'src>, PureExpr<'src>),

    Shl(PureExpr<'src>, PureExpr<'src>),
    Shr(PureExpr<'src>, PureExpr<'src>),
    Rotl(PureExpr<'src>, PureExpr<'src>),
    Rotr(PureExpr<'src>, PureExpr<'src>),

    // Unary Prefix
    BitNot(PureExpr<'src>),
    Neg(PureExpr<'src>),
    Szof(PureExpr<'src>),
}

impl<'src> ast::Binary<'src> {
    fn analyze_pure(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> crate::analyzer::Result<'src, PureOperation<'src>> {
        todo!()
    }
}

impl<'src> ast::Unary<'src> {
    fn analyze_pure(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> crate::analyzer::Result<'src, PureOperation<'src>> {
        todo!()
    }
}

impl<'src> PureOperation<'src> {
    pub const fn precedence(&self) -> ExprPrecedence {
        match self {
            Self::Sub { .. } => ExprPrecedence::Term,
            Self::Neg { .. } => ExprPrecedence::UnaryPre,
            _ => self.op().precedence(),
        }
    }

    pub const fn lhs(&self) -> Option<&PureExpr<'src>> {
        match self {
            Self::And(l, _)
            | Self::Or(l, _)
            | Self::Xor(l, _)
            | Self::Nand(l, _)
            | Self::Nor(l, _)
            | Self::Xnor(l, _)
            | Self::Eq(l, _)
            | Self::Ne(l, _)
            | Self::Lt(l, _)
            | Self::Gt(l, _)
            | Self::Le(l, _)
            | Self::Ge(l, _)
            | Self::BitAnd(l, _)
            | Self::BitOr(l, _)
            | Self::BitXor(l, _)
            | Self::BitNand(l, _)
            | Self::BitNor(l, _)
            | Self::BitXnor(l, _)
            | Self::Add(l, _)
            | Self::Sub(l, _)
            | Self::Mul(l, _)
            | Self::Pow(l, _)
            | Self::Div(l, _)
            | Self::Rem(l, _)
            | Self::Shl(l, _)
            | Self::Shr(l, _)
            | Self::Rotl(l, _)
            | Self::Rotr(l, _) => Some(l),

            Self::BitNot(_) | Self::Not(_) | Self::Exists(_) | Self::Neg(_) | Self::Szof(_) => None,
        }
    }

    #[expect(clippy::unnecessary_wraps, reason = "consistency")]
    pub const fn rhs(&self) -> Option<&PureExpr<'src>> {
        match self {
            Self::And(_, r)
            | Self::Or(_, r)
            | Self::Xor(_, r)
            | Self::Nand(_, r)
            | Self::Nor(_, r)
            | Self::Xnor(_, r)
            | Self::Eq(_, r)
            | Self::Ne(_, r)
            | Self::Lt(_, r)
            | Self::Gt(_, r)
            | Self::Le(_, r)
            | Self::Ge(_, r)
            | Self::Not(r)
            | Self::Exists(r)
            | Self::BitAnd(_, r)
            | Self::BitOr(_, r)
            | Self::BitXor(_, r)
            | Self::BitNand(_, r)
            | Self::BitNor(_, r)
            | Self::BitXnor(_, r)
            | Self::Add(_, r)
            | Self::Sub(_, r)
            | Self::Mul(_, r)
            | Self::Pow(_, r)
            | Self::Div(_, r)
            | Self::Rem(_, r)
            | Self::Shl(_, r)
            | Self::Shr(_, r)
            | Self::Rotl(_, r)
            | Self::Rotr(_, r)
            | Self::BitNot(r)
            | Self::Neg(r)
            | Self::Szof(r) => Some(r),
        }
    }

    pub const fn op(&self) -> Operator {
        match self {
            Self::And { .. } | Self::BitAnd { .. } => Operator::And,
            Self::Or { .. } | Self::BitOr { .. } => Operator::Or,
            Self::Xor { .. } | Self::BitXor { .. } => Operator::Xor,
            Self::Nand { .. } | Self::BitNand { .. } => Operator::Nand,
            Self::Nor { .. } | Self::BitNor { .. } => Operator::Nor,
            Self::Xnor { .. } | Self::BitXnor { .. } => Operator::Xnor,
            Self::Add { .. } => Operator::Add,
            Self::Sub { .. } => Operator::Sub,
            Self::Mul { .. } => Operator::Mul,
            Self::Pow { .. } => Operator::Pow,
            Self::Div { .. } => Operator::Div,
            Self::Rem { .. } => Operator::Rem,
            Self::Shl { .. } => Operator::Shl,
            Self::Shr { .. } => Operator::Shr,
            Self::Rotl { .. } => Operator::Rotl,
            Self::Rotr { .. } => Operator::Rotr,
            Self::Eq { .. } => Operator::Eq,
            Self::Ne { .. } => Operator::Ne,
            Self::Lt { .. } => Operator::Lt,
            Self::Gt { .. } => Operator::Gt,
            Self::Le { .. } => Operator::Le,
            Self::Ge { .. } => Operator::Ge,
            Self::BitNot { .. } | Self::Not { .. } => Operator::Not,
            Self::Neg { .. } => Operator::Neg,
            Self::Exists { .. } => Operator::Exists,
            Self::Szof { .. } => Operator::Szof,
        }
    }
}

impl Display for PureOperation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let is_spaced = match self {
            Self::And { .. }
            | Self::Or { .. }
            | Self::Xor { .. }
            | Self::Nand { .. }
            | Self::Nor { .. }
            | Self::Xnor { .. }
            | Self::Eq { .. }
            | Self::Ne { .. }
            | Self::Lt { .. }
            | Self::Gt { .. }
            | Self::Le { .. }
            | Self::Ge { .. }
            | Self::BitAnd { .. }
            | Self::BitOr { .. }
            | Self::BitXor { .. }
            | Self::BitNand { .. }
            | Self::BitNor { .. }
            | Self::BitXnor { .. }
            | Self::Add { .. }
            | Self::Sub { .. }
            | Self::Div { .. }
            | Self::Rem { .. }
            | Self::Shl { .. }
            | Self::Shr { .. }
            | Self::Rotl { .. }
            | Self::Rotr { .. }
            | Self::Szof { .. } => true,

            Self::Not { .. }
            | Self::BitNot { .. }
            | Self::Exists { .. }
            | Self::Mul { .. }
            | Self::Pow { .. }
            | Self::Neg { .. } => false,
        };
        // TODO: parenthesize depending on precedence
        if let Some(l) = self.lhs() {
            let is_grouped = self.precedence() > l.precedence();
            if is_grouped {
                f.write_str("(")?;
            }
            l.fmt(f)?;
            if is_grouped {
                f.write_str(")")?;
            }
            if is_spaced {
                f.write_str(" ")?;
            }
        }
        self.op().fmt(f)?;
        if let Some(r) = self.rhs() {
            if is_spaced {
                f.write_str(" ")?;
            }
            let is_grouped = self.precedence() > r.precedence();
            if is_grouped {
                f.write_str("(")?;
            }
            r.fmt(f)?;
            if is_grouped {
                f.write_str(")")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PureLiteral {
    #[default]
    None,
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    // TBD: should flpt be supported?
    Char(char),
    Text(String),
}

impl<'src> ast::Literal<'src> {
    fn analyze_pure(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> crate::analyzer::Result<'src, PureLiteral> {
        todo!()
    }
}

impl Display for PureLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let style = match self {
            Self::None | Self::Bool(_) => &SYNTAX_STYLE_ANSI.language_defined,
            Self::Byte(_) | Self::Uint(_) | Self::Sint(_) => &SYNTAX_STYLE_ANSI.number_literal,
            Self::Char(_) | Self::Text(_) => todo!(), // TODO: escapes
        };
        match self {
            Self::None => style.alt_style("none").fmt(f),
            Self::Bool(true) => style.alt_style("true").fmt(f),
            Self::Bool(false) => style.alt_style("fals").fmt(f),
            Self::Byte(n) => style.alt_style(n).fmt(f),
            Self::Uint(n) => style.alt_style(n).fmt(f),
            Self::Sint(n) => style.alt_style(n).fmt(f),
            // hmmm...
            Self::Char(c) => write_highlight(tokenize(&format!("{c:?}")), f, &SYNTAX_STYLE_ANSI),
            Self::Text(s) => write_highlight(tokenize(&format!("{s:?}")), f, &SYNTAX_STYLE_ANSI),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PureVariable<'src>(pub &'src str);

impl<'src> ast::Variable<'src> {
    fn analyze_pure(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> crate::analyzer::Result<'src, PureVariable<'src>> {
        todo!()
    }
}

impl Display for PureVariable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        SYNTAX_STYLE_ANSI.variable.alt_style(self.0).fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PureExpr<'src> {
    Operation(Box<PureOperation<'src>>),
    Literal(PureLiteral),
    Variable(PureVariable<'src>),
}

impl<'src> ast::Expr<'src> {
    fn analyze_pure(
        &self,
        source: &'src str,
        env: &mut StaticEnvironment<'src>,
        ret: Option<&AbstractType<'src>>,
    ) -> crate::analyzer::Result<'src, PureExpr<'src>> {
        match self {
            Self::Binary(x) => x
                .analyze_pure(source, env, ret)
                .map(Box::new)
                .map(PureExpr::Operation),

            Self::Unary(x) => x
                .analyze_pure(source, env, ret)
                .map(Box::new)
                .map(PureExpr::Operation),

            Self::Literal(x) => x.analyze_pure(source, env, ret).map(PureExpr::Literal),

            Self::Variable(x) => x.analyze_pure(source, env, ret).map(PureExpr::Variable),

            Self::Grouping(x) => x.expr.analyze_pure(source, env, ret),

            Self::Block(_) => todo!("error: cannot have a block expression in a pure expression"),
            Self::Directive(_) => todo!("error: cannot have a directive in a pure expression"),
            Self::Type(_) => todo!(),
        }
    }
}

impl PureExpr<'_> {
    pub const fn precedence(&self) -> ExprPrecedence {
        match self {
            Self::Operation(inner) => inner.precedence(),
            Self::Literal(_) | Self::Variable(_) => ExprPrecedence::Primary,
        }
    }
}

impl Default for PureExpr<'_> {
    fn default() -> Self {
        Self::Literal(PureLiteral::default())
    }
}

impl Display for PureExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operation(inner) => inner.fmt(f),
            Self::Literal(inner) => inner.fmt(f),
            Self::Variable(inner) => inner.fmt(f),
        }
    }
}
