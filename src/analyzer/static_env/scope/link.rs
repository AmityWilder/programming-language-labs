#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    reason = "need to be aware of when the structure changes so its formatting can update"
)]

use crate::{
    analyzer::static_env::scope::{env::ModScopeEnvironment, hir},
    error::{CheckDiagnostics, Diagnostic, DiagnosticKind},
    parse::ast::{self, AstNode as _},
    src_code::{map::SourceMap, span::FileId},
    util::OrAllErrsExt as _,
};

// bindings are fontloaded in this module so that all errors can all be found before returning with failure

impl ModScopeEnvironment<'_> {
    pub fn link(
        &mut self,
        unit: hir::HirUnit<hir::UnlinkedVar>,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::HirUnit<hir::LinkedVar>, CheckDiagnostics> {
        unit.decls
            .into_iter()
            .map(|decl| decl.link(self, unit.file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(|decls| hir::HirUnit {
                file: unit.file,
                decls,
            })
    }
}

trait LinkAnalyze {
    type Output;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics>;
}

// ! THE IMPORTANT ONE !
impl LinkAnalyze for hir::UnlinkedVar {
    type Output = hir::LinkedVar;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self(var) = self;
        let full_span = var.span().within_file(file);
        let ast::Variable { path, ident } = var;
        // non-empty
        if let Some(path @ [_, ..]) = path {
            _ = path;
            Ok(todo!("external variable linking"))
        } else {
            let ident = ident.within_file(file);
            let name = &source[ident];
            env.lookup_var(name)
                // variables can shadow function names
                .or_else(|| env.lookup_item(name))
                .map(hir::LinkedVar)
                .ok_or_else(|| {
                    diagnostics.push(Diagnostic::new(full_span, DiagnosticKind::UnknownVar));
                    CheckDiagnostics
                })
        }
    }
}

impl LinkAnalyze for hir::BlockStmt<hir::UnlinkedVar> {
    type Output = hir::BlockStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .into_iter()
            .map(|decl| decl.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice)
            .map(hir::BlockStmt)
    }
}

impl LinkAnalyze for hir::Decl<hir::UnlinkedVar> {
    type Output = hir::Decl<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Mod(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::On(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::On),

            Self::Fun(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Fun),

            Self::Rec(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Rec),

            Self::Cat(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Cat),

            Self::Uni(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Uni),

            Self::Let(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Let),

            Self::Pvt(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Pvt),

            Self::Dfer(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Dfer),

            Self::Stmt(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Stmt),
        }
    }
}

impl LinkAnalyze for hir::DeclMod {
    type Output = hir::DeclMod;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(self)
    }
}

impl LinkAnalyze for hir::DeclLet<hir::UnlinkedVar> {
    type Output = hir::DeclLet<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty, init } = self;
        let ty = ty
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let init = init
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DeclLet {
            id,
            ty: ty?,
            init: init?,
        })
    }
}

impl LinkAnalyze for hir::DeclUni<hir::UnlinkedVar> {
    type Output = hir::DeclUni<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty, init } = self;
        let ty = ty
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let init = init
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DeclUni {
            id,
            ty: ty?,
            init: init?,
        })
    }
}

impl LinkAnalyze for hir::DeclPvt<hir::UnlinkedVar> {
    type Output = hir::DeclPvt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty, init } = self;
        let ty = ty
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let init = init.link(env, file, source, diagnostics);
        Ok(hir::DeclPvt {
            id,
            ty: ty?,
            init: init?,
        })
    }
}

impl LinkAnalyze for hir::DeclOn<hir::UnlinkedVar> {
    type Output = hir::DeclOn<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self {
            generics,
            target,
            iff,
            members,
        } = self;
        let generics = generics
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let target = target.link(env, file, source, diagnostics);
        let iff = iff
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let members = members
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice);
        Ok(hir::DeclOn {
            generics: generics?,
            target: target?,
            iff: iff?,
            members: members?,
        })
    }
}

impl LinkAnalyze for hir::TypeBoundClause<hir::UnlinkedVar> {
    type Output = hir::TypeBoundClause<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { target, bound } = self;
        let target = target.link(env, file, source, diagnostics);
        let bound = bound.link(env, file, source, diagnostics);
        Ok(hir::TypeBoundClause {
            target: target?,
            bound: bound?,
        })
    }
}

impl LinkAnalyze for hir::IffClause<hir::UnlinkedVar> {
    type Output = hir::IffClause<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice)
            .map(hir::IffClause)
    }
}

impl LinkAnalyze for hir::SelfParam {
    type Output = hir::SelfParam;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(self)
    }
}

impl LinkAnalyze for hir::FunParam<hir::UnlinkedVar> {
    type Output = hir::FunParam<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty } = self;
        let ty = ty.link(env, file, source, diagnostics);
        Ok(hir::FunParam { id, ty: ty? })
    }
}

