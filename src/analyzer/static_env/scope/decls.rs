use crate::{
    analyzer::{
        static_env::{
            ItemId, ModId, VarId,
            scope::{
                env::{ModScopeEnvironment, ScopeEnvironment},
                hir,
            },
        },
        types::bounds::Combinator,
    },
    error::{
        CheckDiagnostics, Diagnostic, DiagnosticCategory::StaticAnalysis, DiagnosticKind, Severity,
    },
    lex::token::keyword::Keyword,
    parse::{
        ast::{self, AstNode as _},
        util::OkOrFlatten as _,
    },
    src_code::{code::SourceCode, map::SourceMap, span::FileId},
    util::OrAllErrsExt as _,
};
use std::collections::BTreeMap;

// bindings are fontloaded in this module so that all errors can all be found before returning with failure

impl ScopeEnvironment {
    pub fn declared_submodules(
        &mut self,
        code: &SourceCode,
        id: ModId,
        unit: &ast::ProgramUnit,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> BTreeMap<String, ModId> {
        let mut mapping = BTreeMap::new();
        for decl in &unit.decls {
            if let ast::Decl::Mod(decl) = *decl {
                let subname = &code[decl.name];
                match self.decl_submodule(id, subname.to_owned(), decl) {
                    Ok(subid) => {
                        if let Err(e) = mapping.try_insert(subname.to_owned(), subid) {
                            diagnostics.push(Diagnostic::new(
                                decl.span().within_file(unit.file),
                                DiagnosticKind::Other {
                                    severity: Severity::Error,
                                    category: StaticAnalysis,
                                    msg: format!("duplicate module name: {e:?}"),
                                },
                            ));
                        }
                    }
                    Err(e) => diagnostics.push(Diagnostic::new(
                        decl.span().within_file(unit.file),
                        DiagnosticKind::Other {
                            severity: Severity::Error,
                            category: StaticAnalysis,
                            msg: format!("failed to declare submodule: {e:?}"),
                        },
                    )),
                }
            }
        }
        mapping
    }
}

impl ModScopeEnvironment<'_> {
    pub fn analyze(
        &mut self,
        unit: ast::ProgramUnit,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::HirUnit<hir::Variable>, CheckDiagnostics> {
        unit.decls
            .into_iter()
            .map(|decl| decl.analyze(self, unit.file, source, diagnostics))
            .collect_or_drop_all_errs()
            .map(|decls| hir::HirUnit {
                file: unit.file,
                decls,
            })
            .ok_or(CheckDiagnostics)
    }
}

pub trait VariableRef: Sized + std::fmt::Debug {
    fn try_from_id(
        id: Option<VarId>,
        file: FileId,
        ast: ast::Variable,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self, CheckDiagnostics>;
}

impl VariableRef for hir::Variable {
    fn try_from_id(
        id: Option<VarId>,
        _file: FileId,
        ast: ast::Variable,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self, CheckDiagnostics> {
        let var = if let Some(id) = id {
            Self::Found(id)
        } else {
            Self::Unknown(ast)
        };
        Ok(var)
    }
}

impl VariableRef for hir::PostDeclVar {
    fn try_from_id(
        id: Option<VarId>,
        file: FileId,
        ast: ast::Variable,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self, CheckDiagnostics> {
        id.map(Self).ok_or_else(|| {
            diagnostics.push(Diagnostic::new(
                ast.span().within_file(file),
                DiagnosticKind::UnknownVar,
            ));
            CheckDiagnostics
        })
    }
}

trait DeclAnalyze {
    type Output;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics>;
}

impl DeclAnalyze for ast::BlockStmt {
    type Output = hir::BlockStmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.body
            .into_iter()
            .map(|decl| decl.analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .map(hir::BlockStmt)
            .ok_or(CheckDiagnostics)
    }
}

impl DeclAnalyze for ast::Decl {
    type Output = hir::Decl<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Mod(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::Mod),

            Self::On(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::On),

            Self::Var(inner) => inner.analyze(env, file, source, diagnostics),

            Self::Fun(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::Fun),

            Self::Type(inner) => inner.analyze(env, file, source, diagnostics),

            Self::Dfer(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::Dfer),

