use crate::{
    analyzer::static_env::{
        ItemId,
        scope::{
            env::ScopeEnvironment,
            hir::{self, HirUnit},
        },
    },
    lex::token::{
        keyword::Keyword,
        punc::{OpSide, Punctuation},
    },
    parse::ast,
    src_code::code::SourceCode,
};
use std::fmt;

mod helpers {
    use super::HirDisplay;
    use crate::{analyzer::static_env::scope::env::ScopeEnvironment, src_code::code::SourceCode};
    use std::fmt::{self, Write};

    /// [`std::fmt::PadAdapter`] is private :c
    pub struct Indented<'buf> {
        buf: &'buf mut (dyn 'buf + fmt::Write),
        on_newline: bool,
    }

    impl<'buf> Indented<'buf> {
        const INDENT: &str = "  ";

        fn wrap<'slot, 'fmt: 'buf + 'slot>(
            f: &'fmt mut fmt::Formatter<'_>,
            slot: &'slot mut Option<Self>,
        ) -> fmt::Formatter<'slot> {
            fmt::FormattingOptions::new()
                .alternate(f.alternate()) // <- the only option that's important in this situation
                .create_formatter(slot.insert(Self {
                    buf: f,
                    on_newline: true,
                }))
        }
    }

    impl Write for Indented<'_> {
        // while this function looks exactly identical to the one in the standard library,
        // I actually wrote it BEFORE checking the implementation and I just happened to "convergent evolution" the same definition.
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for line in s.split_inclusive('\n') {
                if self.on_newline {
                    self.buf.write_str(Self::INDENT)?;
                }
                self.on_newline = line.ends_with('\n');
                self.buf.write_str(line)?;
            }
            Ok(())
        }

        // this one I wrote after finding the implementation, and it's basically copy-pasted
        fn write_char(&mut self, c: char) -> fmt::Result {
            if self.on_newline {
                self.buf.write_str(Self::INDENT)?;
            }
            self.on_newline = c == '\n';
            self.buf.write_char(c)
        }
    }

    pub struct HirBlock<'fmt, 'buf> {
        f: &'fmt mut fmt::Formatter<'buf>,
        res: fmt::Result,
        has_prev: bool,
    }

    impl<'fmt, 'buf> HirBlock<'fmt, 'buf> {
        pub fn new(f: &'fmt mut fmt::Formatter<'buf>) -> Self {
            Self {
                res: f.write_str("{"),
                f,
                has_prev: false,
            }
        }

        pub fn finish(&mut self) -> fmt::Result {
            self.res.and_then(|()| self.f.write_str("}"))
        }

        pub fn entry_with<F>(
            &mut self,
            fun: F,
            source: &SourceCode,
            env: &ScopeEnvironment<'_>,
        ) -> &mut Self
        where
            F: FnOnce(&SourceCode, &ScopeEnvironment<'_>, &mut fmt::Formatter<'_>) -> fmt::Result,
        {
            self.res = self.res.and_then(|()| {
                if self.f.alternate() {
                    if !self.has_prev {
                        self.f.write_str("\n")?;
                    }
                    let mut slot = None;
                    let mut writer = Indented::wrap(self.f, &mut slot);
                    fun(source, env, &mut writer)?;
                    self.f.write_str("\n")
                } else {
                    if self.has_prev {
                        self.f.write_str(" ")?;
                    }
                    fun(source, env, &mut *self.f)
                }
            });
            self.has_prev = true;
            self
        }

        pub fn entry<D>(
            &mut self,
            item: &D,
            source: &SourceCode,
            env: &ScopeEnvironment<'_>,
        ) -> &mut Self
        where
            D: ?Sized + HirDisplay,
        {
            self.entry_with(move |source, env, f| item.fmt(source, env, f), source, env)
        }

        pub fn entries<D, I>(
            &mut self,
            items: I,
            source: &SourceCode,
            env: &ScopeEnvironment<'_>,
        ) -> &mut Self
        where
            D: HirDisplay,
            I: IntoIterator<Item = D>,
        {
            for item in items {
                self.entry(&item, source, env);
            }
            self
        }
    }
}
use helpers::*;

