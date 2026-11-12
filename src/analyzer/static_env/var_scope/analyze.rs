use crate::{
    analyzer::static_env::var_scope::env::ScopeEnvironment,
    error::Diagnostic,
    parse::ast::{self, AstNode},
    src_code::{code::SourceCode, span::FileId},
};

impl<'src> ScopeEnvironment<'src> {
    pub fn analyze(
        &mut self,
        code: &'src SourceCode,
        prgm: &ast::ProgramUnit,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for decl in &prgm.decls {
            self.analyze_decl(prgm.file, code, decl, diagnostics);
        }
    }

    fn analyze_decl(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        decl: &ast::Decl,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match decl {
            ast::Decl::Mod(mdl) => {
                todo!("module declaration scope analysis")
            }

            ast::Decl::Type(ty) => {
                todo!("type declaration scope analysis")
            }

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
                    (self.analyze_decl(file, code, decl, diagnostics));
                }
            }

            ast::Decl::Stmt(stmt) => {
                (self.analyze_stmt(file, code, stmt, diagnostics));
            }
        }
    }

    fn analyze_block(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        block: &[ast::Decl],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for decl in block {
            self.analyze_decl(file, code, decl, diagnostics);
        }
    }

    fn analyze_stmt(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        stmt: &ast::Stmt,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
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
                (self.analyze_expr(file, code, expr, diagnostics));
            }

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::Rep { pred },
                block: ast::BlockStmt { body, .. },
                ..
            }) => {
                (self.analyze_expr(file, code, pred, diagnostics));
                (self.analyze_block(file, code, body, diagnostics));
            }

            ast::Stmt::Loop(ast::LoopBlock {
                kind: ast::LoopKind::For { bind, iter, .. },
                block: ast::BlockStmt { body, .. },
                ..
            }) => {
                (self.analyze_expr(file, code, iter, diagnostics));

                let ident = bind.ident;
                let def = ident.within_file(file); // TODO: which span represents the "definition" of the binding?
                let id = self.decl_item(&code[ident], def);
                self.mark_ref(ident.within_file(file), id);

                (self.analyze_block(file, code, body, diagnostics));
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
                    (self.analyze_expr(file, code, pred, diagnostics));
                    (self.analyze_block(file, code, then, diagnostics));
                    if let Some(ast::OrStmt { body, .. }) = or {
                        match body {
                            ast::OrBody::More(nested) => stmt = nested,

                            ast::OrBody::Last(ast::BlockStmt { body, .. }) => {
                                (self.analyze_block(file, code, body, diagnostics));
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
                (self.analyze_block(file, code, body, diagnostics));
            }
        }
    }

    fn analyze_expr(
        &mut self,
        file: FileId,
        code: &'src SourceCode,
        expr: &ast::Expr,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
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