            Self::Stmt(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::Stmt),
        }
    }
}

impl DeclAnalyze for ast::DeclMod {
    type Output = hir::DeclMod;

    fn analyze(
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

impl DeclAnalyze for ast::DeclVar {
    type Output = hir::Decl<hir::Variable>;

    fn analyze(
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
            .map(|x| x.expr.analyze(env, file, source, diagnostics))
            .transpose();

        let ty = self
            .ty
            .clone()
            .map(|annotation| annotation.ty.analyze(env, file, source, diagnostics))
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

impl DeclAnalyze for ast::DeclOn {
    type Output = hir::DeclOn<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let generics = self
            .params
            .map(|x| x.analyze(env, file, source, diagnostics))
            .transpose();
        let target = self.ty.analyze(env, file, source, diagnostics);
        let iff = self
            .iff
            .map(|x| x.analyze(env, file, source, diagnostics))
            .transpose();
        let members = self
            .body
            .analyze(env, file, source, diagnostics)
            .map(|x| x.0);
        Ok(hir::DeclOn {
            generics: generics?,
            target: target?,
            iff: iff?,
            members: members?,
        })
    }
}

impl DeclAnalyze for ast::TypeBoundClause {
    type Output = hir::TypeBoundClause<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let target = self.ty.analyze(env, file, source, diagnostics);
        let bound = self.bound.analyze(env, file, source, diagnostics);
        Ok(hir::TypeBoundClause {
            target: target?,
            bound: bound?,
        })
    }
}

impl DeclAnalyze for ast::IffClause {
    type Output = hir::IffClause<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.bounds
            .into_iter()
            .map(|x| x.analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics)
            .map(hir::IffClause)
    }
}

impl DeclAnalyze for ast::SelfParam {
    type Output = hir::SelfParam;

    fn analyze(
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

impl DeclAnalyze for ast::FnParam {
    type Output = hir::FunParam<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let ident = self.name.within_file(file);
        let id = env.decl_var(source[ident].to_owned(), ident);
        let ty = self.ty.ty.analyze(env, file, source, diagnostics);
        Ok(hir::FunParam { id, ty: ty? })
    }
}

impl DeclAnalyze for ast::DeclFun {
    type Output = hir::DeclFun<hir::Variable>;

    fn analyze(
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
            .map(|x| x.analyze(env, file, source, diagnostics))
            .transpose();
        let self_param = head
            .self_param
            .map(|x| x.analyze(env, file, source, diagnostics))
            .transpose();
        let params = head
            .params
            .into_iter()
            .map(|x| x.analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        let ret = head
            .ret
            .map(|x| x.ty.analyze(env, file, source, diagnostics))
            .transpose();
        let iff = head
            .iff
            .map(|x| x.analyze(env, file, source, diagnostics))
            .transpose();
        let body = body
            .map(|x| x.analyze(env, file, source, diagnostics))
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

impl DeclAnalyze for ast::DeclType {
    type Output = hir::Decl<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            ast::DeclType::Rec(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::Rec),

            ast::DeclType::Cat(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Decl::Cat),
        }
    }
}

impl DeclAnalyze for (ast::Field, ItemId) {
    type Output = hir::Field<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let (field, parent) = self;
        let ident = field.name.within_file(file);
        let field_name = &source[ident];
        let ty = field.ty.ty.clone().analyze(env, file, source, diagnostics);
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

impl DeclAnalyze for ast::DeclRec {
    type Output = hir::DeclRec<hir::Variable>;

    fn analyze(
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
            .map(|x| (x, id).analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        Ok(hir::DeclRec {
            id,
            fields: fields?,
        })
    }
}

impl DeclAnalyze for ast::DeclCat {
    type Output = hir::DeclCat<hir::Variable>;