pub trait HirDisplay {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;
}

impl<T: ?Sized + HirDisplay> HirDisplay for &T {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        (*self).fmt(source, env, f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayHir<'env, 'hir, 'src> {
    pub code: &'src SourceCode,
    pub env: &'env ScopeEnvironment<'src>,
    pub hir: &'hir HirUnit,
}

impl fmt::Display for DisplayHir<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.hir.fmt(self.code, self.env, f)
    }
}

impl HirDisplay for ItemId {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let item = env.get_decl(*self).expect("HIR IDs should be valid");
        write!(f, "{}#[{}]", &source[item.decl.identifier()], self)
    }
}

impl HirDisplay for ast::TypeExpr {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ast::TypeExpr::Primitive(inner) => inner.fmt(source, env, f),
            ast::TypeExpr::Instance(inner) => inner.fmt(source, env, f),
            ast::TypeExpr::Fallible(inner) => inner.fmt(source, env, f),
            ast::TypeExpr::Ref(inner) => inner.fmt(source, env, f),
            ast::TypeExpr::View(inner) => inner.fmt(source, env, f),
            ast::TypeExpr::Assoc(inner) => inner.fmt(source, env, f),
            ast::TypeExpr::FunPtr(inner) => inner.fmt(source, env, f),
        }
    }
}

impl HirDisplay for ast::PrimitiveType {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.ty.as_str())
    }
}

impl HirDisplay for ast::Typename {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        for part in &self.path {
            write!(f, "{}::", &source[part.name])?;
        }
        f.write_str(&source[self.name])
    }
}

impl HirDisplay for ast::TypeInstance {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.name.fmt(source, env, f)?;
        if let Some(args) = &self.args {
            f.write_str("<")?;
            for arg in &args.list {
                arg.fmt(source, env, f)?;
                f.write_str(",")?;
            }
            f.write_str(">")?;
        }
        Ok(())
    }
}

impl HirDisplay for ast::FallType {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.then_ty.fmt(source, env, f)?;
        write!(f, " | ")?;
        fmt::Display::fmt(self.ty.as_str(), f)
    }
}

impl HirDisplay for ast::RefType {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str("@")?;
        self.ty.fmt(source, env, f)
    }
}

impl HirDisplay for ast::ViewType {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str("[")?;
        self.item.fmt(source, env, f)?;
        if let Some(count) = &self.count {
            f.write_str("; ")?;
            todo!("expr display")
        }
        f.write_str("]")
    }
}

impl HirDisplay for ast::AssocType {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.base.fmt(source, env, f)?;
        f.write_str(Punctuation::PathSep.as_str())?;
        self.assoc.fmt(source, env, f)
    }
}

impl HirDisplay for ast::AssocTypeSource {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Simple(inner) => inner.fmt(source, env, f),

            Self::Grouped(inner) => {
                f.write_str("<")?;
                inner.inner.fmt(source, env, f)?;
                f.write_str(">")
            }
        }
    }
}

