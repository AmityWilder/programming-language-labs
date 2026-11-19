#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    reason = "need to be aware of when the structure changes so its formatting can update"
)]

use crate::{
    analyzer::static_env::scope::{env::ModScopeEnvironment, hir},
    error::{CheckDiagnostics, Diagnostic},
    src_code::{map::SourceMap, span::FileId},
    util::OrAllErrsExt as _,
};

// bindings are fontloaded in this module so that all errors can all be found before returning with failure

impl ModScopeEnvironment<'_> {
    pub fn link(
        &mut self,
        unit: hir::HirUnit<hir::PostDeclVar>,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::HirUnit<hir::PostDeclVar>, CheckDiagnostics> {
        unit.decls
            .into_iter()
            .map(|decl| decl.link(self, unit.file, source, diagnostics))
            .collect_or_drop_all_errs()
            .map(|decls| hir::HirUnit {
                file: unit.file,
                decls,
            })
            .ok_or(CheckDiagnostics)
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

impl LinkAnalyze for hir::BlockStmt<hir::Variable> {
    type Output = hir::BlockStmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.body
            .into_iter()
            .map(|decl| decl.link(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .map(hir::BlockStmt)
            .ok_or(CheckDiagnostics)
    }
}

impl LinkAnalyze for hir::Decl<hir::Variable> {
    type Output = hir::Decl<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Mod(decl_mod) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::On(decl_on) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Fun(decl_fun) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Rec(decl_rec) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Cat(decl_cat) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Uni(decl_uni) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Let(decl_let) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Pvt(decl_pvt) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Dfer(decl_dfer) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::Stmt(stmt) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

        }
    }
}

impl LinkAnalyze for hir::DeclMod<hir::Variable> {
    type Output = hir::DeclMod;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let id = env
            .find_submodule(&source[self.name.within_file(file)])
            .expect("should be handled in pre-scan");
        Ok(hir::DeclMod { id })
    }
}

impl LinkAnalyze for hir::DeclVar<hir::Variable> {
    type Output = hir::Decl<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        // definition should be BEFORE binding
        let init = self
            .assignment
            .clone()
            .map(|x| x.expr.link(env, file, source, diagnostics))
            .transpose();

        let ty = self
            .ty
            .clone()
            .map(|annotation| annotation.ty.link(env, file, source, diagnostics))
            .transpose();

        let kw = self.kw;

        let ident = self.binding.ident.within_file(file);
        let id = env.decl_var(source[ident].to_owned(), ident);
        Ok(match kw {
            ast::VarKeyword::Let => hir::Decl::Let(hir::DeclLet {
                id,
                ty: ty?,
                init: init?,
            }),

            ast::VarKeyword::Uni => hir::Decl::Uni(hir::DeclUni {
                id,
                ty: ty?,
                init: init?,
            }),

            ast::VarKeyword::Pvt => {
                let init = init.transpose().ok_or_else_flat(|| {
                    diagnostics.push(Diagnostic::new(
                        ident.within_file(file),
                        DiagnosticKind::Other {
                            severity: Severity::Error,
                            category: StaticAnalysis,
                            msg: "pivots must have an initializer".to_owned(),
                        },
                    ));
                    CheckDiagnostics
                });
                hir::Decl::Pvt(hir::DeclPvt {
                    id,
                    ty: ty?,
                    init: init?,
                })
            }
        })
    }
}

impl LinkAnalyze for hir::DeclOn<hir::Variable> {
    type Output = hir::DeclOn<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let generics = self
            .params
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let target = self.ty.link(env, file, source, diagnostics);
        let iff = self
            .iff
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let members = self
            .body
            .link(env, file, source, diagnostics)
            .map(|x| x.0);
        Ok(hir::DeclOn {
            generics: generics?,
            target: target?,
            iff: iff?,
            members: members?,
        })
    }
}

impl LinkAnalyze for hir::TypeBoundClause<hir::Variable> {
    type Output = hir::TypeBoundClause<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let target = self.ty.link(env, file, source, diagnostics);
        let bound = self.bound.link(env, file, source, diagnostics);
        Ok(hir::TypeBoundClause {
            target: target?,
            bound: bound?,
        })
    }
}

impl LinkAnalyze for hir::IffClause<hir::Variable> {
    type Output = hir::IffClause<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.bounds
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics)
            .map(hir::IffClause)
    }
}

impl LinkAnalyze for hir::SelfParam<hir::Variable> {
    type Output = hir::SelfParam;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let ident = self.span.within_file(file);
        debug_assert_eq!(&source[ident], Keyword::Me.as_str(), "ident should be `me`");
        let id = env.decl_var(Keyword::Me.to_string(), ident);
        Ok(hir::SelfParam { id })
    }
}

