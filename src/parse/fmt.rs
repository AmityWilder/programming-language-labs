use crate::{
    SYNTAX_STYLE_ANSI,
    highlight::{TokenHighlight, style::StyleWrapper, write_highlight},
    lex::token::TokenKind,
    parse::ast,
    src_code::{code::SourceCode, span::RelSpan},
};
use std::{
    fmt::{self, Write},
    iter::once,
};

mod helpers {
    use super::*;

    pub trait LispOp {
        fn fmt_op(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    impl LispOp for str {
        fn fmt_op(&self, _source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&SYNTAX_STYLE_ANSI.keyword.alt_style(self), f)
        }
    }

    impl LispOp for RelSpan {
        fn fmt_op(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            source
                .get(*self)
                .expect("should be a valid span within source")
                .fmt_op(source, f)
        }
    }

    pub trait LispArgs {
        fn fmt(&self, source: &SourceCode, f: &mut LispCall<'_, '_>) -> fmt::Result;
    }

    pub struct LispCall<'fmt, 'buf>(&'fmt mut fmt::Formatter<'buf>);

    impl<'fmt, 'buf> LispCall<'fmt, 'buf> {
        pub fn new(
            op: &(impl ?Sized + LispOp),
            source: &SourceCode,
            f: &'fmt mut fmt::Formatter<'buf>,
        ) -> Result<Self, fmt::Error> {
            f.write_char('(')?;
            op.fmt_op(source, f)?;
            Ok(Self(f))
        }

        pub fn call<'me>(
            &'me mut self,
            op: &(impl ?Sized + LispOp),
            source: &SourceCode,
        ) -> Result<LispCall<'me, 'buf>, fmt::Error> {
            self.0.write_char(' ')?;
            LispCall::new(op, source, self.0)
        }

        pub fn call_with<'me>(
            &'me mut self,
            op: &(impl ?Sized + LispOp),
            source: &SourceCode,
            f: impl FnOnce(&mut LispCall<'me, 'buf>) -> Result<(), fmt::Error>,
        ) -> Result<(), fmt::Error> {
            let mut call = self.call(op, source)?;
            f(&mut call)?;
            call.finish()
        }

        pub fn write_arg<'me>(
            &'me mut self,
            arg: impl LispDisplayValue,
            source: &SourceCode,
        ) -> Result<&'me mut Self, fmt::Error> {
            self.0.write_char(' ')?;
            arg.fmt_value(source, self.0)?;
            Ok(self)
        }

        pub fn write_arg_iter<'me>(
            &'me mut self,
            args: impl IntoIterator<Item: LispDisplayValue>,
            source: &SourceCode,
        ) -> Result<&'me mut Self, fmt::Error> {
            for arg in args {
                self.0.write_char(' ')?;
                arg.fmt_value(source, self.0)?;
            }
            Ok(self)
        }

        pub fn write_args<'me>(
            &'me mut self,
            args: &(impl ?Sized + LispArgs),
            source: &SourceCode,
        ) -> Result<&'me mut Self, fmt::Error> {
            args.fmt(source, self)?;
            Ok(self)
        }

        pub fn finish(&mut self) -> fmt::Result {
            self.0.write_char(')')
        }
    }
}
use helpers::*;

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

impl<T> LispDisplayValue for (RelSpan, T)
where
    for<'src> (&'src str, T): TokenHighlight<'src>,
{
    fn fmt_value(self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (span, kind) = self;
        let src = source.get(span).expect("AST spans should be valid");
        write_highlight(once((src, kind)), f, &SYNTAX_STYLE_ANSI)
    }
}

impl LispArgs for [ast::Decl] {
    fn fmt(&self, source: &SourceCode, f: &mut LispCall<'_, '_>) -> fmt::Result {
        let mut iter = self.iter();
        while let Some(item) = iter.next() {
            match item {
                ast::Decl::Type(ty) => {
                    f.write_arg(ty, source)?;
                }

                ast::Decl::Var(var) => {
                    f.write_arg(var, source)?;
                    if let Some(assignment) = &var.assignment {
                        f.call(&assignment.op, source)?
                            .write_arg(&var.binding, source)?
                            .write_arg(&assignment.expr, source)?
                            .finish()?;
                    }
                }

                ast::Decl::Defer(defer) => {
                    f.call(&defer.kw, source)?
                        .write_arg(&defer.on_pop, source)?
                        .write_args(iter.as_slice(), source)?
                        .finish()?;
                }

                ast::Decl::Stmt(inner) => {
                    f.write_arg(inner, source)?.finish()?;
                }
            }
        }
        Ok(())
    }
}