impl HirDisplay for hir::HirUnit {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        for decl in &self.decls {
            decl.fmt(source, env, f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl HirDisplay for ast::FunType {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}(", Keyword::Fun)?;
        let mut has_prev = false;
        for arg in &self.params {
            if has_prev {
                f.write_str(", ")?;
            } else {
                has_prev = true;
            }
            arg.fmt(source, env, f)?;
        }
        f.write_str(")")?;
        if let Some(ret) = &self.ret {
            write!(f, " {} ", Punctuation::Arrow)?;
            ret.ty.fmt(source, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::Decl {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Decl::Rec(inner) => inner.fmt(source, env, f),
            hir::Decl::Cat(inner) => inner.fmt(source, env, f),
            hir::Decl::Uni(inner) => inner.fmt(source, env, f),
            hir::Decl::Let(inner) => inner.fmt(source, env, f),
            hir::Decl::Pvt(inner) => inner.fmt(source, env, f),
            hir::Decl::Defer(inner) => inner.fmt(source, env, f),
            hir::Decl::Stmt(inner) => inner.fmt(source, env, f),
        }
    }
}

impl HirDisplay for hir::DeclRec {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Rec)?;
        self.id.fmt(source, env, f)?;
        f.write_str(" ")?;
        HirBlock::new(f).entries(&self.fields, source, env).finish()
    }
}

impl HirDisplay for hir::Field {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.id.fmt(source, env, f)?;
        f.write_str(": ")?;
        self.ty.fmt(source, env, f)?;
        f.write_str(",")
    }
}

impl HirDisplay for hir::DeclCat {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Cat)?;
        self.id.fmt(source, env, f)?;
        HirBlock::new(f)
            .entries(&self.discriminants, source, env)
            .finish()
    }
}

impl HirDisplay for hir::Discriminant {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.id.fmt(source, env, f)?;
        self.val.fmt(source, env, f)
    }
}

impl HirDisplay for hir::DeclUni {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Uni)?;
        self.id.fmt(source, env, f)
    }
}

impl HirDisplay for hir::DeclLet {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Let)?;
        self.id.fmt(source, env, f)?;
        f.write_str(";")
    }
}

impl HirDisplay for hir::DeclPvt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Pvt)?;
        self.id.fmt(source, env, f)?;
        f.write_str(";")
    }
}

impl HirDisplay for hir::DeclDefer {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // TODO: param
        self.on_pop.fmt(source, env, f)
    }
}

impl HirDisplay for hir::Stmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Stmt::Expr(inner) => inner.fmt(source, env, f),
            hir::Stmt::Print(inner) => inner.fmt(source, env, f),
            hir::Stmt::If(inner) => inner.fmt(source, env, f),
            hir::Stmt::Pick(inner) => inner.fmt(source, env, f),
            hir::Stmt::Loop(inner) => inner.fmt(source, env, f),
            hir::Stmt::Cord(inner) => inner.fmt(source, env, f),
            hir::Stmt::Rep(inner) => inner.fmt(source, env, f),
            hir::Stmt::For(inner) => inner.fmt(source, env, f),
            hir::Stmt::Done(inner) => inner.fmt(source, env, f),
            hir::Stmt::Skip(inner) => inner.fmt(source, env, f),
            hir::Stmt::Block(inner) => inner.fmt(source, env, f),
        }
    }
}

impl HirDisplay for hir::ExprStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.expr.fmt(source, env, f)?;
        f.write_str(";")
    }
}

impl HirDisplay for hir::Expr {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Expr::Block(inner) => inner.fmt(source, env, f),
            hir::Expr::Binary(inner) => inner.fmt(source, env, f),
            hir::Expr::Unary(inner) => inner.fmt(source, env, f),
            hir::Expr::Literal(inner) => inner.fmt(source, env, f),
            hir::Expr::Directive(inner) => inner.fmt(source, env, f),
            hir::Expr::Variable(inner) => inner.fmt(source, env, f),
        }
    }
}

impl HirDisplay for hir::BlockExpr {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::BlockExpr::If(inner) => inner.fmt(source, env, f),
            hir::BlockExpr::Loop(inner) => inner.fmt(source, env, f),
            hir::BlockExpr::Cord(inner) => inner.fmt(source, env, f),
            hir::BlockExpr::Block(inner) => inner.fmt(source, env, f),
        }
    }
}

impl HirDisplay for hir::Binary {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str("(")?;
        self.lhs.fmt(source, env, f)?;
        write!(f, " {} ", self.op)?;
        self.rhs.fmt(source, env, f)?;
        f.write_str(")")
    }
}

