use crate::{
    analyzer::static_env::scope::env::ScopeEnvironment,
    error::Error,
    parse::ast::{self, AstNode},
    src_code::{code::SourceCode, span::FileId},
};

trait CollectErrors {
    fn collect_errs<A>(self) -> Result<(), A>
    where
        A: FromIterator<Error>;
}

impl<J: IntoIterator<Item = Error>, I: IntoIterator<Item = Result<(), J>>> CollectErrors for I {
    fn collect_errs<A>(self) -> Result<(), A>
    where
        A: FromIterator<Error>,
    {
        let mut no_errors = true;
        let errors = self
            .into_iter()
            .flat_map(|x| {
                no_errors |= x.is_err();
                x.err().into_flat_iter()
            })
            .collect::<A>();
        no_errors.ok_or(errors)
    }
}

impl<'src> ScopeEnvironment<'src> {
    pub fn analyze(
        &mut self,
        code: &'src SourceCode,
        prgm: &ast::ProgramUnit,
    ) -> Result<(), Vec<Error>> {
        let mut errors = Vec::new();
        for decl in &prgm.decls {
            if let Err(e) = self.analyze_decl(prgm.file, code, decl) {
                errors.extend(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn analyze_decl(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        decl: &ast::Decl,
    ) -> Result<(), Vec<Error>> {
        match decl {
            ast::Decl::Var(decl) => {
                let ident = decl.binding.ident;
                let var = self.decl_item(&code[ident], decl.span().within_file(file));
                self.mark_ref(ident.within_file(file), var);
                Ok(())
            }

            ast::Decl::Defer(defer) => {
                if let Some(param) = &defer.param {
                    let ident = param.param.ident;
                    let var = self.decl_item(&code[ident], decl.span().within_file(file));
                    self.mark_ref(ident.within_file(file), var);
                }
                let mut errors = Vec::new();
                for decl in &defer.on_pop.body {
                    if let Err(e) = self.analyze_decl(file, code, decl) {
                        errors.extend(e);
                    }
                }
                errors.is_empty().ok_or(errors)
            }

            ast::Decl::Stmt(stmt) => self.analyze_stmt(file, code, stmt),
        }
    }

    fn analyze_block(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        block: &[ast::Decl],
    ) -> Result<(), Vec<Error>> {
        block
            .iter()
            .map(|decl| self.analyze_decl(file, code, decl))
            .collect_errs()
    }

    fn analyze_stmt(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        stmt: &ast::Stmt,
    ) -> Result<(), Vec<Error>> {
        match stmt {
            ast::Stmt::Expr(ast::ExprStmt { expr, .. })
            | ast::Stmt::Print(ast::PrintStmt {
                group: ast::Grouping { expr, .. },
                ..
            })
            | ast::Stmt::Done(ast::DoneStmt {
                value: Some(expr), ..
            }) => self.analyze_expr(file, code, expr),

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::Rep { pred },
                block: ast::BlockStmt { body, .. },
                ..
            }) => {
                // self.analyze_expr(file, code, expr);
                self.analyze_block(file, code, body)
            }

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::For { bind, iter, .. },
                block: ast::BlockStmt { body, .. },
                ..
            }) => self.analyze_block(file, code, body),

            ast::Stmt::Done(ast::DoneStmt { value: None, .. })
            | ast::Stmt::Skip(ast::SkipStmt { .. }) => Ok(()),

            ast::Stmt::If(ast::IfStmt { pred, then, or, .. }) => todo!(),

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::Loop | ast::LoopKind::Cord,
                block: ast::BlockStmt { body, .. },
                ..
            })
            | ast::Stmt::Block(ast::BlockStmt { body, .. }) => self.analyze_block(file, code, body),
        }
    }

    fn analyze_expr(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        expr: &ast::Expr,
    ) -> Result<(), Vec<Error>> {
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