impl LinkAnalyze for hir::FnParam<hir::Variable> {
    type Output = hir::FunParam<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let ident = self.name.within_file(file);
        let id = env.decl_var(source[ident].to_owned(), ident);
        let ty = self.ty.ty.link(env, file, source, diagnostics);
        Ok(hir::FunParam { id, ty: ty? })
    }
}

impl LinkAnalyze for hir::DeclFun<hir::Variable> {
    type Output = hir::DeclFun<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let (head, body) = match self {
            Self::Declare(decl) => (decl.head, None),
            Self::Define(def) => (def.head, Some(def.body)),
        };
        let ident = head.name.within_file(file);
        let name = &source[ident];
        let id = env.decl_item(name.to_owned(), ident).map_err(|e| {
            diagnostics.push(Diagnostic::new(
                ident,
                DiagnosticKind::Other {
                    severity: Severity::Error,
                    category: StaticAnalysis,
                    msg: format!(
                        "duplicate function `{name}`: first definition:\x1b[0m {}",
                        env.get_decl(e.og)
                            .expect("duplicate item error should give a valid ID")
                            .decl
                            .display_loc(source)
                    ),
                },
            ));
            CheckDiagnostics
        });
        let generics = head
            .generics
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let self_param = head
            .self_param
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let params = head
            .params
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        let ret = head
            .ret
            .map(|x| x.ty.link(env, file, source, diagnostics))
            .transpose();
        let iff = head
            .iff
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        let body = body
            .map(|x| x.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DeclFun {
            id: id?,
            generics: generics?,
            self_param: self_param?,
            params: params?,
            ret: ret?,
            iff: iff?,
            body: body?,
        })
    }
}

impl LinkAnalyze for hir::DeclType<hir::Variable> {
    type Output = hir::Decl<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            ast::DeclType::Rec(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Rec),

            ast::DeclType::Cat(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Decl::Cat),
        }
    }
}

impl LinkAnalyze for (ast::Field, ItemId)<hir::Variable> {
    type Output = hir::Field<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let (field, parent) = self;
        let ident = field.name.within_file(file);
        let field_name = &source[ident];
        let ty = field.ty.ty.clone().link(env, file, source, diagnostics);
        let id = env
            .decl_member(parent, field_name.to_owned(), ident)
            .map_err(|e| {
                diagnostics.push(Diagnostic::new(
                    ident,
                    DiagnosticKind::Other {
                        severity: Severity::Error,
                        category: StaticAnalysis,
                        msg: format!("could not declare field {parent}::{field_name}: {e:?}"),
                    },
                ));
                CheckDiagnostics
            });
        Ok(hir::Field { id: id?, ty: ty? })
    }
}

impl LinkAnalyze for hir::DeclRec<hir::Variable> {
    type Output = hir::DeclRec<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        // TODO: generics
        let ident = self.name.within_file(file);
        let name = &source[ident];
        let id = env.decl_item(name.to_owned(), ident).map_err(|e| {
            diagnostics.push(Diagnostic::new(
                ident,
                DiagnosticKind::Other {
                    severity: Severity::Error,
                    category: StaticAnalysis,
                    msg: format!("could not declare rec {name}: {e:?}"),
                },
            ));
            CheckDiagnostics
        })?;
        let fields = self
            .body
            .fields
            .into_iter()
            .map(|x| (x, id).link(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        Ok(hir::DeclRec {
            id,
            fields: fields?,
        })
    }
}

impl LinkAnalyze for hir::DeclCat<hir::Variable> {
    type Output = hir::DeclCat<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let ident = self.name.within_file(file);
        let name = &source[ident];
        let id = env.decl_item(name.to_owned(), ident).map_err(|e| {
            diagnostics.push(Diagnostic::new(
                ident,
                DiagnosticKind::Other {
                    severity: Severity::Error,
                    category: StaticAnalysis,
                    msg: format!("{e:?}"),
                },
            ));
            CheckDiagnostics
        });
        let discriminants = self
            .discriminants
            .items
            .into_iter()
            .map(|_disc| todo!("cat discriminant analysis"))
            .collect();
        Ok(hir::DeclCat {
            id: id?,
            discriminants,
        })
    }
}

impl LinkAnalyze for hir::DeclDfer<hir::Variable> {
    type Output = hir::DeclDfer<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let param = self.param.map(|x| {
            let ident = x.param.ident.within_file(file);
            env.decl_var(source[ident].to_owned(), ident)
        });
        let on_pop = self.on_pop.link(env, file, source, diagnostics);
        Ok(hir::DeclDfer {
            param,
            on_pop: on_pop?,
        })
    }
}