    fn analyze(
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

impl DeclAnalyze for ast::DeclDfer {
    type Output = hir::DeclDfer<hir::Variable>;

    fn analyze(
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
        let on_pop = self.on_pop.analyze(env, file, source, diagnostics);
        Ok(hir::DeclDfer {
            param,
            on_pop: on_pop?,
        })
    }
}

impl DeclAnalyze for ast::Stmt {
    type Output = hir::Stmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            ast::Stmt::Expr(inner) => inner
                .expr
                .analyze(env, file, source, diagnostics)
                .map(|expr| hir::Stmt::Expr(hir::ExprStmt { expr })),

            ast::Stmt::Print(inner) => inner
                .group
                .expr
                .analyze(env, file, source, diagnostics)
                .map(|expr| hir::Stmt::Print(hir::PrintStmt { expr })),

            ast::Stmt::If(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Stmt::If),

            ast::Stmt::Loop(inner) => inner.analyze(env, file, source, diagnostics),

            ast::Stmt::Fail(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Stmt::Fail),

            ast::Stmt::Give(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Stmt::Give),

            // not sure if this is a good language feature
            ast::Stmt::Emit(_inner) => todo!("emit analysis"),

            ast::Stmt::Done(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Stmt::Done),

            ast::Stmt::Skip(inner) => Ok(hir::Stmt::Skip(hir::SkipStmt {
                label: inner.label.map(|lbl| hir::Label { name: lbl.name }),
            })),

            ast::Stmt::Block(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Stmt::Block),
        }
    }
}

impl DeclAnalyze for ast::LoopBlock {
    type Output = hir::Stmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let label = self.label.map(|name| hir::Label {
            name: name.label.name,
        });
        let block = self.block.analyze(env, file, source, diagnostics);
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
                pred: pred.analyze(env, file, source, diagnostics)?,
                body: block?,
            }),

            ast::LoopKind::For { bind, iter, .. } => {
                let ident = bind.ident.within_file(file);
                let name = &source[ident];
                let var = env.decl_var(name.to_owned(), ident);
                hir::Stmt::For(hir::ForStmt {
                    var: hir::BindingPattern { var },
                    iter: iter.analyze(env, file, source, diagnostics)?,
                    body: block?,
                })
            }
        })
    }
}

impl DeclAnalyze for ast::PrintStmt {
    type Output = hir::PrintStmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.group
            .expr
            .analyze(env, file, source, diagnostics)
            .map(|expr| hir::PrintStmt { expr })
    }
}

impl DeclAnalyze for ast::GiveStmt {
    type Output = hir::GiveStmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.value
            .map(|expr| expr.analyze(env, file, source, diagnostics))
            .transpose()
            .map(|expr| hir::GiveStmt { expr })
    }
}

impl DeclAnalyze for ast::FailStmt {
    type Output = hir::FailStmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.value
            .map(|expr| expr.analyze(env, file, source, diagnostics))
            .transpose()
            .map(|expr| hir::FailStmt { expr })
    }
}

impl DeclAnalyze for ast::DoneStmt {
    type Output = hir::DoneStmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let label = self.label.map(|lbl| hir::Label { name: lbl.name });
        let expr = self
            .value
            .map(|expr| expr.analyze(env, file, source, diagnostics))
            .transpose();
        Ok(hir::DoneStmt { label, expr: expr? })
    }
}

impl DeclAnalyze for ast::Expr {
    type Output = hir::Expr<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Block(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Expr::Block),

            Self::Binary(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::Expr::Binary(Box::new(x))),

            Self::Unary(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::Expr::Unary(Box::new(x))),

            Self::Index(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::Expr::Index(Box::new(x))),

            Self::Call(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::Expr::Call(Box::new(x))),

            Self::Literal(inner) => Ok(hir::Expr::Literal(inner)),

            Self::Directive(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Expr::Directive),

            Self::Member(_inner) => todo!("member access analysis"),

            Self::Variable(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::Expr::Variable),

            Self::Grouping(inner) => inner.expr.analyze(env, file, source, diagnostics),

            Self::Type(_inner) => todo!("expr type expr analysis"),
        }
    }
}

impl DeclAnalyze for ast::BlockExpr {
    type Output = hir::BlockExpr<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::If(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::BlockExpr::If(Box::new(x))),

            Self::Loop(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| match x {
                    hir::Stmt::Loop(stmt) => hir::BlockExpr::Loop(Box::new(stmt)),
                    hir::Stmt::Cord(stmt) => hir::BlockExpr::Cord(Box::new(stmt)),

                    hir::Stmt::Rep(_) | hir::Stmt::For(_) => {
                        unimplemented!("ast::BlockExpr should not be Rep or For; found: {x:?}")
                    }

                    _ => unreachable!("loop analysis should only produce a loop"),
                }),