impl LinkAnalyze for hir::DeclFun<hir::UnlinkedVar> {
    type Output = hir::DeclFun<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self {
            id,
            generics,
            self_param,
            params,
            ret,
            iff,
            body,
        } = self;
        let generics = generics
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let self_param = self_param
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let params = params
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice);
        let ret = ret
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let iff = iff
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let body = body
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DeclFun {
            id,
            generics: generics?,
            self_param: self_param?,
            params: params?,
            ret: ret?,
            iff: iff?,
            body: body?,
        })
    }
}

impl LinkAnalyze for hir::Field<hir::UnlinkedVar> {
    type Output = hir::Field<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty } = self;
        let ty = ty.link(env, file, source, diagnostics);
        Ok(hir::Field { id, ty: ty? })
    }
}

impl LinkAnalyze for hir::DeclRec<hir::UnlinkedVar> {
    type Output = hir::DeclRec<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        // TODO: generics
        let Self { id, fields } = self;
        let fields = fields
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice);
        Ok(hir::DeclRec {
            id,
            fields: fields?,
        })
    }
}

impl LinkAnalyze for hir::DeclCat<hir::UnlinkedVar> {
    type Output = hir::DeclCat<hir::LinkedVar>;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, discriminants } = self;
        let discriminants = discriminants
            .into_iter()
            .map(|_disc| todo!("cat discriminant analysis"))
            .collect();
        Ok(hir::DeclCat { id, discriminants })
    }
}

impl LinkAnalyze for hir::DeclDfer<hir::UnlinkedVar> {
    type Output = hir::DeclDfer<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { param, on_pop } = self;
        let on_pop = on_pop.link(env, file, source, diagnostics);
        Ok(hir::DeclDfer {
            param,
            on_pop: on_pop?,
        })
    }
}

impl LinkAnalyze for hir::Stmt<hir::UnlinkedVar> {
    type Output = hir::Stmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Expr(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Expr),

            Self::Print(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Print),

            Self::If(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::If),

            Self::Pick(_inner) => todo!("pick statement linking"),

            Self::Loop(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Loop),

            Self::Cord(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Cord),

            Self::Rep(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Rep),

            Self::For(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::For),

            Self::Give(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Give),

            Self::Fail(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Fail),

            Self::Done(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Done),

            Self::Skip(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Skip),

            Self::Block(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Block),
        }
    }
}

impl LinkAnalyze for hir::LoopStmt<hir::UnlinkedVar> {
    type Output = hir::LoopStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, body } = self;
        let body = body.link(env, file, source, diagnostics);
        Ok(hir::LoopStmt { label, body: body? })
    }
}

impl LinkAnalyze for hir::CordStmt<hir::UnlinkedVar> {
    type Output = hir::CordStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, body } = self;
        let body = body.link(env, file, source, diagnostics);
        Ok(hir::CordStmt { label, body: body? })
    }
}

impl LinkAnalyze for hir::RepStmt<hir::UnlinkedVar> {
    type Output = hir::RepStmt<hir::LinkedVar>;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        todo!()
    }
}

impl LinkAnalyze for hir::ForStmt<hir::UnlinkedVar> {
    type Output = hir::ForStmt<hir::LinkedVar>;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        todo!()
    }
}

impl LinkAnalyze for hir::ExprStmt<hir::UnlinkedVar> {
    type Output = hir::ExprStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .link(env, file, source, diagnostics)
            .map(hir::ExprStmt)
    }
}

impl LinkAnalyze for hir::PrintStmt<hir::UnlinkedVar> {
    type Output = hir::PrintStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .link(env, file, source, diagnostics)
            .map(hir::PrintStmt)
    }
}

impl LinkAnalyze for hir::GiveStmt<hir::UnlinkedVar> {
    type Output = hir::GiveStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.expr
            .map(|expr| expr.link(env, file, source, diagnostics))
            .transpose()
            .map(|expr| hir::GiveStmt { expr })
    }
}

impl LinkAnalyze for hir::FailStmt<hir::UnlinkedVar> {
    type Output = hir::FailStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.expr
            .map(|expr| expr.link(env, file, source, diagnostics))
            .transpose()
            .map(|expr| hir::FailStmt { expr })
    }
}

impl LinkAnalyze for hir::DoneStmt<hir::UnlinkedVar> {
    type Output = hir::DoneStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, expr } = self;
        let expr = expr
            .map(|expr| expr.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DoneStmt { label, expr: expr? })
    }
}

impl LinkAnalyze for hir::SkipStmt {
    type Output = hir::SkipStmt;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(self)
    }
}

impl LinkAnalyze for hir::Expr<hir::UnlinkedVar> {
    type Output = hir::Expr<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Block(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Expr::Block),

