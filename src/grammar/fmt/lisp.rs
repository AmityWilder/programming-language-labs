use crate::{
    SYNTAX_STYLE_ANSI,
    grammar::ast::{Binary, Directive, Expr, Grouping, Literal, OrType, TypeExpr, Unary, Variable},
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

macro_rules! parenthesize {
    ($f:expr, $kw:expr $(, $args:expr)* $(,)?) => {{
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
        parenthesize!(f, self.op.lex, Lisp::new(&self.lhs), Lisp::new(&self.rhs))
    }
}

impl LispDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        parenthesize!(f, self.op.lex, Lisp::new(&self.operand))
    }
}

impl LispDisplay for Grouping<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        parenthesize!(f, "group", Lisp::new(&self.expr))
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

impl LispDisplay for TypeExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, or_ty } = self;
        if let Some(OrType { pipe, ty }) = or_ty {
            parenthesize!(
                f,
                pipe.lex,
                Lisp::new(&TypeName(name.lex)),
                Lisp::new(&TypeName(ty.lex)),
            )
        } else {
            std::fmt::Display::fmt(Lisp::new(&TypeName(name.lex)), f)
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

impl LispDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => LispDisplay::fmt(&**inner, f),
            Self::Unary(inner) => LispDisplay::fmt(&**inner, f),
            // TBD: should this use val instead of lex?
            Self::Literal(inner) => LispDisplay::fmt(inner, f),
            Self::Directive(inner) => LispDisplay::fmt(inner, f),
            Self::Variable(inner) => LispDisplay::fmt(inner, f),
            Self::Grouping(inner) => LispDisplay::fmt(&**inner, f),
            Self::Type(inner) => LispDisplay::fmt(&**inner, f),
        }
    }
}