impl LinkAnalyze for hir::Stmt<hir::Variable> {
    type Output = hir::Stmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            ast::Stmt::Expr(inner) => inner
                .expr
                .link(env, file, source, diagnostics)
                .map(|expr| hir::Stmt::Expr(hir::ExprStmt { expr })),

            ast::Stmt::Print(inner) => inner
                .group
                .expr
                .link(env, file, source, diagnostics)
                .map(|expr| hir::Stmt::Print(hir::PrintStmt { expr })),

            ast::Stmt::If(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::If),

            ast::Stmt::Loop(inner) => inner.link(env, file, source, diagnostics),

            ast::Stmt::Fail(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Fail),

            ast::Stmt::Give(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Give),

            // not sure if this is a good language feature
            ast::Stmt::Emit(_inner) => todo!("emit analysis"),

            ast::Stmt::Done(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Done),

            ast::Stmt::Skip(inner) => Ok(hir::Stmt::Skip(hir::SkipStmt {
                label: inner.label.map(|lbl| hir::Label { name: lbl.name }),
            })),

            ast::Stmt::Block(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Stmt::Block),
        }
    }
}

impl LinkAnalyze for hir::LoopBlock<hir::Variable> {
    type Output = hir::Stmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let label = self.label.map(|name| hir::Label {
            name: name.label.name,
        });
        let block = self.block.link(env, file, source, diagnostics);
        Ok(match self.kind {
            ast::LoopKind::Loop => hir::Stmt::Loop(hir::LoopStmt {
                label,
                body: block?,
            }),

            ast::LoopKind::Cord => hir::Stmt::Cord(hir::CordStmt {
                label,
                body: block?,
            }),

            ast::LoopKind::Rep { pred } => hir::Stmt::Rep(hir::RepStmt {
                pred: pred.link(env, file, source, diagnostics)?,
                body: block?,
            }),

            ast::LoopKind::For { bind, iter, .. } => {
                let ident = bind.ident.within_file(file);
                let name = &source[ident];
                let var = env.decl_var(name.to_owned(), ident);
                hir::Stmt::For(hir::ForStmt {
                    var: hir::BindingPattern { var },
                    iter: iter.link(env, file, source, diagnostics)?,
                    body: block?,
                })
            }
        })
    }
}

impl LinkAnalyze for hir::PrintStmt<hir::Variable> {
    type Output = hir::PrintStmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.group
            .expr
            .link(env, file, source, diagnostics)
            .map(|expr| hir::PrintStmt { expr })
    }
}

impl LinkAnalyze for hir::GiveStmt<hir::Variable> {
    type Output = hir::GiveStmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.value
            .map(|expr| expr.link(env, file, source, diagnostics))
            .transpose()
            .map(|expr| hir::GiveStmt { expr })
    }
}

impl LinkAnalyze for hir::FailStmt<hir::Variable> {
    type Output = hir::FailStmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.value
            .map(|expr| expr.link(env, file, source, diagnostics))
            .transpose()
            .map(|expr| hir::FailStmt { expr })
    }
}

impl LinkAnalyze for hir::DoneStmt<hir::Variable> {
    type Output = hir::DoneStmt<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let label = self.label.map(|lbl| hir::Label { name: lbl.name });
        let expr = self
            .value
            .map(|expr| expr.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DoneStmt { label, expr: expr? })
    }
}

impl LinkAnalyze for hir::Expr<hir::Variable> {
    type Output = hir::Expr<hir::PostDeclVar>;

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

            Self::Member(_inner) => todo!("member access analysis"),

            Self::Variable(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::Expr::Variable),

            Self::Grouping(inner) => inner.expr.link(env, file, source, diagnostics),

            Self::Type(_inner) => todo!("expr type expr analysis"),
        }
    }
}

impl LinkAnalyze for hir::BlockExpr<hir::Variable> {
    type Output = hir::BlockExpr<hir::PostDeclVar>;

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
                .map(|x| match x {
                    hir::Stmt::Loop(stmt) => hir::BlockExpr::Loop(Box::new(stmt)),
                    hir::Stmt::Cord(stmt) => hir::BlockExpr::Cord(Box::new(stmt)),

                    hir::Stmt::Rep(_) | hir::Stmt::For(_) => {
                        unimplemented!("ast::BlockExpr should not be Rep or For; found: {x:?}")
                    }

                    _ => unreachable!("loop analysis should only produce a loop"),
                }),

            Self::Block(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::BlockExpr::Block),
        }
    }
}