impl LispDisplay for ast::ProgramUnit {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("program_unit", source, f)?
            .write_args(self.decls.as_slice(), source)?
            .finish()
    }
}

impl LispDisplay for ast::BlockStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("block", source, f)?
            .write_args(self.body.as_slice(), source)?
            .finish()
    }
}

impl LispDisplay for ast::DeclType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rec(rec) => rec.fmt(source, f),
            Self::Cat(cat) => cat.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::GenericGroup {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("generics", source, f)?
            .write_arg_iter(self.list.as_slice(), source)?
            .finish()
    }
}

impl LispDisplay for ast::DeclRec {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw, source, f)?
            .write_arg((self.name, TokenKind::Ident), source)?
            .write_arg_iter(self.generics.as_ref(), source)?
            .write_arg(&self.body, source)?
            .finish()
    }
}

impl LispDisplay for ast::FieldSet {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("fieldset", source, f)?
            .write_arg_iter(self.fields.as_slice(), source)?
            .finish()
    }
}

impl LispDisplay for ast::Field {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("field", source, f)?
            .write_arg((self.name, TokenKind::Ident), source)?
            .write_arg(&self.ty.ty, source)?
            .finish()
    }
}

impl LispDisplay for ast::DeclCat {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw, source, f)?
            .write_arg((self.name, TokenKind::Ident), source)?
            .write_arg(&self.discriminants, source)?
            .finish()
    }
}

impl LispDisplay for ast::CatDiscSet {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("discset", source, f)?
            .write_arg_iter(self.items.as_slice(), source)?
            .finish()
    }
}

impl LispDisplay for ast::CatDisc {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("=", source, f)?
            .write_arg((self.name, TokenKind::Ident), source)?
            .write_arg_iter(self.val.as_ref().map(|x| &x.expr), source)?
            .finish()
    }
}

impl LispDisplay for ast::Binary {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(self.op.as_str(), source, f)?
            .write_arg(&self.lhs, source)?
            .write_arg(&self.rhs, source)?
            .finish()
    }
}

impl LispDisplay for ast::Unary {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(self.op.as_str(), source, f)?
            .write_arg(&self.operand, source)?
            .finish()
    }
}

impl LispDisplay for ast::Literal {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.span, self.kind).fmt_value(source, f)
    }
}

impl LispDisplay for ast::Directive {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.kw_span, TokenKind::from(self.kw.to_keyword())).fmt_value(source, f)
    }
}

impl LispDisplay for ast::Variable {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.ident, TokenKind::Ident).fmt_value(source, f)
    }
}

impl LispDisplay for ast::Grouping {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("group", source, f)?
            .write_arg(&self.expr, source)?
            .finish()
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
        LispCall::new(&self.kw, source, f)?
            .write_arg(&self.group.expr, source)?
            .finish()
    }
}

impl LispDisplay for ast::IfStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw, source, f)?
            .write_arg(&self.pred, source)?
            .write_arg(&self.then, source)?
            .write_arg_iter(self.or.as_deref(), source)?
            .finish()
    }
}

impl LispDisplay for ast::OrStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw, source, f)?
            .write_arg(&self.body, source)?
            .finish()
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
        LispCall::new(&self.dot, source, f)?
            .write_arg((self.name, TokenKind::Ident), source)?
            .finish()
    }
}

impl LispDisplay for ast::DoneStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw, source, f)?
            .write_arg_iter(self.label.as_ref(), source)?
            .write_arg_iter(self.value.as_ref(), source)?
            .finish()
    }
}

impl LispDisplay for ast::SkipStmt {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw, source, f)?
            .write_arg_iter(self.label.as_ref(), source)?
            .finish()
    }
}

