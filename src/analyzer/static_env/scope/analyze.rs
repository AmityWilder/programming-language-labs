use crate::{
    analyzer::static_env::scope::env::ScopeEnvironment,
    error::Error,
    parse::ast::{self, AstNode},
    src_code::{code::SourceCode, span::FileId},
};

impl<'src> ScopeEnvironment<'src> {
    pub fn analyze(
        &mut self,
        code: &'src SourceCode,
        prgm: &ast::ProgramUnit,
    ) -> Result<(), Vec<Error>> {
        let errors: Vec<_> = prgm
            .decls
            .iter()
            .flat_map(|decl| self.analyze_decl(prgm.file, code, decl))
            .collect();

        errors.is_empty().ok_or(errors)
    }

    fn analyze_decl(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        decl: &ast::Decl,
    ) -> Vec<Error> {
        let mut errors = Vec::new();
        match decl {
            ast::Decl::Var(decl) => {
                let ident = decl.binding.ident;
                let var = self.decl_item(&code[ident], decl.span().within_file(file));
                self.mark_ref(ident.within_file(file), var);
            }

            ast::Decl::Defer(defer) => {
                if let Some(param) = &defer.param {
                    let ident = param.param.ident;
                    let var = self.decl_item(&code[ident], decl.span().within_file(file));
                    self.mark_ref(ident.within_file(file), var);
                }
                for decl in &defer.on_pop.body {
                    errors.extend(self.analyze_decl(file, code, decl));
                }
            }

            ast::Decl::Stmt(stmt) => {
                errors.extend(self.analyze_stmt(file, code, stmt));
            }
        }
        errors
    }

    fn analyze_block(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        block: &[ast::Decl],
    ) -> Vec<Error> {
        block
            .iter()
            .flat_map(|decl| self.analyze_decl(file, code, decl))
            .collect()
    }

    fn analyze_stmt(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        stmt: &ast::Stmt,
    ) -> Vec<Error> {
        let mut errors = Vec::new();
        // make sure expressions are analyzed before declarations! `let x = x;` should not be a self-reference!
        match stmt {
            ast::Stmt::Expr(ast::ExprStmt { expr, .. })
            | ast::Stmt::Print(ast::PrintStmt {
                group: ast::Grouping { expr, .. },
                ..
            })
            | ast::Stmt::Done(ast::DoneStmt {
                value: Some(expr), ..
            }) => {
                errors.extend(self.analyze_expr(file, code, expr));
            }

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::Rep { pred },
                block: ast::BlockStmt { body, .. },
                ..
            }) => {
                errors.extend(self.analyze_expr(file, code, pred));
                errors.extend(self.analyze_block(file, code, body));
            }

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::For { bind, iter, .. },
                block: ast::BlockStmt { body, .. },
                ..
            }) => {
                errors.extend(self.analyze_expr(file, code, iter));

                let ident = bind.ident;
                let def = ident.within_file(file); // TODO: which span represents the "definition" of the binding?
                let id = self.decl_item(&code[ident], def);
                self.mark_ref(ident.within_file(file), id);

                errors.extend(self.analyze_block(file, code, body));
            }

            // TBD: should labels be analyzed?
            ast::Stmt::Done(ast::DoneStmt { value: None, .. })
            | ast::Stmt::Skip(ast::SkipStmt { .. }) => {}

            ast::Stmt::If(stmt) => {
                let mut stmt = stmt;
                loop {
                    let ast::IfStmt {
                        pred,
                        then: ast::BlockStmt { body: then, .. },
                        or,
                        ..
                    } = stmt;
                    errors.extend(self.analyze_expr(file, code, pred));
                    errors.extend(self.analyze_block(file, code, then));
                    if let Some(ast::OrStmt { body, .. }) = or {
                        match body {
                            ast::OrBody::More(nested) => stmt = nested,

                            ast::OrBody::Last(ast::BlockStmt { body, .. }) => {
                                errors.extend(self.analyze_block(file, code, body));
                                break;
                            }
                        }
                    }
                }
            }

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::Loop | ast::LoopKind::Cord,
                block: ast::BlockStmt { body, .. },
                ..
            })
            | ast::Stmt::Block(ast::BlockStmt { body, .. }) => {
                errors.extend(self.analyze_block(file, code, body));
            }
        }
        errors
    }

    fn analyze_expr(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        expr: &ast::Expr,
    ) -> Vec<Error> {
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