            Self::Binary(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::Expr::Binary(Box::new(x))),

            Self::Unary(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::Expr::Unary(Box::new(x))),

            Self::Index(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::Expr::Index(Box::new(x))),

            Self::Call(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::Expr::Call(Box::new(x))),

            Self::Literal(inner) => Ok(hir::Expr::Literal(inner)),

            Self::Directive(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Expr::Directive),

            Self::Variable(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Expr::Variable),
        }
    }
}

impl LinkAnalyze for hir::BlockExpr<hir::UnlinkedVar> {
    type Output = hir::BlockExpr<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::If(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::BlockExpr::If(Box::new(x))),

            Self::Loop(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::BlockExpr::Loop(Box::new(x))),

            Self::Cord(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::BlockExpr::Cord(Box::new(x))),

            Self::Block(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::BlockExpr::Block),
        }
    }
}

impl LinkAnalyze for hir::Directive {
    type Output = hir::Directive;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(self)
    }
}

impl LinkAnalyze for hir::IfStmt<hir::UnlinkedVar> {
    type Output = hir::IfStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let pred = self.pred.link(env, file, source, diagnostics);
        let then = self.then.link(env, file, source, diagnostics);
        let or = self
            .or
            .map(|or| or.link(env, file, source, diagnostics).map(Box::new))
            .transpose();
        Ok(hir::IfStmt {
            pred: pred?,
            then: then?,
            or: or?,
        })
    }
}

impl LinkAnalyze for hir::OrStmt<hir::UnlinkedVar> {
    type Output = hir::OrStmt<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::More(stmt) => stmt
                .link(env, file, source, diagnostics)
                .map(hir::OrStmt::More),

            Self::Last(block) => block
                .link(env, file, source, diagnostics)
                .map(hir::OrStmt::Last),
        }
    }
}

impl LinkAnalyze for hir::TypeExpr<hir::UnlinkedVar> {
    type Output = hir::TypeExpr<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Primitive(x) => Ok(hir::TypeExpr::Primitive(x)),

            Self::Instance(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeExpr::Instance),

            Self::Fallible(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Fallible(Box::new(x))),

            Self::Ref(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Ref(Box::new(x))),

            Self::View(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::View(Box::new(x))),

            Self::Assoc(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Assoc(Box::new(x))),

            Self::FunPtr(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeExpr::FunPtr),

            Self::Opaque(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Opaque(Box::new(x))),
        }
    }
}

impl LinkAnalyze for hir::FallType<hir::UnlinkedVar> {
    type Output = hir::FallType<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { then_ty, ty } = self;
        let then_ty = then_ty.link(env, file, source, diagnostics);
        Ok(hir::FallType {
            then_ty: then_ty?,
            ty,
        })
    }
}

impl LinkAnalyze for hir::ViewType<hir::UnlinkedVar> {
    type Output = hir::ViewType<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { item, count } = self;
        let item = item.link(env, file, source, diagnostics);
        let count = count
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::ViewType {
            item: item?,
            count: count?,
        })
    }
}

impl LinkAnalyze for hir::AssocType<hir::UnlinkedVar> {
    type Output = hir::AssocType<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let base = self.base.link(env, file, source, diagnostics);
        let assoc = self.assoc.link(env, file, source, diagnostics);
        Ok(hir::AssocType {
            base: base?,
            assoc: assoc?,
        })
    }
}

impl LinkAnalyze for hir::AssocTypeSource<hir::UnlinkedVar> {
    type Output = hir::AssocTypeSource<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Simple(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::AssocTypeSource::Simple),

            Self::Grouped(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::AssocTypeSource::Grouped),
        }
    }
}

impl LinkAnalyze for hir::OpaqueType<hir::UnlinkedVar> {
    type Output = hir::OpaqueType<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .link(env, file, source, diagnostics)
            .map(hir::OpaqueType)
    }
}

impl LinkAnalyze for hir::Typename {
    type Output = hir::Typename;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        // TODO: can the module name be located?
        Ok(hir::Typename {
            path: self
                .path
                .into_iter()
                .map(|part| hir::NamespacePathPart { name: part.name })
                .collect(),
            name: self.name,
        })
    }
}

impl LinkAnalyze for hir::FunType<hir::UnlinkedVar> {
    type Output = hir::FunType<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let params = self
            .params
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice);
        let ret = self
            .ret
            .map(|x| x.link(env, file, source, diagnostics).map(Box::new))
            .transpose();
        Ok(hir::FunType {
            params: params?,
            ret: ret?,
        })
    }
}

impl LinkAnalyze for hir::RefType<hir::UnlinkedVar> {
    type Output = hir::RefType<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(hir::RefType {
            ty: self.ty.link(env, file, source, diagnostics)?,
        })
    }
}