impl LispDisplay for ast::LoopBlock {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = LispCall::new(&self.kw, source, f)?;
        f.write_arg_iter(self.label.as_ref(), source)?;
        match &self.kind {
            ast::LoopKind::Loop | ast::LoopKind::Cord => f.write_arg(&self.block, source)?,

            ast::LoopKind::Rep { pred } => {
                f.write_arg(pred, source)?.write_arg(&self.block, source)?
            }

            ast::LoopKind::For { bind, in_kw, iter } => {
                f.call(in_kw, source)?
                    .write_arg(bind, source)?
                    .write_arg(iter, source)?
                    .finish()?;
                f.write_arg(&self.block, source)?
            }
        }
        .finish()
    }
}

impl LispDisplay for ast::Stmt {
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
        (self.ident, TokenKind::Ident).fmt_value(source, f)
    }
}

impl LispDisplay for ast::DeferParam {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.param.fmt(source, f)
    }
}

impl LispDisplay for ast::DeclVar {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.kw_span, source, f)?
            .write_arg(&self.binding, source)?
            .write_arg_iter(self.ty.as_ref(), source)?
            // assignment should be handled by the parent Block
            .finish()
    }
}

impl LispDisplay for ast::TypeAnnotation {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ty.fmt_value(source, f)
    }
}

impl LispDisplay for ast::PrimitiveType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.span, TokenKind::from(self.ty.to_keyword())).fmt_value(source, f)
    }
}

impl LispDisplay for ast::FallType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.pipe, source, f)?
            .write_arg(&*self.then_ty, source)?
            .write_arg(
                (self.ty_span, TokenKind::from(self.ty.to_keyword())),
                source,
            )?
            .finish()
    }
}

impl LispDisplay for ast::RefType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.sym, source, f)?
            .write_arg(&*self.ty, source)?
            .finish()
    }
}

impl LispDisplay for ast::ViewType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("view", source, f)?
            .write_arg(&*self.item, source)?
            .write_arg_iter(self.count.as_ref().map(|x| &x.count), source)?
            .finish()
    }
}

impl LispDisplay for ast::GroupedType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new("type_group", source, f)?
            .write_arg(&self.inner, source)?
            .finish()
    }
}

impl LispDisplay for ast::AssocTypeSource {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple(inner) => inner.fmt(source, f),
            Self::Grouped(inner) => inner.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::AssocType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LispCall::new(&self.sep, source, f)?
            .write_arg(&self.base, source)?
            .write_arg(&self.assoc, source)?
            .finish()
    }
}

impl LispDisplay for ast::FunType {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = LispCall::new(&self.kw, source, f)?;
        if let Some(ret) = self.ret.as_deref() {
            // TBD: what about the arrow?
            f.write_arg(&ret.ty, source)?;
        }
        f.call("params", source)?
            .write_arg_iter(self.args.as_slice(), source)?
            .finish()?;
        f.finish()
    }
}

impl LispDisplay for ast::TypeExpr {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(inner) => inner.fmt(source, f),
            Self::Instance(inner) => inner.fmt(source, f),
            Self::Fallible(inner) => inner.fmt(source, f),
            Self::Ref(inner) => inner.fmt(source, f),
            Self::View(inner) => inner.fmt(source, f),
            Self::Assoc(inner) => inner.fmt(source, f),
            Self::FunPtr(inner) => inner.fmt(source, f),
        }
    }
}

impl LispDisplay for ast::TypeInstance {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(args) = self.args.as_ref() {
            LispCall::new("generic", source, f)?
                .write_arg(&self.name, source)?
                .write_arg_iter(args.list.as_slice(), source)?
                .finish()
        } else {
            self.name.fmt(source, f)
        }
    }
}

impl LispDisplay for ast::Typename {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = (self.name, TokenKind::Ident);
        if self.path.is_empty() {
            LispCall::new("::", source, f)?
                .write_arg_iter(&self.path, source)?
                .write_arg(name, source)?
                .finish()
        } else {
            name.fmt_value(source, f)
        }
    }
}

impl LispDisplay for ast::NamespacePathPart {
    fn fmt(&self, source: &SourceCode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.name, TokenKind::Ident).fmt_value(source, f)
    }
}
