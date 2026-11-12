use super::{env::ModScopeEnvironment, hir};
use crate::{
    error::Diagnostic,
    parse::{ast, util::OkOrFlatten},
    src_code::{code::SourceCode, span::FileId},
    util::collect_all_errs_if_any,
};

impl<'src> ModScopeEnvironment<'_, 'src> {
    pub fn analyze(
        &mut self,
        source: &'src SourceCode,
        prgm: ast::ProgramUnit,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::HirUnit, ()> {
        collect_all_errs_if_any(
            prgm.decls
                .into_iter()
                .map(|decl| self.analyze_decl(prgm.file, source, decl, diagnostics)),
        )
        .map(|stmts| hir::HirUnit {
            file: prgm.file,
            stmts,
        })
        .map_err(|it| it.for_each(drop))
    }

    fn analyze_decl(
        &mut self,
        file: FileId,
        source: &'src SourceCode,
        decl: ast::Decl,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Decl, ()> {
        Ok(match decl {
            ast::Decl::Mod(_inner) => todo!("submodules"),

            ast::Decl::Var(inner) => {
                let ident = inner.binding.ident;
                let name = &source[ident];
                let id = self.decl_var(name, ident.within_file(file), inner.clone());
                let ty = inner.ty.map(|annotation| annotation.ty);
                let init = inner
                    .assignment
                    .map(|x| self.analyze_expr(file, source, x.expr, diagnostics))
                    .transpose();
                match inner.kw {
                    ast::VarKeyword::Let => hir::Decl::Let(hir::DeclLet {
                        id,
                        ty,
                        init: init?,
                    }),

                    ast::VarKeyword::Uni => hir::Decl::Uni(hir::DeclUni {
                        id,
                        ty,
                        init: init?,
                    }),

                    ast::VarKeyword::Pvt => hir::Decl::Pvt(hir::DeclPvt {
                        id,
                        ty,
                        init: {
                            init.transpose()
                                .ok_or_else_flat(|| todo!("error: pvt needs to be initialized"))?
                        },
                    }),
                }
            }

            ast::Decl::Type(inner) => match inner {
                ast::DeclType::Rec(inner) => {
                    // TODO: generics
                    let ident = inner.name;
                    let name = &source[ident];
                    match self.decl_item(name, ident.within_file(file), inner.clone()) {
                        Ok(id) => hir::Decl::Rec(hir::DeclRec {
                            id,
                            fields: inner
                                .body
                                .fields
                                .into_iter()
                                .map(|field| {
                                    let ident = field.name;
                                    let name = &source[ident];
                                    let ty = field.ty.ty.clone();
                                    match self.decl_member(id, name, ident.within_file(file), field)
                                    {
                                        Ok(id) => hir::Field { id, ty },
                                        Err(e) => todo!("error: {e:?}"),
                                    }
                                })
                                .collect(),
                        }),
                        Err(e) => todo!("error: {e:?}"),
                    }
                }

                ast::DeclType::Cat(inner) => {
                    let ident = inner.name;
                    let name = &source[ident];
                    match self.decl_item(name, ident.within_file(file), inner.clone()) {
                        Ok(id) => hir::Decl::Cat(hir::DeclCat {
                            id,
                            discriminants: inner
                                .discriminants
                                .items
                                .into_iter()
                                .map(|disc| todo!("discriminant"))
                                .collect(),
                        }),
                        Err(e) => todo!("error: {e:?}"),
                    }
                }
            },

            ast::Decl::Defer(inner) => hir::Decl::Defer(hir::DeclDefer {
                param: inner.param.map(|x| {
                    self.decl_var(&source[x.param.ident], x.param.ident.within_file(file), x)
                }),
                on_pop: hir::BlockStmt {
                    decls: collect_all_errs_if_any(
                        inner
                            .on_pop
                            .body
                            .into_iter()
                            .map(|decl| self.analyze_decl(file, source, decl, diagnostics)),
                    )
                    .map_err(|it| it.for_each(drop))?,
                },
            }),

            ast::Decl::Stmt(inner) => self
                .analyze_stmt(file, source, inner, diagnostics)
                .map(hir::Decl::Stmt)?,
        })
    }

    fn analyze_stmt(
        &mut self,
        file: FileId,
        source: &'src SourceCode,
        stmt: ast::Stmt,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Stmt, ()> {
        match stmt {
            ast::Stmt::Expr(inner) => self
                .analyze_expr(file, source, inner.expr, diagnostics)
                .map(|expr| hir::Stmt::Expr(hir::ExprStmt { expr })),

            ast::Stmt::Print(inner) => self
                .analyze_expr(file, source, inner.group.expr, diagnostics)
                .map(|expr| hir::Stmt::Print(hir::PrintStmt { expr })),

            ast::Stmt::If(inner) => todo!(),

            ast::Stmt::Loop(inner) => todo!(),

            ast::Stmt::Done(inner) => todo!(),

            ast::Stmt::Skip(inner) => todo!(),

            ast::Stmt::Block(inner) => todo!(),
        }
    }

    fn analyze_expr(
        &mut self,
        file: FileId,
        source: &'src SourceCode,
        expr: ast::Expr,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Expr, ()> {
        match expr {
            ast::Expr::Block(inner) => todo!(),

            ast::Expr::Binary(inner) => todo!(),

            ast::Expr::Unary(inner) => todo!(),

            ast::Expr::Literal(inner) => todo!(),

            ast::Expr::Directive(inner) => todo!(),

            ast::Expr::Variable(inner) => todo!(),

            ast::Expr::Grouping(inner) => todo!(),

            ast::Expr::Type(inner) => todo!(),
        }
    }
}
