use crate::{
    SYNTAX_STYLE_ANSI,
    grammar::{
        Binary, Expr, Grouping, OrType, TypeExpr, Unary,
        ast::{ArgList, ArgList1, FnCall, FnSource, Ternary},
    },
    highlight::{style::StyleWrapper, syntax::Syntax, write_highlight},
    scanner::token::Token,
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

impl LispDisplay for Ternary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        parenthesize!(
            f,
            format!("{}{}", self.lop.lex, self.rop.lex),
            Lisp::new(&self.mhs),
            Lisp::new(&self.lhs),
            Lisp::new(&self.rhs)
        )
    }
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

impl LispDisplay for TypeExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, or_ty } = self;
        if let Some(OrType { pipe, ty }) = or_ty {
            parenthesize!(f, pipe.lex, name.lex, ty.lex)
        } else {
            std::fmt::Display::fmt(name.lex, f)
        }
    }
}

impl LispDisplay for FnCall<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { func, args, .. } = self;
        f.write_str("(")?;
        match func {
            FnSource::Ident(name) => {
                if f.alternate() {
                    std::fmt::Display::fmt(
                        &crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(name.lex),
                        f,
                    )?;
                } else {
                    f.write_str(name.lex)?;
                }
            }
            FnSource::Group(grouping) => LispDisplay::fmt(grouping, f)?,
        }
        if let Some(ArgList { first, rest, .. }) = args {
            f.write_str(" ")?;
            LispDisplay::fmt(first, f)?;
            let mut rest = rest.as_ref();
            while let Some(ArgList1 {
                arg, rest: next, ..
            }) = rest
            {
                f.write_str(" ")?;
                LispDisplay::fmt(arg, f)?;
                rest = next.as_deref();
            }
        }
        f.write_str(")")
    }
}

impl LispDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ternary(inner) => LispDisplay::fmt(&**inner, f),
            Self::Binary(inner) => LispDisplay::fmt(&**inner, f),
            Self::Unary(inner) => LispDisplay::fmt(&**inner, f),
            // TBD: should this use val instead of lex?
            Self::Literal(tkn @ Token { lex, .. }) => {
                if f.alternate() {
                    write_highlight(std::iter::once(tkn), f, &SYNTAX_STYLE_ANSI)
                } else {
                    f.write_str(lex)
                }
            }
            Self::Grouping(inner) => LispDisplay::fmt(&**inner, f),
            Self::Type(inner) => LispDisplay::fmt(&**inner, f),
            Self::FnCall(inner) => LispDisplay::fmt(&**inner, f),
        }
    }
}