impl LinkAnalyze for hir::TypeInstance<hir::UnlinkedVar> {
    type Output = hir::TypeInstance<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let name = self.name.link(env, file, source, diagnostics);
        let args = self
            .args
            .map(|args| args.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::TypeInstance {
            name: name?,
            args: args?,
        })
    }
}

impl LinkAnalyze for hir::Binary<hir::UnlinkedVar> {
    type Output = hir::Binary<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let lhs = self.lhs.link(env, file, source, diagnostics);
        let rhs = self.rhs.link(env, file, source, diagnostics);
        Ok(hir::Binary {
            op: self.op,
            lhs: lhs?,
            rhs: rhs?,
        })
    }
}

impl LinkAnalyze for hir::Unary<hir::UnlinkedVar> {
    type Output = hir::Unary<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let operand = self.operand.link(env, file, source, diagnostics);
        Ok(hir::Unary {
            op: self.op,
            side: self.side,
            operand: operand?,
        })
    }
}

impl LinkAnalyze for hir::Subscript<hir::UnlinkedVar> {
    type Output = hir::Subscript<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { arr, idx } = self;
        let arr = arr.link(env, file, source, diagnostics);
        let idx = idx.link(env, file, source, diagnostics);
        Ok(hir::Subscript {
            arr: arr?,
            idx: idx?,
        })
    }
}

impl LinkAnalyze for hir::FnCall<hir::UnlinkedVar> {
    type Output = hir::FnCall<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let fun = self.fun.link(env, file, source, diagnostics);
        let args = self
            .args
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice);
        Ok(hir::FnCall {
            fun: fun?,
            args: args?,
        })
    }
}

impl LinkAnalyze for hir::GenericGroup<hir::UnlinkedVar> {
    type Output = hir::GenericGroup<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice)
            .map(hir::GenericGroup)
    }
}

impl LinkAnalyze for hir::TypeBound<hir::UnlinkedVar> {
    type Output = hir::TypeBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Conjunction(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeBound::Conjunction),

            Self::Disjunction(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeBound::Disjunction),

            Self::Selection(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeBound::Selection),

            Self::Not(inner) => inner
                .link(env, file, source, diagnostics)
                .map(Box::new)
                .map(hir::TypeBound::Not),

            Self::Exact(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeBound::Exact),

            Self::Has(inner) => inner
                .link(env, file, source, diagnostics)
                .map(Box::new)
                .map(hir::TypeBound::Has),

            Self::Shape(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeBound::Shape),

            Self::Supports(inner) => inner
                .link(env, file, source, diagnostics)
                .map(Box::new)
                .map(hir::TypeBound::Supports),

            Self::Satisfies(inner) => inner
                .link(env, file, source, diagnostics)
                .map(Box::new)
                .map(hir::TypeBound::Satisfies),
        }
    }
}

impl LinkAnalyze for hir::ConjunctionBound<hir::UnlinkedVar> {
    type Output = hir::ConjunctionBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice)
            .map(hir::ConjunctionBound)
    }
}

impl LinkAnalyze for hir::DisjunctionBound<hir::UnlinkedVar> {
    type Output = hir::DisjunctionBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice)
            .map(hir::DisjunctionBound)
    }
}

impl LinkAnalyze for hir::SelectionBound<hir::UnlinkedVar> {
    type Output = hir::SelectionBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_or_acc_errs::<_, CheckDiagnostics>()
            .map(Vec::into_boxed_slice)
            .map(hir::SelectionBound)
    }
}

impl LinkAnalyze for hir::NegativeBound<hir::UnlinkedVar> {
    type Output = hir::NegativeBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .link(env, file, source, diagnostics)
            .map(hir::NegativeBound)
    }
}

impl LinkAnalyze for hir::ExactTypeBound<hir::UnlinkedVar> {
    type Output = hir::ExactTypeBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .link(env, file, source, diagnostics)
            .map(hir::ExactTypeBound)
    }
}

impl LinkAnalyze for hir::MemberBound<hir::UnlinkedVar> {
    type Output = hir::MemberBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { member, req } = self;
        let req = req
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::MemberBound { member, req: req? })
    }
}

impl LinkAnalyze for hir::ShapeBound<hir::UnlinkedVar> {
    type Output = hir::ShapeBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.0
            .link(env, file, source, diagnostics)
            .map(hir::ShapeBound)
    }
}

impl LinkAnalyze for hir::OverloadBound {
    type Output = hir::OverloadBound;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(self)
    }
}

impl LinkAnalyze for hir::ExprBound<hir::UnlinkedVar> {
    type Output = hir::ExprBound<hir::LinkedVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let Self { param, expr } = self;
        let expr = expr.link(env, file, source, diagnostics);
        Ok(hir::ExprBound { param, expr: expr? })
    }
}
