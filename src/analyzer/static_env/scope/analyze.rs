use super::{env::ModScopeEnvironment, hir};
use crate::{
    analyzer::{
        static_env::{ModId, scope::env::ScopeEnvironment},
        types::bounds::Combinator,
    },
    error::{CheckDiagnostics, Diagnostic},
    parse::{ast, util::OkOrFlatten as _},
    src_code::{code::SourceCode, span::FileId},
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
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> BTreeMap<String, ModId> {
        let mut mapping = BTreeMap::new();
        for decl in &unit.decls {
            if let ast::Decl::Mod(decl) = *decl {
                let subname = &code[decl.name];
                match self.decl_submodule(id, subname.to_owned(), decl) {
                    Ok(subid) => {
                        if let Err(e) = mapping.try_insert(subname.to_owned(), subid) {
                            todo!("error: duplicate module name: {e:?}");
                        }
                    }
                    Err(e) => todo!("error declaring submodule: {e:?}"),
                }
            }
        }
        mapping
    }
}

impl ModScopeEnvironment<'_> {
    pub fn analyze(
        &mut self,
        source: &SourceCode,
        prgm: ast::ProgramUnit,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::HirUnit, CheckDiagnostics> {
        prgm.decls
            .into_iter()
            .map(|decl| self.analyze_decl(prgm.file, source, decl, diagnostics))
            .collect_or_drop_all_errs()
            .map(|decls| hir::HirUnit {
                file: prgm.file,
                decls,
            })
            .ok_or(CheckDiagnostics)
    }

    fn analyze_block(
        &mut self,
        file: FileId,
        source: &SourceCode,
        block: ast::BlockStmt,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::BlockStmt, CheckDiagnostics> {
        block
            .body
            .into_iter()
            .map(|decl| self.analyze_decl(file, source, decl, diagnostics))
            .collect_boxed_or_drop_all_errs()
            .map(|decls| hir::BlockStmt { decls })
            .ok_or(CheckDiagnostics)
    }

    fn analyze_decl(
        &mut self,
        file: FileId,
        source: &SourceCode,
        decl: ast::Decl,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Decl, CheckDiagnostics> {
        Ok(match decl {
            ast::Decl::Mod(inner) => {
                let id = self
                    .find_submodule(&source[inner.name])
                    .expect("should be handled in pre-scan");
                hir::Decl::Mod(hir::DeclMod { id })
            }

            ast::Decl::On(_inner) => {
                todo!("`on` block analysis")
            }

            ast::Decl::Var(inner) => {
                let ident = inner.binding.ident;
                let name = &source[ident];
                let id = self.decl_var(name.to_owned(), inner.clone());
                let ty = inner
                    .ty
                    .map(|annotation| self.analyze_type(file, source, annotation.ty, diagnostics))
                    .transpose();
                let init = inner
                    .assignment
                    .map(|x| self.analyze_expr(file, source, x.expr, diagnostics))
                    .transpose();
                match inner.kw {
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

                    ast::VarKeyword::Pvt => hir::Decl::Pvt(hir::DeclPvt {
                        id,
                        ty: ty?,
                        init: {
                            init.transpose()
                                .ok_or_else_flat(|| todo!("error: pvt needs to be initialized"))?
                        },
                    }),
                }
            }

            ast::Decl::Fun(_inner) => {
                todo!("function declaration analysis")
            }

            ast::Decl::Type(inner) => match inner {
                ast::DeclType::Rec(inner) => {
                    // TODO: generics
                    let ident = inner.name;
                    let name = &source[ident];
                    let id = self
                        .decl_item(name.to_owned(), inner.clone())
                        .map_err(|e| todo!("error declaring rec {name}: {e:?}"))?;
                    hir::Decl::Rec(hir::DeclRec {
                        id,
                        fields: inner
                            .body
                            .fields
                            .into_iter()
                            .map(|field| -> Result<_, CheckDiagnostics> {
                                let ident = field.name;
                                let field_name = &source[ident];
                                let ty = self.analyze_type(
                                    file,
                                    source,
                                    field.ty.ty.clone(),
                                    diagnostics,
                                );
                                let id = self
                                    .decl_member(id, field_name.to_owned(), field)
                                    .map_err(|e| {
                                        todo!("error declaring field {name}::{field_name}: {e:?}");
                                    });
                                Ok(hir::Field { id: id?, ty: ty? })
                            })
                            .collect_boxed_or_drop_all_errs()
                            .ok_or(CheckDiagnostics)?,
                    })
                }

                ast::DeclType::Cat(inner) => {
                    let ident = inner.name;
                    let name = &source[ident];
                    match self.decl_item(name.to_owned(), inner.clone()) {
                        Ok(id) => hir::Decl::Cat(hir::DeclCat {
                            id,
                            discriminants: inner
                                .discriminants
                                .items
                                .into_iter()
                                .map(|_disc| todo!("discriminant"))
                                .collect(),
                        }),
                        Err(e) => todo!("error: {e:?}"),
                    }
                }
            },

            ast::Decl::Dfer(inner) => hir::Decl::Dfer(hir::DeclDfer {
                param: inner
                    .param
                    .map(|x| self.decl_var(source[x.param.ident].to_owned(), x)),
                on_pop: self.analyze_block(file, source, inner.on_pop, diagnostics)?,
            }),

            ast::Decl::Stmt(inner) => self
                .analyze_stmt(file, source, inner, diagnostics)
                .map(hir::Decl::Stmt)?,
        })
    }

    fn analyze_stmt(
        &mut self,
        file: FileId,
        source: &SourceCode,
        stmt: ast::Stmt,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Stmt, CheckDiagnostics> {
        match stmt {
            ast::Stmt::Expr(inner) => self
                .analyze_expr(file, source, inner.expr, diagnostics)
                .map(|expr| hir::Stmt::Expr(hir::ExprStmt { expr })),

            ast::Stmt::Print(inner) => self
                .analyze_expr(file, source, inner.group.expr, diagnostics)
                .map(|expr| hir::Stmt::Print(hir::PrintStmt { expr })),

            ast::Stmt::If(inner) => self
                .analyze_if_stmt(file, source, inner, diagnostics)
                .map(hir::Stmt::If),

            ast::Stmt::Loop(inner) => self.analyze_loop_stmt(file, source, inner, diagnostics),

            ast::Stmt::Fail(_inner) => todo!("fail statement analysis"),
            ast::Stmt::Give(_inner) => todo!("give statement analysis"),
            ast::Stmt::Emit(_inner) => todo!("emit statement analysis"),

            ast::Stmt::Done(inner) => Ok(hir::Stmt::Done(hir::DoneStmt {
                label: inner.label.map(|lbl| hir::Label { name: lbl.name }),
                expr: inner
                    .value
                    .map(|expr| self.analyze_expr(file, source, expr, diagnostics))
                    .transpose()?,
            })),

            ast::Stmt::Skip(inner) => Ok(hir::Stmt::Skip(hir::SkipStmt {
                label: inner.label.map(|lbl| hir::Label { name: lbl.name }),
            })),

            ast::Stmt::Block(inner) => self
                .analyze_block(file, source, inner, diagnostics)
                .map(hir::Stmt::Block),
        }
    }

    fn analyze_if_stmt(
        &mut self,
        file: FileId,
        source: &SourceCode,
        stmt: ast::IfStmt,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::IfStmt, CheckDiagnostics> {
        let pred = self.analyze_expr(file, source, stmt.pred, diagnostics);
        let then = self.analyze_block(file, source, stmt.then, diagnostics);
        let or = stmt
            .or
            .map(|or| match or.body {
                ast::OrBody::More(stmt) => self
                    .analyze_if_stmt(file, source, stmt, diagnostics)
                    .map(hir::OrStmt::More),
                ast::OrBody::Last(block) => self
                    .analyze_block(file, source, block, diagnostics)
                    .map(hir::OrStmt::Last),
            })
            .transpose();
        Ok(hir::IfStmt {
            pred: pred?,
            then: then?,
            or: or?.map(Box::new),
        })
    }

    fn analyze_loop_stmt(
        &mut self,
        file: FileId,
        source: &SourceCode,
        stmt: ast::LoopBlock,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Stmt, CheckDiagnostics> {
        let label = stmt.label.map(|name| hir::Label {
            name: name.label.name,
        });
        let block = self.analyze_block(file, source, stmt.block, diagnostics);
        Ok(match stmt.kind {
            ast::LoopKind::Loop => hir::Stmt::Loop(hir::LoopStmt {
                label,
                body: block?,
            }),

            ast::LoopKind::Cord => hir::Stmt::Cord(hir::CordStmt {
                label,
                body: block?,
            }),

            ast::LoopKind::Rep { pred } => hir::Stmt::Rep(hir::RepStmt {
                pred: self.analyze_expr(file, source, pred, diagnostics)?,
                body: block?,
            }),

            kind @ ast::LoopKind::For { .. } => {
                let ast::LoopKind::For { bind, iter, .. } = kind.clone() else {
                    unreachable!()
                };
                let ident = bind.ident;
                let name = &source[ident];
                let var = self.decl_var(name.to_owned(), kind);
                hir::Stmt::For(hir::ForStmt {
                    var: hir::BindingPattern { var },
                    iter: self.analyze_expr(file, source, iter, diagnostics)?,
                    body: block?,
                })
            }
        })
    }

    fn analyze_func_type(
        &mut self,
        file: FileId,
        source: &SourceCode,
        ty: ast::FunType,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::FunType, CheckDiagnostics> {
        todo!()
    }

    fn analyze_type(
        &mut self,
        file: FileId,
        source: &SourceCode,
        ty: ast::TypeExpr,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::TypeExpr, CheckDiagnostics> {
        match ty {
            // TODO: hmm...
            ast::TypeExpr::Primitive(inner) => Ok(hir::TypeExpr::Primitive(match inner.ty {
                ast::TypenameKeyword::None => hir::PrimitiveType::None,
                ast::TypenameKeyword::Nevr => hir::PrimitiveType::Nevr,
                ast::TypenameKeyword::Fail => hir::PrimitiveType::Fail,
                ast::TypenameKeyword::Bool => hir::PrimitiveType::Bool,
                ast::TypenameKeyword::Uint => hir::PrimitiveType::Uint,
                ast::TypenameKeyword::Sint => hir::PrimitiveType::Sint,
                ast::TypenameKeyword::Flpt => hir::PrimitiveType::Flpt,
                ast::TypenameKeyword::Char => hir::PrimitiveType::Char,
                ast::TypenameKeyword::Text => hir::PrimitiveType::Text,
            })),

            ast::TypeExpr::Instance(inner) => Ok(hir::TypeExpr::Instance(todo!())),

            ast::TypeExpr::Fallible(inner) => Ok(hir::TypeExpr::Fallible(todo!())),

            ast::TypeExpr::Ref(inner) => Ok(hir::TypeExpr::Ref(Box::new(hir::RefType {
                ty: self.analyze_type(file, source, *inner.ty, diagnostics)?,
            }))),

            ast::TypeExpr::View(inner) => Ok(hir::TypeExpr::View(todo!())),

            ast::TypeExpr::Assoc(inner) => Ok(hir::TypeExpr::Assoc(todo!())),

            ast::TypeExpr::FunPtr(inner) => self
                .analyze_func_type(file, source, inner, diagnostics)
                .map(hir::TypeExpr::FunPtr),

            ast::TypeExpr::Opaque(inner) => Ok(hir::TypeExpr::Opaque(todo!())),
        }
    }

    fn analyze_ty_bound(
        &mut self,
        file: FileId,
        source: &SourceCode,
        bound: ast::TypeBound,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::TypeBound, CheckDiagnostics> {
        match bound {
            // TODO: join combo bounds
            ast::TypeBound::Combo(inner) => {
                let lhs = self.analyze_ty_bound(file, source, inner.lhs, diagnostics);
                let rhs = self.analyze_ty_bound(file, source, inner.rhs, diagnostics);
                let bounds = Box::from([lhs?, rhs?]);
                Ok(match inner.sep {
                    Combinator::All => hir::TypeBound::Conjunction(hir::ConjunctionBound(bounds)),
                    Combinator::Any => hir::TypeBound::Disjunction(hir::DisjunctionBound(bounds)),
                    Combinator::One => hir::TypeBound::Selection(hir::SelectionBound(bounds)),
                })
            }

            ast::TypeBound::Not(inner) => self
                .analyze_ty_bound(file, source, inner.rhs, diagnostics)
                .map(|bound| hir::TypeBound::Not(Box::new(hir::NegativeBound { bound }))),

            ast::TypeBound::Exact(inner) => Ok(hir::TypeBound::Exact(hir::ExactTypeBound {
                ty: self.analyze_type(file, source, inner.ty, diagnostics)?,
            })),

            ast::TypeBound::Has(inner) => Ok(hir::TypeBound::Has(Box::new(hir::MemberBound {
                member: todo!(),
                req: None,
            }))),

            ast::TypeBound::Shape(inner) => Ok(hir::TypeBound::Shape(hir::ShapeBound(
                self.analyze_func_type(file, source, *inner, diagnostics)?,
            ))),

            ast::TypeBound::Supports(_inner) => {
                Ok(hir::TypeBound::Supports(Box::new(hir::OverloadBound {
                    // TODO
                })))
            }

            ast::TypeBound::Satisfies(inner) => {
                Ok(hir::TypeBound::Satisfies(Box::new(hir::ExprBound {
                    name: todo!(),
                    expr: todo!(),
                })))
            }

            ast::TypeBound::Group(inner) => {
                self.analyze_ty_bound(file, source, inner.inner, diagnostics)
            }
        }
    }

    fn analyze_expr(
        &mut self,
        file: FileId,
        source: &SourceCode,
        expr: ast::Expr,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Expr, CheckDiagnostics> {
        match expr {
            ast::Expr::Block(inner) => Ok(hir::Expr::Block(match inner {
                ast::BlockExpr::If(inner) => self
                    .analyze_if_stmt(file, source, *inner, diagnostics)
                    .map(|x| hir::BlockExpr::If(Box::new(x)))?,

                ast::BlockExpr::Loop(inner) => {
                    let label = inner.label.map(|name| hir::Label {
                        name: name.label.name,
                    });
                    match inner.kind {
                        ast::LoopKind::Loop => hir::BlockExpr::Loop(Box::new(hir::LoopStmt {
                            label,
                            body: self.analyze_block(file, source, inner.block, diagnostics)?,
                        })),

                        ast::LoopKind::Cord => hir::BlockExpr::Cord(Box::new(hir::CordStmt {
                            label,
                            body: self.analyze_block(file, source, inner.block, diagnostics)?,
                        })),

                        ast::LoopKind::Rep { .. } | ast::LoopKind::For { .. } => {
                            todo!("error: only loop or cord can be an expression")
                        }
                    }
                }

                ast::BlockExpr::Block(inner) => self
                    .analyze_block(file, source, inner, diagnostics)
                    .map(hir::BlockExpr::Block)?,
            })),

            ast::Expr::Binary(inner) => {
                let lhs = self.analyze_expr(file, source, inner.lhs, diagnostics);
                let rhs = self.analyze_expr(file, source, inner.rhs, diagnostics);
                Ok(hir::Expr::Binary(Box::new(hir::Binary {
                    op: inner.op,
                    lhs: lhs?,
                    rhs: rhs?,
                })))
            }

            ast::Expr::Unary(inner) => Ok(hir::Expr::Unary(Box::new(hir::Unary {
                op: inner.op,
                side: inner.side,
                operand: self.analyze_expr(file, source, inner.operand, diagnostics)?,
            }))),

            ast::Expr::Index(inner) => {
                let arr = self.analyze_expr(file, source, inner.arr, diagnostics);
                let idx = self.analyze_expr(file, source, inner.index, diagnostics);
                Ok(hir::Expr::Index(Box::new(hir::Subscript {
                    arr: arr?,
                    idx: idx?,
                })))
            }

            ast::Expr::Call(inner) => {
                let fun = self.analyze_expr(file, source, inner.fun, diagnostics);
                let args = inner
                    .args
                    .into_iter()
                    .map(|arg| self.analyze_expr(file, source, arg, diagnostics))
                    .collect_boxed_or_drop_all_errs()
                    .ok_or(CheckDiagnostics);
                Ok(hir::Expr::Call(Box::new(hir::FnCall {
                    fun: fun?,
                    args: args?,
                })))
            }

            ast::Expr::Literal(inner) => Ok(hir::Expr::Literal(inner)),

            ast::Expr::Directive(inner) => Ok(hir::Expr::Directive(match inner.kw {
                ast::DirectiveKw::Inpt => hir::Directive::Inpt,
            })),

            ast::Expr::Variable(inner) => {
                let id = self
                    .lookup_var(&source[inner.ident])
                    .ok_or_else(|| todo!("error: unknown variable"))?;
                Ok(hir::Expr::Variable(hir::Variable { id }))
            }

            ast::Expr::Member(_inner) => {
                todo!("member access analysis")
            }

            ast::Expr::Grouping(inner) => self.analyze_expr(file, source, inner.expr, diagnostics),

            ast::Expr::Type(_inner) => todo!("type expr"),
        }
    }
}
