use crate::{
    analyzer::{scope::hir, static_env::scope::ScopeEnvironment},
    error::{CheckDiagnostics, Diagnostic},
    exec::tree,
    src_code::map::SourceMap,
};

// TODO: this skips a lot of passes

pub fn finalize(
    hir: Vec<hir::HirUnit<hir::FinalPass>>,
    source: &SourceMap,
    env: &mut ScopeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<tree::Program, CheckDiagnostics> {
    let units = hir.into_iter();
    let (lo, hi) = units.size_hint();
    let mut stmts = Vec::with_capacity(hi.unwrap_or(lo));
    for unit in units {
        stmts.extend(finalize_decls(unit.decls, source, env, diagnostics)?);
    }
    Ok(tree::Program {
        stmts: stmts.into_boxed_slice(),
    })
}

fn finalize_decls<I>(
    decls: I,
    source: &SourceMap,
    env: &mut ScopeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Vec<tree::Stmt>, CheckDiagnostics>
where
    I: IntoIterator<Item = hir::Decl<hir::FinalPass>>,
{
    let mut decls = decls.into_iter();
    let (lo, hi) = decls.size_hint();
    let mut stmts = Vec::with_capacity(hi.unwrap_or(lo));
    while let Some(decl) = decls.next() {
        match decl {
            hir::Decl::Dfer(x) => {
                stmts.push(tree::Stmt::Dfer(tree::Dfer {
                    // TODO
                    ret_param: None,
                    on_pop: x.on_pop.finalize(source, env, diagnostics)?,
                    do_first: tree::Block {
                        stmts: finalize_decls(decls, source, env, diagnostics)?.into_boxed_slice(),
                    },
                }));
                break;
            }

            x => stmts.extend(x.finalize(source, env, diagnostics)?),
        }
    }
    Ok(stmts)
}

impl hir::Decl<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<tree::Stmt>, CheckDiagnostics> {
        match self {
            hir::Decl::Mod(x) => x.finalize(source, env, diagnostics).map(|()| Vec::new()),
            hir::Decl::On(x) => x.finalize(source, env, diagnostics).map(|()| Vec::new()),

            hir::Decl::Fun(x) => x.finalize(source, env, diagnostics).map(|()| Vec::new()),
            hir::Decl::Rec(x) => x.finalize(source, env, diagnostics).map(|()| Vec::new()),
            hir::Decl::Cat(x) => x.finalize(source, env, diagnostics).map(|()| Vec::new()),

            hir::Decl::Uni(x) => x
                .finalize(source, env, diagnostics)
                .map(|x| x.into_iter().map(tree::Stmt::Expr).collect()),
            hir::Decl::Let(x) => x
                .finalize(source, env, diagnostics)
                .map(|x| x.into_iter().map(tree::Stmt::Expr).collect()),
            hir::Decl::Pvt(x) => x
                .finalize(source, env, diagnostics)
                .map(|x| vec![tree::Stmt::Expr(x)]),

            hir::Decl::Dfer(_) => unimplemented!("should be handled by caller"),

            hir::Decl::Stmt(x) => x.finalize(source, env, diagnostics).map(|x| vec![x]),
        }
    }
}

impl hir::DeclMod<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CheckDiagnostics> {
        // job done
        Ok(())
    }
}

impl hir::DeclOn<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CheckDiagnostics> {
        // TODO
        Ok(())
    }
}

impl hir::DeclFun<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CheckDiagnostics> {
        // TODO
        Ok(())
    }
}

impl hir::DeclRec<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CheckDiagnostics> {
        // job done
        Ok(())
    }
}

impl hir::DeclCat<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CheckDiagnostics> {
        // TODO: do the discriminant values need to be evaluated?
        Ok(())
    }
}

impl hir::DeclUni<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        // initializer
        todo!()
    }
}

impl hir::DeclLet<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        // initializer
        todo!()
    }
}

impl hir::DeclPvt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        // initializer
        todo!()
    }
}

