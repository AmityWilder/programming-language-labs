use crate::{
    analyzer::static_env::{
        ItemId,
        scope::{
            env::{self, ScopeEnvironment},
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
    use std::fmt;

    pub struct HirBlock<'fmt, 'buf> {
        f: &'fmt mut fmt::Formatter<'buf>,
        res: fmt::Result,
        has_decls: bool,
    }

    impl<'fmt, 'buf> HirBlock<'fmt, 'buf> {
        pub fn new(f: &'fmt mut fmt::Formatter<'buf>) -> Self {
            Self {
                res: f.write_str("{"),
                f,
                has_decls: false,
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
                    if !self.has_decls {
                        self.f.write_str("\n")?;
                    }
                    // TODO: indent
                    fun(source, env, &mut *self.f)?;
                    self.f.write_str("\n")
                } else {
                    if self.has_decls {
                        self.f.write_str(" ")?;
                    }
                    fun(source, env, &mut *self.f)
                }
            });
            self.has_decls = true;
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
            for decl in items {
                self.entry(&decl, source, env);
            }
            self
        }

        pub fn entries_with<F, I>(
            &mut self,
            items: I,
            source: &SourceCode,
            env: &ScopeEnvironment<'_>,
        ) -> &mut Self
        where
            F: FnOnce(&SourceCode, &ScopeEnvironment<'_>, &mut fmt::Formatter<'_>) -> fmt::Result,
            I: IntoIterator<Item = F>,
        {
            for fun in items {
                self.entry_with(fun, source, env);
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
        write!(f, "{}{{{}}}", &source[item.decl.identifier()], self)
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
        HirBlock::new(f)
            .entries_with(
                self.fields.iter().map(|field| {
                    |source: &SourceCode, env: &ScopeEnvironment<'_>, f: &mut fmt::Formatter<'_>| {
                        field.id.fmt(source, env, f)?;
                        f.write_str(": ")?;
                        field.ty.fmt(source, env, f)?;
                        f.write_str(",")
                    }
                }),
                source,
                env,
            )
            .finish()
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

impl HirDisplay for hir::Literal {
    fn fmt(
        &self,
        _source: &SourceCode,
        _env: &ScopeEnvironment<'_>,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "<literal>") // TODO
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
        self.then.fmt(source, env, f)?;
        if let Some(or) = self.or.as_deref() {
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
