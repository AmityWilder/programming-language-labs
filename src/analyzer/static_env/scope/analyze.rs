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
                    match self.decl_item(name, ident.within_file(file), inner.clone().into()) {
                        Ok(id) => hir::Decl::Rec(hir::DeclRec {
                            id,
                            fields: inner
                                .body
                                .fields
                                .into_iter()
                                .map(|field| todo!("field"))
                                .collect(),
                        }),
                        Err(e) => todo!("error: {e:?}"),
                    }
                }

                ast::DeclType::Cat(inner) => {
                    let ident = inner.name;
                    let name = &source[ident];
                    match self.decl_item(name, ident.within_file(file), inner.clone().into()) {
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

            ast::Decl::Defer(inner) => todo!("analyze Decl::Defer"),

            ast::Decl::Stmt(inner) => todo!("analyze Decl::Stmt"),
        })
    }

    fn analyze_expr(
        &mut self,
        file: FileId,
        source: &'src SourceCode,
        expr: ast::Expr,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<hir::Expr, ()> {
        todo!("analyze_expr")
    }
}