impl hir::Stmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Stmt, CheckDiagnostics> {
        match self {
            Self::Expr(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Expr),
            Self::Print(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Print),

            Self::If(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Branch),
            Self::Pick(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Branch),

            Self::Loop(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Loop),
            Self::Cord(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Loop),
            Self::Rep(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Loop),
            Self::For(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Loop),

            Self::Give(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Give),
            Self::Fail(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Fail),
            Self::Done(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Done),
            Self::Skip(x) => x.finalize(source, env, diagnostics).map(tree::Stmt::Skip),

            Self::Block(_) => unimplemented!("should be handled by caller"),
        }
    }
}

impl hir::ExprStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        self.0.finalize(source, env, diagnostics)
    }
}

impl hir::Expr<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        match self {
            hir::Expr::Block(_x) => todo!(),
            hir::Expr::Binary(_x) => todo!(),
            hir::Expr::Unary(_x) => todo!(),
            hir::Expr::Index(_x) => todo!(),
            hir::Expr::Call(_x) => todo!(),
            hir::Expr::Literal(_x) => todo!(),
            hir::Expr::Directive(_x) => todo!(),
            hir::Expr::Variable(_x) => todo!(),
        }
    }
}

impl hir::PrintStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Print, CheckDiagnostics> {
        self.0
            .finalize(source, env, diagnostics)
            .map(|msg| tree::Print { msg })
    }
}

enum IfStmtIntoIter<S: hir::HirPass> {
    None,
    More(hir::IfStmt<S>),
    Last(hir::BlockStmt<S>),
}

impl<S: hir::HirPass> Default for IfStmtIntoIter<S> {
    fn default() -> Self {
        Self::None
    }
}

impl<S: hir::HirPass> IfStmtIntoIter<S> {
    fn new(stmt: hir::IfStmt<S>) -> Self {
        Self::More(stmt)
    }

    /// Will always return [`None`] if there are more cases remaining
    fn default_case(self) -> Option<hir::BlockStmt<S>> {
        if let Self::Last(x) = self {
            Some(x)
        } else {
            None
        }
    }
}

impl<S: hir::HirPass> Iterator for IfStmtIntoIter<S> {
    type Item = (hir::Expr<S>, hir::BlockStmt<S>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Self::More(x) = std::mem::take(self) {
            let hir::IfStmt { pred, then, or } = x;
            if let Some(or) = or {
                *self = match *or {
                    hir::OrStmt::More(x) => Self::More(x),
                    hir::OrStmt::Last(x) => Self::Last(x),
                };
            } else {
                // already handled by `take`
            }
            Some((pred, then))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::None | Self::Last(_) => (0, Some(0)),
            Self::More(_) => (1, None),
        }
    }
}

impl hir::IfStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Branch, CheckDiagnostics> {
        let mut cases = Vec::new();
        let mut iter = IfStmtIntoIter::new(self);
        while let Some((pred, then)) = iter.next() {
            let case = tree::Case {
                pred: pred.finalize(source, env, diagnostics)?,
                then: then.finalize(source, env, diagnostics)?,
            };
            cases.push(case);
        }
        let default = iter
            .default_case()
            .map(|x| x.finalize(source, env, diagnostics))
            .transpose()?;
        Ok(tree::Branch {
            cases: cases.into_boxed_slice(),
            default,
        })
    }
}

impl hir::PickExpr<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Branch, CheckDiagnostics> {
        todo!()
    }
}

impl hir::LoopStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        todo!()
    }
}

impl hir::CordStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        todo!()
    }
}

impl hir::RepStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        todo!()
    }
}

impl hir::ForStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        todo!()
    }
}

impl hir::GiveStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Give, CheckDiagnostics> {
        todo!()
    }
}

impl hir::FailStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Fail, CheckDiagnostics> {
        todo!()
    }
}

impl hir::SkipStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Skip, CheckDiagnostics> {
        todo!()
    }
}

impl hir::DoneStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Done, CheckDiagnostics> {
        todo!()
    }
}

impl hir::BlockStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Block, CheckDiagnostics> {
        finalize_decls(self.decls, source, env, diagnostics).map(|x| tree::Block {
            stmts: x.into_boxed_slice(),
        })
    }
}