impl LinkAnalyze for hir::Directive<hir::Variable> {
    type Output = hir::Directive;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(match self.kw {
            ast::DirectiveKw::Inpt => hir::Directive::Inpt,
        })
    }
}

impl LinkAnalyze for hir::Variable<hir::Variable> {
    type Output = hir::Variable;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        if self.path.is_empty() {
            let ident = self.ident.within_file(file);
            let name = &source[ident];
            // variables can shadow function names
            let id = env
                .lookup_var(name)
                // maybe a function
                .or_else(|| env.lookup_item(name));
            if let Some(id) = id {
                return Ok(hir::Variable::Found(id));
            }
        }
        Ok(hir::Variable::Unknown(self))
    }
}

impl LinkAnalyze for hir::IfStmt<hir::Variable> {
    type Output = hir::IfStmt<hir::PostDeclVar>;

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
            .map(|or| {
                match or.body {
                    ast::OrBody::More(stmt) => stmt
                        .link(env, file, source, diagnostics)
                        .map(hir::OrStmt::More),

                    ast::OrBody::Last(block) => block
                        .link(env, file, source, diagnostics)
                        .map(hir::OrStmt::Last),
                }
                .map(Box::new)
            })
            .transpose();
        Ok(hir::IfStmt {
            pred: pred?,
            then: then?,
            or: or?,
        })
    }
}

impl LinkAnalyze for hir::TypeExpr<hir::Variable> {
    type Output = hir::TypeExpr<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            // TODO: hmm...
            ast::TypeExpr::Primitive(inner) => Ok(hir::TypeExpr::Primitive(match inner.ty {
                ast::TypenameKeyword::None => hir::PrimitiveType::None,
                ast::TypenameKeyword::Nevr => hir::PrimitiveType::Nevr,
                ast::TypenameKeyword::Fail => hir::PrimitiveType::Fail,
                ast::TypenameKeyword::Bool => hir::PrimitiveType::Bool,
                ast::TypenameKeyword::Byte => hir::PrimitiveType::Byte,
                ast::TypenameKeyword::Uint => hir::PrimitiveType::Uint,
                ast::TypenameKeyword::Sint => hir::PrimitiveType::Sint,
                ast::TypenameKeyword::Flpt => hir::PrimitiveType::Flpt,
                ast::TypenameKeyword::Char => hir::PrimitiveType::Char,
                ast::TypenameKeyword::Text => hir::PrimitiveType::Text,
            })),

            ast::TypeExpr::Instance(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeExpr::Instance),

            ast::TypeExpr::Fallible(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Fallible(Box::new(x))),

            ast::TypeExpr::Ref(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Ref(Box::new(x))),

            ast::TypeExpr::View(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::View(Box::new(x))),

            ast::TypeExpr::Assoc(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Assoc(Box::new(x))),

            ast::TypeExpr::FunPtr(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeExpr::FunPtr),

            ast::TypeExpr::Opaque(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Opaque(Box::new(x))),
        }
    }
}

impl LinkAnalyze for hir::FallType<hir::Variable> {
    type Output = hir::FallType<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let then_ty = self.then_ty.link(env, file, source, diagnostics);
        let ty = match self.ty {
            ast::OrTypeKeyword::None => hir::OrType::None,
            ast::OrTypeKeyword::Nevr => hir::OrType::Nevr,
            ast::OrTypeKeyword::Fail => hir::OrType::Fail,
        };
        Ok(hir::FallType {
            then_ty: then_ty?,
            ty,
        })
    }
}

impl LinkAnalyze for hir::ViewType<hir::Variable> {
    type Output = hir::ViewType<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let item = self.item.link(env, file, source, diagnostics);
        let count = self
            .count
            .map(|x| x.count.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::ViewType {
            item: item?,
            count: count?,
        })
    }
}

impl LinkAnalyze for hir::AssocType<hir::Variable> {
    type Output = hir::AssocType<hir::PostDeclVar>;

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

impl LinkAnalyze for hir::AssocTypeSource<hir::Variable> {
    type Output = hir::AssocTypeSource<hir::PostDeclVar>;

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
                .inner
                .link(env, file, source, diagnostics)
                .map(hir::AssocTypeSource::Grouped),
        }
    }
}

impl LinkAnalyze for hir::OpaqueType<hir::Variable> {
    type Output = hir::OpaqueType<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.bound
            .link(env, file, source, diagnostics)
            .map(hir::OpaqueType)
    }
}