impl HirDisplay for hir::Unary {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str("(")?;
        match self.side {
            OpSide::Postfix => {
                self.operand.fmt(source, env, f)?;
                fmt::Display::fmt(self.op.as_str(), f)?;
            }
            OpSide::Prefix => {
                fmt::Display::fmt(self.op.as_str(), f)?;
                self.operand.fmt(source, env, f)?;
            }
        }
        f.write_str(")")
    }
}

impl HirDisplay for ast::Literal {
    fn fmt(
        &self,
        source: &SourceCode,
        _env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}#[{} literal]",
            &source[self.span],
            match self.kind {
                ast::LiteralKind::None => "none",
                ast::LiteralKind::Bool(_) => "bool",
                ast::LiteralKind::Number => "number",
                ast::LiteralKind::Char => "char",
                ast::LiteralKind::Text => "text",
                ast::LiteralKind::Macro => "macro",
            }
        )
    }
}

impl HirDisplay for hir::Directive {
    fn fmt(
        &self,
        _source: &SourceCode,
        _env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Directive::Inpt => f.write_str(Keyword::Inpt.as_str()),
        }
    }
}

impl HirDisplay for hir::Variable {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.id.fmt(source, env, f)
    }
}

impl HirDisplay for hir::PrintStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}(", Keyword::Prnt)?;
        self.expr.fmt(source, env, f)?;
        f.write_str(");")
    }
}

impl HirDisplay for hir::IfStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::If)?;
        self.pred.fmt(source, env, f)?;
        f.write_str(" ")?;
        self.then.fmt(source, env, f)?;
        if let Some(or) = self.or.as_deref() {
            write!(f, " {} ", Keyword::Or)?;
            or.fmt(source, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::OrStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::OrStmt::More(inner) => inner.fmt(source, env, f),
            hir::OrStmt::Last(inner) => inner.fmt(source, env, f),
        }
    }
}

impl HirDisplay for hir::PickExpr {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Pick)?;
        HirBlock::new(f).entries(&self.arms, source, env).finish()
    }
}

impl HirDisplay for hir::PickArm {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.pat.fmt(source, env, f)?;
        write!(f, " {} ", Punctuation::FatArrow)?;
        self.expr.fmt(source, env, f)
    }
}

impl HirDisplay for hir::PickPattern {
    fn fmt(
        &self,
        _source: &SourceCode,
        _env: &ScopeEnvironment<'_>,
        _f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        todo!("pick pattern")
    }
}

impl HirDisplay for hir::LoopStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Loop)?;
        self.body.fmt(source, env, f)
    }
}

impl HirDisplay for hir::CordStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Cord)?;
        self.body.fmt(source, env, f)
    }
}

impl HirDisplay for hir::RepStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::Rep)?;
        self.pred.fmt(source, env, f)?;
        self.body.fmt(source, env, f)
    }
}

impl HirDisplay for hir::BindingPattern {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.var.fmt(source, env, f)
    }
}

impl HirDisplay for hir::ForStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ", Keyword::For)?;
        self.var.fmt(source, env, f)?;
        write!(f, " {} ", Keyword::In)?;
        self.iter.fmt(source, env, f)?;
        self.body.fmt(source, env, f)
    }
}

impl HirDisplay for hir::Label {
    fn fmt(
        &self,
        source: &SourceCode,
        _env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, ".{}", &source[self.name])
    }
}

impl HirDisplay for hir::DoneStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", Keyword::Done)?;
        if let Some(label) = &self.label {
            f.write_str(" ")?;
            label.fmt(source, env, f)?;
        }
        if let Some(expr) = &self.expr {
            f.write_str(" ")?;
            expr.fmt(source, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::SkipStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", Keyword::Skip)?;
        if let Some(label) = &self.label {
            f.write_str(" ")?;
            label.fmt(source, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::BlockStmt {
    fn fmt(
        &self,
        source: &SourceCode,
        env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        HirBlock::new(f).entries(&self.decls, source, env).finish()
    }
}
