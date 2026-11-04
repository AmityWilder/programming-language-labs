use crate::{
    SYNTAX_STYLE_ANSI,
    highlight::{style::StyleWrapper, write_highlight},
    lex::token::TokenKind,
    parse::ast,
    src_code::{code::SourceCode, span::RelSpan},
};
use std::{
    fmt::{self},
    iter::once,
};

macro_rules! chain {
    ($only:expr $(,)?) => {
        $only.into_iter().map(|x| -> &dyn LispDisplay { x })
    };

    ($first:expr, $($rest:expr),+ $(,)?) => {
        chain!($first)$(.chain(chain!($rest)))+
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lisp<'src, 'ast, T: ?Sized> {
    pub source: &'src SourceCode,
    pub node: &'ast T,
}

impl<T: ?Sized + LispDisplay> fmt::Display for Lisp<'_, '_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.node.fmt(self.source, f)
    }
}

pub trait LispDisplayValue: Sized {
    fn fmt_value(self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub trait LispDisplay {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: ?Sized + LispDisplay> LispDisplayValue for &T {
    fn fmt_value(self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self).fmt(source, f)
    }
}

impl<T: Into<TokenKind>> LispDisplayValue for (&str, T) {
    fn fmt_value(self, _source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (span, kind) = self;
        write_highlight(once((span, kind.into())), f, &SYNTAX_STYLE_ANSI)
    }
}

impl<T: Into<TokenKind>> LispDisplayValue for (RelSpan, T) {
    fn fmt_value(self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (span, kind) = self;
        (source.get(span).expect("AST spans should be valid"), kind).fmt_value(source, f)
    }
}

impl<O, A> LispDisplayValue for (O, A)
where
    O: LispDisplayValue,
    A: IntoIterator<Item: LispDisplayValue>,
{
    fn fmt_value(self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let style = SYNTAX_STYLE_ANSI.keyword;
        let (op, args) = self;
        f.write_str("(")?;
        if f.alternate() {
            fmt::Display::fmt(&style.begin(), f)?;
        }
        op.fmt_value(source, f)?;
        if f.alternate() {
            fmt::Display::fmt(&style.end(), f)?;
        }
        for arg in args {
            f.write_str(" ")?;
            arg.fmt_value(source, f)?;
        }
        f.write_str(")")
    }
}

impl LispDisplay for ast::ProgramUnit {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ("unit", &self.decls).fmt_value(source, f)
    }
}

impl LispDisplay for ast::Binary {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispDisplayValue::fmt_value(((self.op_span, self.op), [&self.lhs, &self.rhs]), source, f)
    }
}

impl LispDisplay for ast::Unary {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.op_span, [&self.operand]).fmt_value(source, f)
    }
}

impl LispDisplay for ast::Literal {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.span.fmt_value(source, f)
    }
}

impl LispDisplay for ast::Directive {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kw_span.fmt_value(source, f)
    }
}

impl LispDisplay for ast::Variable {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ident.fmt_value(source, f)
    }
}

impl LispDisplay for ast::Grouping {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ("group", once(&self.expr)).fmt_value(source, f)
    }
}

impl LispDisplay for ast::BlockStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ("block", self.body.as_slice()).fmt_value(source, f)
    }
}

impl LispDisplay for ast::BlockExpr {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::If(inner) => inner.fmt(source, f),
            Self::Loop(inner) => inner.fmt(source, f),
            Self::Block(inner) => inner.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::Expr {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(inner) => inner.fmt(source, f),
            Self::Binary(inner) => inner.fmt(source, f),
            Self::Unary(inner) => inner.fmt(source, f),
            Self::Literal(inner) => inner.fmt(source, f),
            Self::Directive(inner) => inner.fmt(source, f),
            Self::Variable(inner) => inner.fmt(source, f),
            Self::Grouping(inner) => inner.fmt(source, f),
            Self::Type(inner) => inner.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::ExprStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.expr.fmt(source, f)
    }
}

impl LispDisplay for ast::PrintStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.kw, [&self.group.expr]).fmt_value(source, f)
    }
}

impl LispDisplay for ast::IfStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (
            self.kw,
            chain!(once(&self.pred), once(&self.then), self.or.as_deref()),
        )
            .fmt_value(source, f)
    }
}

impl LispDisplay for ast::OrStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.kw, once(&self.body)).fmt_value(source, f)
    }
}

impl LispDisplay for ast::OrBody {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::More(inner) => inner.fmt(source, f),
            Self::Last(inner) => inner.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::ScopeLabel {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.label.fmt(source, f)
    }
}

impl LispDisplay for ast::Label {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.dot, once(self.name)).fmt_value(source, f)
    }
}

impl LispDisplay for ast::DoneStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.kw, chain!(self.label.as_ref(), self.value.as_ref())).fmt_value(source, f)
    }
}

impl LispDisplay for ast::SkipStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.kw, chain!(self.label.as_ref())).fmt_value(source, f)
    }
}

impl LispDisplay for ast::LoopBlock {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ast::LoopKind::Loop | ast::LoopKind::Cord => {
                (self.kw, chain!(self.label.as_ref(), once(&self.block))).fmt_value(source, f)
            }

            ast::LoopKind::Rep { pred } => (
                self.kw,
                chain!(self.label.as_ref(), once(pred), once(&self.block)),
            )
                .fmt_value(source, f),

            ast::LoopKind::For { .. } => todo!(),
        }
    }
}

impl LispDisplay for ast::Statement {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expr(inner) => inner.fmt(source, f),
            Self::Print(inner) => inner.fmt(source, f),
            Self::If(inner) => inner.fmt(source, f),
            Self::Loop(inner) => inner.fmt(source, f),
            Self::Done(inner) => inner.fmt(source, f),
            Self::Skip(inner) => inner.fmt(source, f),
            Self::Block(inner) => inner.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::BindingPattern {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ident.fmt_value(source, f)
    }
}

impl LispDisplay for ast::DeferParam {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.param.fmt(source, f)
    }
}

impl LispDisplay for ast::DeclDefer {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.kw, chain!(self.param.as_ref(), once(&self.on_pop))).fmt_value(source, f)
    }
}

impl LispDisplay for ast::DeclVar {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (
            self.kw_span,
            chain!(
                once(&self.binding),
                self.ty.as_ref(),
                self.assignment.as_ref()
            ),
        )
            .fmt_value(source, f)
    }
}

impl LispDisplay for ast::Assignment {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.op, once(&self.expr)).fmt_value(source, f)
    }
}

impl LispDisplay for ast::TypeAnnotation {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ty.fmt_value(source, f)
    }
}

impl LispDisplay for ast::TypeExpr {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(or_ty) = self.or_ty.as_ref() {
            (or_ty.pipe, chain!(once(&self.then_ty), once(or_ty))).fmt_value(source, f)
        } else {
            self.then_ty.fmt(source, f)
        }
    }
}

impl LispDisplay for ast::OrType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ty_span.fmt_value(source, f)
    }
}

impl LispDisplay for ast::TypeInstance {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(args) = self.args.as_ref() {
            ("generic", chain!(once(&self.name), args.list.as_slice())).fmt_value(source, f)
        } else {
            self.name.fmt(source, f)
        }
    }
}

impl LispDisplay for ast::Typename {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            self.name_span.fmt_value(source, f)
        } else {
            todo!()
        }
    }
}

impl LispDisplay for ast::NamespacePathPart {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl LispDisplay for ast::Declaration {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(inner) => inner.fmt(source, f),
            Self::Defer(inner) => inner.fmt(source, f),
            Self::Stmt(inner) => inner.fmt(source, f),
        }
    }
}