impl LinkAnalyze for hir::Typename<hir::Variable> {
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

impl LinkAnalyze for hir::FunType<hir::Variable> {
    type Output = hir::FunType<hir::PostDeclVar>;

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
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        let ret = self
            .ret
            .map(|x| x.ty.link(env, file, source, diagnostics).map(Box::new))
            .transpose();
        Ok(hir::FunType {
            params: params?,
            ret: ret?,
        })
    }
}

impl LinkAnalyze for hir::RefType<hir::Variable> {
    type Output = hir::RefType<hir::PostDeclVar>;

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

impl LinkAnalyze for hir::TypeInstance<hir::Variable> {
    type Output = hir::TypeInstance<hir::PostDeclVar>;

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

impl LinkAnalyze for hir::Binary<hir::Variable> {
    type Output = hir::Binary<hir::PostDeclVar>;

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

impl LinkAnalyze for hir::Unary<hir::Variable> {
    type Output = hir::Unary<hir::PostDeclVar>;

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

impl LinkAnalyze for hir::Subscript<hir::Variable> {
    type Output = hir::Subscript<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let arr = self.arr.link(env, file, source, diagnostics);
        let idx = self.index.link(env, file, source, diagnostics);
        Ok(hir::Subscript {
            arr: arr?,
            idx: idx?,
        })
    }
}

impl LinkAnalyze for hir::FnCall<hir::Variable> {
    type Output = hir::FnCall<hir::PostDeclVar>;

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
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        Ok(hir::FnCall {
            fun: fun?,
            args: args?,
        })
    }
}

impl LinkAnalyze for hir::GenericGroup<hir::Variable> {
    type Output = hir::GenericGroup<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.list
            .into_iter()
            .map(|x| x.link(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics)
            .map(hir::GenericGroup)
    }
}

impl LinkAnalyze for hir::TypeBound<hir::Variable> {
    type Output = hir::TypeBound<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Conjunction(inner) => inner.link(env, file, source, diagnostics),

            Self::Disjunction(inner) => inner.link(env, file, source, diagnostics),

            Self::Selection(inner) => inner.link(env, file, source, diagnostics),

            Self::Not(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Not(Box::new(x))),

            Self::Exact(inner) => inner
                .link(env, file, source, diagnostics)
                .map(hir::TypeBound::Exact),

            Self::Has(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Has(Box::new(x))),

            Self::Shape(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Shape(hir::ShapeBound(x))),

            Self::Supports(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Supports(Box::new(x))),

            Self::Satisfies(inner) => inner
                .link(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Satisfies(Box::new(x))),

        }
    }
}

impl LinkAnalyze for hir::ComboBound<hir::Variable> {
    type Output = hir::TypeBound<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let lhs = self.lhs.link(env, file, source, diagnostics);
        let rhs = self.rhs.link(env, file, source, diagnostics);
        let bounds = Box::from([lhs?, rhs?]);
        Ok(match self.sep {
            Combinator::All => hir::TypeBound::Conjunction(hir::ConjunctionBound(bounds)),
            Combinator::Any => hir::TypeBound::Disjunction(hir::DisjunctionBound(bounds)),
            Combinator::One => hir::TypeBound::Selection(hir::SelectionBound(bounds)),
        })
    }
}

impl LinkAnalyze for hir::NegativeBound<hir::Variable> {
    type Output = hir::NegativeBound<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.rhs
            .link(env, file, source, diagnostics)
            .map(hir::NegativeBound)
    }
}

impl LinkAnalyze for hir::ExactTypeBound<hir::Variable> {
    type Output = hir::ExactTypeBound<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.ty
            .link(env, file, source, diagnostics)
            .map(hir::ExactTypeBound)
    }
}

impl LinkAnalyze for hir::MemberBound<hir::Variable> {
    type Output = hir::MemberBound<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let req = self
            .req
            .map(|x| x.bound.link(env, file, source, diagnostics))
            .transpose();
        Ok(hir::MemberBound {
            member: self.member,
            req: req?,
        })
    }
}

impl LinkAnalyze for hir::OverloadBound<hir::Variable> {
    type Output = hir::OverloadBound;

    fn link(
        self,
        _env: &mut ModScopeEnvironment<'_>,
        _file: FileId,
        _source: &SourceMap,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(hir::OverloadBound {
            // TODO
        })
    }
}

impl LinkAnalyze for hir::ExprBound<hir::Variable> {
    type Output = hir::ExprBound<hir::PostDeclVar>;

    fn link(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        // TODO: scope the parameter
        let ident = self.name.within_file(file);
        let param = env.decl_var(source[ident].to_owned(), ident);
        let expr = self.expr.link(env, file, source, diagnostics);
        Ok(hir::ExprBound { param, expr: expr? })
    }
}
