use crate::{
    SYNTAX_STYLE_ANSI,
    grammar::{Binary, Expr, Grouping, OrType, TypeExpr, Unary},
    highlight::{style::StyleWrapper, syntax::Syntax, write_highlight},
    scanner::token::{Token, punc::Punctuation, value::LexValue},
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

/// Means of displaying content with Polish notation
#[allow(dead_code, reason = "for bonus points")]
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

impl LispDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(
                f,
                "({} {:#})",
                crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(self.op.lex),
                Lisp::new(&self.operand)
            )
        } else {
            write!(f, "({} {})", self.op.lex, Lisp::new(&self.operand))
        }
    }
}

impl PolishDisplay for Unary<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self.op.val {
            // TBD: how does polish notation represent unary negative?
            LexValue::Punctuation(Punctuation::SubNeg) => "- 0",
            _ => self.op.lex,
        };
        write!(f, "{op} {}", Polish::new(&self.operand))
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

impl LispDisplay for TypeExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, or_ty } = self;
        if f.alternate() {
            if let Some(OrType { pipe, ty }) = or_ty {
                write!(
                    f,
                    "({} {} {})",
                    crate::SYNTAX_STYLE_ANSI[Syntax::Keyword].style(pipe.lex),
                    crate::SYNTAX_STYLE_ANSI[Syntax::Typename].style(name.lex),
                    crate::SYNTAX_STYLE_ANSI[Syntax::Typename].style(ty.lex)
                )
            } else {
                let ty = crate::SYNTAX_STYLE_ANSI[Syntax::Typename];
                std::fmt::Display::fmt(&ty.style(name.lex), f)
            }
        } else {
            if let Some(OrType { pipe, ty }) = or_ty {
                write!(f, "({} {} {})", pipe.lex, name.lex, ty.lex)
            } else {
                f.write_str(name.lex)
            }
        }
    }
}

impl PolishDisplay for TypeExpr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, or_ty } = self;
        if let Some(OrType { pipe, ty }) = or_ty {
            write!(f, "{} {} {}", pipe.lex, name.lex, ty.lex)
        } else {
            f.write_str(name.lex)
        }
    }
}

impl LispDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            Self::Type(inner) => LispDisplay::fmt(inner, f),
        }
    }
}

impl PolishDisplay for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(inner) => PolishDisplay::fmt(&**inner, f),
            Self::Unary(inner) => PolishDisplay::fmt(&**inner, f),
            Self::Literal(Token { lex, .. }) => f.write_str(lex), // TBD: should this use val instead of lex?
            Self::Grouping(inner) => PolishDisplay::fmt(&**inner, f),
            Self::Type(inner) => PolishDisplay::fmt(inner, f),
        }
    }
}
