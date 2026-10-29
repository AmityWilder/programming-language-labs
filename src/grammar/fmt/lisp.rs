use crate::{
    SYNTAX_STYLE_ANSI,
    grammar::ast::{
        Binary, BlockExpr, BlockStmt, DeclDefer, DeclVar, Declaration, Directive, Expr, Grouping,
        IfStmt, Literal, LoopBlock, LoopKind, OrBody, OrStmt, OrType, Statement, TypeExpr, Unary,
        Variable,
    },
    highlight::{style::StyleWrapper, syntax::Syntax, write_highlight},
};

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

/// Adapter to display contents as lisp
#[repr(transparent)]
pub struct LispDyn<'a>(dyn 'a + LispDisplay);

impl<'a> LispDyn<'a> {
    pub const fn new<'b>(value: &'b (dyn 'a + LispDisplay)) -> &'b Self {
        // SAFETY: Lisp is a transparent wrapper for `T`.
        unsafe { std::mem::transmute(value) }
    }
}

impl std::fmt::Display for LispDyn<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LispDisplay::fmt(&self.0, f) // calls LispDisplay::fmt, since T isn't proven to implement any other fmt
    }
}

fn lisp<'ops, T, U, I>(f: &mut std::fmt::Formatter<'_>, op: &T, operands: I) -> std::fmt::Result
where
    T: ?Sized + std::fmt::Display,
    U: ?Sized + std::fmt::Display + 'ops,
    I: 'ops + IntoIterator<Item = &'ops U>,
{
    f.write_str("(")?;
    if f.alternate() {
        std::fmt::Display::fmt(&crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(op), f)?;
    } else {
        &op.fmt(f)?;
    }
    for operand in operands {
        f.write_str(" ")?;
        operand.fmt(f)?;
    }
    f.write_str(")")
}

macro_rules! parenthesize {
    ($f:expr, $kw:expr, [$($args:expr),* $(,)?] $(,)?) => {{
        $f.write_str("(")?;
        if $f.alternate() {
            std::fmt::Display::fmt(&crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style($kw), $f)?;
        } else {
            std::fmt::Display::fmt(&$kw, $f)?;
        }
        $(
        $f.write_str(" ")?;
        std::fmt::Display::fmt(&$args, $f)?;
        )*
        $f.write_str(")")
    }};
}

impl LispDisplay for Binary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        parenthesize!(f, self.op.lex, [Lisp::new(&self.lhs), Lisp::new(&self.rhs)])
    }
}

impl LispDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        parenthesize!(f, self.op.lex, [Lisp::new(&self.operand)])
    }
}

impl LispDisplay for Grouping<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        parenthesize!(f, "group", [Lisp::new(&self.expr)])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TypeName<'src>(&'src str);

impl LispDisplay for TypeName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            std::fmt::Display::fmt(&SYNTAX_STYLE_ANSI[Syntax::Typename].style(self.0), f)
        } else {
            f.write_str(self.0)
        }
    }
}

impl LispDisplay for Literal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { value } = self;
        if f.alternate() {
            write_highlight(std::iter::once(value), f, &SYNTAX_STYLE_ANSI)
        } else {
            f.write_str(value.lex)
        }
    }
}

impl LispDisplay for Directive<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { kw } = self;
        if f.alternate() {
            std::fmt::Display::fmt(&SYNTAX_STYLE_ANSI[Syntax::Keyword].style(self.kw.lex), f)
        } else {
            f.write_str(kw.lex)
        }
    }
}

impl LispDisplay for Variable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { ident } = self;
        if f.alternate() {
            std::fmt::Display::fmt(&SYNTAX_STYLE_ANSI[Syntax::Variable].style(ident.lex), f)
        } else {
            f.write_str(ident.lex)
        }
    }
}

impl LispDisplay for OrStmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { body, .. } = self;
        match body {
            OrBody::More(inner) => LispDisplay::fmt(inner, f),
            OrBody::Last(inner) => LispDisplay::fmt(inner, f),
        }
    }
}

impl LispDisplay for IfStmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            if_kw,
            pred,
            then,
            or,
        } = self;
        lisp(
            f,
            &if_kw.lex,
            ([LispDyn::new(pred), LispDyn::new(then)])
                .into_iter()
                .chain(or.as_deref().into_iter().map(|or| LispDyn::new(or))),
        )
    }
}

impl LispDisplay for LoopBlock<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            label: _, // TBD
            kw,
            kind,
            block,
        } = self;
        match kind {
            LoopKind::Loop | LoopKind::Cord => lisp(f, &kw.lex, std::iter::once(Lisp::new(block))),

            LoopKind::Rep { .. } | LoopKind::For { .. } => {
                Err(std::fmt::Error) // not an expression
            }
        }
    }
}

impl LispDisplay for DeclVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            var_kw,
            binding,
            assignment,
            ..
        } = self;
        f.write_str("(")?;
        if f.alternate() {
            std::fmt::Display::fmt(
                &crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(var_kw.lex),
                f,
            )?;
        } else {
            std::fmt::Display::fmt(&var_kw.lex, f)?;
        }
        f.write_str(" ")?;
        std::fmt::Display::fmt(
            &crate::SYNTAX_STYLE_ANSI[Syntax::Variable].style(binding.ident.lex),
            f,
        )?;
        if let Some(assignment) = assignment {
            f.write_str(" ")?;
            LispDisplay::fmt(&assignment.expr, f)?;
        }
        f.write_str(")")
    }
}

impl LispDisplay for Statement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expr(inner) => LispDisplay::fmt(&inner.expr, f),
            Self::Print(_) => Ok(()), // wont this leave an extra space?
            Self::If(inner) => LispDisplay::fmt(inner, f),
            Self::Loop(inner) => LispDisplay::fmt(inner, f),
            Self::Done(_inner) => todo!(),
            Self::Skip(_inner) => todo!(),
            Self::Block(inner) => LispDisplay::fmt(inner, f),
        }
    }
}

impl LispDisplay for Declaration<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(inner) => LispDisplay::fmt(inner, f),
            Self::Defer(_inner) => Ok(()), // wont this leave an extra space?
            Self::Stmt(inner) => LispDisplay::fmt(inner, f),
        }
    }
}

impl LispDisplay for BlockStmt<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { body, .. } = self;
        lisp(f, "block", body.iter().map(|x| LispDyn::new(x)))
    }
}

impl LispDisplay for BlockExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::If(inner) => LispDisplay::fmt(&**inner, f),
            Self::Loop(inner) => LispDisplay::fmt(&**inner, f),
            Self::Block(inner) => LispDisplay::fmt(inner, f),
        }
    }
}

impl LispDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block(inner) => LispDisplay::fmt(inner, f),
            Self::Binary(inner) => LispDisplay::fmt(&**inner, f),
            Self::Unary(inner) => LispDisplay::fmt(&**inner, f),
            // TBD: should this use val instead of lex?
            Self::Literal(inner) => LispDisplay::fmt(inner, f),
            Self::Directive(inner) => LispDisplay::fmt(inner, f),
            Self::Variable(inner) => LispDisplay::fmt(inner, f),
            Self::Grouping(inner) => LispDisplay::fmt(&**inner, f),
            Self::Type(_inner) => todo!(),
        }
    }
}