            Self::Block(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::BlockExpr::Block),
        }
    }
}

impl DeclAnalyze for ast::Directive {
    type Output = hir::Directive;

    fn analyze(
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

impl DeclAnalyze for ast::Variable {
    type Output = hir::Variable;

    fn analyze(
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

impl DeclAnalyze for ast::IfStmt {
    type Output = hir::IfStmt<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let pred = self.pred.analyze(env, file, source, diagnostics);
        let then = self.then.analyze(env, file, source, diagnostics);
        let or = self
            .or
            .map(|or| {
                match or.body {
                    ast::OrBody::More(stmt) => stmt
                        .analyze(env, file, source, diagnostics)
                        .map(hir::OrStmt::More),

                    ast::OrBody::Last(block) => block
                        .analyze(env, file, source, diagnostics)
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

impl DeclAnalyze for ast::TypeExpr {
    type Output = hir::TypeExpr<hir::Variable>;

    fn analyze(
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
                .analyze(env, file, source, diagnostics)
                .map(hir::TypeExpr::Instance),

            ast::TypeExpr::Fallible(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Fallible(Box::new(x))),

            ast::TypeExpr::Ref(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Ref(Box::new(x))),

            ast::TypeExpr::View(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::View(Box::new(x))),

            ast::TypeExpr::Assoc(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Assoc(Box::new(x))),

            ast::TypeExpr::FunPtr(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::TypeExpr::FunPtr),

            ast::TypeExpr::Opaque(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeExpr::Opaque(Box::new(x))),
        }
    }
}

impl DeclAnalyze for ast::FallType {
    type Output = hir::FallType<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let then_ty = self.then_ty.analyze(env, file, source, diagnostics);
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

impl DeclAnalyze for ast::ViewType {
    type Output = hir::ViewType<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let item = self.item.analyze(env, file, source, diagnostics);
        let count = self
            .count
            .map(|x| x.count.analyze(env, file, source, diagnostics))
            .transpose();
        Ok(hir::ViewType {
            item: item?,
            count: count?,
        })
    }
}

impl DeclAnalyze for ast::AssocType {
    type Output = hir::AssocType<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let base = self.base.analyze(env, file, source, diagnostics);
        let assoc = self.assoc.analyze(env, file, source, diagnostics);
        Ok(hir::AssocType {
            base: base?,
            assoc: assoc?,
        })
    }
}

impl DeclAnalyze for ast::AssocTypeSource {
    type Output = hir::AssocTypeSource<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Simple(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::AssocTypeSource::Simple),
            Self::Grouped(inner) => inner
                .inner
                .analyze(env, file, source, diagnostics)
                .map(hir::AssocTypeSource::Grouped),
        }
    }
}

impl DeclAnalyze for ast::OpaqueType {
    type Output = hir::OpaqueType<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.bound
            .analyze(env, file, source, diagnostics)
            .map(hir::OpaqueType)
    }
}

impl DeclAnalyze for ast::Typename {
    type Output = hir::Typename;

    fn analyze(
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

impl DeclAnalyze for ast::FunType {
    type Output = hir::FunType<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let params = self
            .params
            .into_iter()
            .map(|x| x.analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        let ret = self
            .ret
            .map(|x| x.ty.analyze(env, file, source, diagnostics).map(Box::new))
            .transpose();
        Ok(hir::FunType {
            params: params?,
            ret: ret?,
        })
    }
}

impl DeclAnalyze for ast::RefType {
    type Output = hir::RefType<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        Ok(hir::RefType {
            ty: self.ty.analyze(env, file, source, diagnostics)?,
        })
    }
}

impl DeclAnalyze for ast::TypeInstance {
    type Output = hir::TypeInstance<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let name = self.name.analyze(env, file, source, diagnostics);
        let args = self
            .args
            .map(|args| args.analyze(env, file, source, diagnostics))
            .transpose();
        Ok(hir::TypeInstance {
            name: name?,
            args: args?,
        })
    }
}

impl DeclAnalyze for ast::Binary {
    type Output = hir::Binary<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let lhs = self.lhs.analyze(env, file, source, diagnostics);
        let rhs = self.rhs.analyze(env, file, source, diagnostics);
        Ok(hir::Binary {
            op: self.op,
            lhs: lhs?,
            rhs: rhs?,
        })
    }
}

impl DeclAnalyze for ast::Unary {
    type Output = hir::Unary<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let operand = self.operand.analyze(env, file, source, diagnostics);
        Ok(hir::Unary {
            op: self.op,
            side: self.side,
            operand: operand?,
        })
    }
}

impl DeclAnalyze for ast::Subscript {
    type Output = hir::Subscript<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let arr = self.arr.analyze(env, file, source, diagnostics);
        let idx = self.index.analyze(env, file, source, diagnostics);
        Ok(hir::Subscript {
            arr: arr?,
            idx: idx?,
        })
    }
}

