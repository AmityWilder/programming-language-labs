use super::{env::ModScopeEnvironment, tir};
use crate::{
    analyzer::static_env::TypeId, error::Diagnostic, parse::ast, util::collect_all_errs_if_any,
};

impl ModScopeEnvironment<'_, '_> {
    pub fn analyze(&mut self, prgm: ast::ProgramUnit) -> Result<tir::TirUnit, Vec<Diagnostic>> {
        collect_all_errs_if_any(prgm.decls.into_iter().map(|decl| self.analyze_decl(decl)))
            .map(|stmts| tir::TirUnit {
                file: prgm.file,
                stmts,
            })
            .map_err(|it| it.flatten().collect())
    }

    fn analyze_decl(&mut self, decl: ast::Decl) -> Result<tir::Decl, Vec<Diagnostic>> {
        Ok(match decl {
            ast::Decl::Mod(_inner) => todo!("define module"),

            ast::Decl::Var(inner) => match inner.kw {
                ast::VarKeyword::Let => tir::Decl::Let(tir::DeclLet {
                    binding: tir::BindingPattern {
                        name: inner.binding.ident,
                    },
                    ty: inner
                        .ty
                        .map(|annotation| self.analyze_type(annotation.ty))
                        .transpose()?,
                    init: inner
                        .assignment
                        .map(|x| self.analyze_expr(x.expr))
                        .transpose()?,
                }),
                ast::VarKeyword::Uni => tir::Decl::Uni(tir::DeclUni {
                    name: inner.binding.ident,
                    ty: inner
                        .ty
                        .map(|annotation| self.analyze_type(annotation.ty))
                        .transpose()?,
                    init: inner
                        .assignment
                        .map(|x| self.analyze_expr(x.expr))
                        .transpose()?,
                }),
                ast::VarKeyword::Pvt => tir::Decl::Pvt(tir::DeclPvt {
                    name: inner.binding.ident,
                    ty: inner
                        .ty
                        .map(|annotation| self.analyze_type(annotation.ty))
                        .transpose()?,
                    init: {
                        match inner
                            .assignment
                            .ok_or_else(|| todo!("error: pvt needs to be initialized"))
                        {
                            Ok(x) => self.analyze_expr(x.expr)?,
                            Err(e) => return Err(vec![e]),
                        }
                    },
                }),
            },

            ast::Decl::Type(inner) => todo!(),

            ast::Decl::Defer(inner) => todo!(),

            ast::Decl::Stmt(inner) => todo!(),
        })
    }

    fn analyze_expr(&mut self, expr: ast::Expr) -> Result<tir::Expr, Vec<Diagnostic>> {
        todo!("analyze_expr")
    }

    fn analyze_type(&mut self, ty: ast::TypeExpr) -> Result<TypeId, Vec<Diagnostic>> {
        todo!("analyze_type")
    }
}