impl DeclAnalyze for ast::FnCall {
    type Output = hir::FnCall<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let fun = self.fun.analyze(env, file, source, diagnostics);
        let args = self
            .args
            .into_iter()
            .map(|x| x.analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics);
        Ok(hir::FnCall {
            fun: fun?,
            args: args?,
        })
    }
}

impl DeclAnalyze for ast::GenericGroup {
    type Output = hir::GenericGroup<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.list
            .into_iter()
            .map(|x| x.analyze(env, file, source, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .ok_or(CheckDiagnostics)
            .map(hir::GenericGroup)
    }
}

impl DeclAnalyze for ast::TypeBound {
    type Output = hir::TypeBound<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Combo(inner) => inner.analyze(env, file, source, diagnostics),

            Self::Not(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Not(Box::new(x))),

            Self::Exact(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(hir::TypeBound::Exact),

            Self::Has(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Has(Box::new(x))),

            Self::Shape(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Shape(hir::ShapeBound(x))),

            Self::Supports(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Supports(Box::new(x))),

            Self::Satisfies(inner) => inner
                .analyze(env, file, source, diagnostics)
                .map(|x| hir::TypeBound::Satisfies(Box::new(x))),

            Self::Group(inner) => inner.inner.analyze(env, file, source, diagnostics),
        }
    }
}

impl DeclAnalyze for ast::ComboBound {
    type Output = hir::TypeBound<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let lhs = self.lhs.analyze(env, file, source, diagnostics);
        let rhs = self.rhs.analyze(env, file, source, diagnostics);
        let bounds = Box::from([lhs?, rhs?]);
        Ok(match self.sep {
            Combinator::All => hir::TypeBound::Conjunction(hir::ConjunctionBound(bounds)),
            Combinator::Any => hir::TypeBound::Disjunction(hir::DisjunctionBound(bounds)),
            Combinator::One => hir::TypeBound::Selection(hir::SelectionBound(bounds)),
        })
    }
}

impl DeclAnalyze for ast::NegativeBound {
    type Output = hir::NegativeBound<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.rhs
            .analyze(env, file, source, diagnostics)
            .map(hir::NegativeBound)
    }
}

impl DeclAnalyze for ast::ExactTypeBound {
    type Output = hir::ExactTypeBound<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        self.ty
            .analyze(env, file, source, diagnostics)
            .map(hir::ExactTypeBound)
    }
}

impl DeclAnalyze for ast::MemberBound {
    type Output = hir::MemberBound<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        let req = self
            .req
            .map(|x| x.bound.analyze(env, file, source, diagnostics))
            .transpose();
        Ok(hir::MemberBound {
            member: self.member,
            req: req?,
        })
    }
}

impl DeclAnalyze for ast::OverloadBound {
    type Output = hir::OverloadBound;

    fn analyze(
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

impl DeclAnalyze for ast::ExprBound {
    type Output = hir::ExprBound<hir::Variable>;

    fn analyze(
        self,
        env: &mut ModScopeEnvironment<'_>,
        file: FileId,
        source: &SourceMap,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Self::Output, CheckDiagnostics> {
        // TODO: scope the parameter
        let ident = self.name.within_file(file);
        let param = env.decl_var(source[ident].to_owned(), ident);
        let expr = self.expr.analyze(env, file, source, diagnostics);
        Ok(hir::ExprBound { param, expr: expr? })
    }
}
