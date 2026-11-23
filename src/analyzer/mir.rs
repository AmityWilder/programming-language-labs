use crate::{
    analyzer::{scope::hir, static_env::scope::ScopeEnvironment},
    error::{CheckDiagnostics, Diagnostic},
    exec::tree,
    src_code::map::SourceMap,
};
use std::collections::VecDeque;

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
        stmts.extend(finalize_decls::<_, Vec<_>>(
            unit.decls,
            source,
            env,
            diagnostics,
        )?);
    }
    Ok(tree::Program {
        stmts: stmts.into_boxed_slice(),
    })
}

fn finalize_decls<I, A>(
    decls: I,
    source: &SourceMap,
    env: &mut ScopeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<A, CheckDiagnostics>
where
    I: IntoIterator<Item = hir::Decl<hir::FinalPass>>,
    A: Default + Extend<tree::Stmt>,
{
    let mut stmts = A::default();
    let mut decls = decls.into_iter();
    let (lo, hi) = decls.size_hint();
    stmts.extend_reserve(hi.unwrap_or(lo));
    while let Some(decl) = decls.next() {
        match decl {
            hir::Decl::Dfer(x) => {
                stmts.extend_one(tree::Stmt::Dfer(tree::Dfer {
                    // TODO
                    ret_param: None,

                    on_pop: x.on_pop.finalize(source, env, diagnostics)?,

                    do_first: tree::Block {
                        stmts: finalize_decls::<_, Vec<_>>(decls, source, env, diagnostics)?
                            .into_boxed_slice(),
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
            hir::Decl::Mod(_) | hir::Decl::Rec(_) => Ok(Vec::new()),

            hir::Decl::On(x) => x.finalize(source, env, diagnostics),
            hir::Decl::Cat(x) => x.finalize(source, env, diagnostics),
            hir::Decl::Fun(x) => x.finalize(source, env, diagnostics),

            hir::Decl::Uni(x) => x
                .finalize(source, env, diagnostics)
                .map(|x| Vec::from_iter(x.map(tree::Stmt::Expr))),
            hir::Decl::Let(x) => x
                .finalize(source, env, diagnostics)
                .map(|x| Vec::from_iter(x.map(tree::Stmt::Expr))),
            hir::Decl::Pvt(x) => x
                .finalize(source, env, diagnostics)
                .map(|x| [tree::Stmt::Expr(x)].to_vec()),

            hir::Decl::Dfer(_) => unimplemented!("should be handled by caller"),

            hir::Decl::Stmt(x) => x.finalize(source, env, diagnostics).map(|x| vec![x]),
        }
    }
}

impl hir::DeclOn<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<tree::Stmt>, CheckDiagnostics> {
        todo!()
    }
}

impl hir::DeclFun<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<tree::Stmt>, CheckDiagnostics> {
        // TODO: function bodies can contain globals or other definitions that need to be evaluated
        todo!()
    }
}

impl hir::DeclCat<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<tree::Stmt>, CheckDiagnostics> {
        // TODO: evaluate discriminants(?)
        todo!()
    }
}

impl hir::DeclUni<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        self.init
            .map(|x| x.finalize(source, env, diagnostics))
            .transpose()
    }
}

impl hir::DeclLet<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        self.init
            .map(|x| x.finalize(source, env, diagnostics))
            .transpose()
    }
}

impl hir::DeclPvt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        // when are constants initialized?
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
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        match self {
            hir::Expr::Block(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Binary(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Unary(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Index(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Call(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Literal(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Directive(x) => x.finalize(source, env, diagnostics),
            hir::Expr::Variable(x) => x.finalize(source, env, diagnostics),
        }
    }
}

impl hir::BlockExpr<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!()
    }
}

impl hir::Binary<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        use crate::lex::token::Operator;
        let Self { op, lhs, rhs } = self;
        let lhs = lhs.finalize(source, env, diagnostics)?;
        let rhs = rhs.finalize(source, env, diagnostics)?;
        let args = Box::from([lhs, rhs]);
        let what = match op {
            Operator::Assign => todo!(),
            Operator::And => todo!(),
            Operator::Or => todo!(),
            Operator::Nand => todo!(),
            Operator::Nor => todo!(),
            Operator::Xor => todo!(),
            Operator::Xnor => todo!(),
            Operator::AndAssign => todo!(),
            Operator::OrAssign => todo!(),
            Operator::XorAssign => todo!(),
            Operator::NandAssign => todo!(),
            Operator::NorAssign => todo!(),
            Operator::XnorAssign => todo!(),
            Operator::Add => todo!(),
            Operator::Sub => todo!(),
            Operator::Mul => todo!(),
            Operator::Pow => todo!(),
            Operator::Div => todo!(),
            Operator::Rem => todo!(),
            Operator::AddAssign => todo!(),
            Operator::SubAssign => todo!(),
            Operator::MulAssign => todo!(),
            Operator::PowAssign => todo!(),
            Operator::DivAssign => todo!(),
            Operator::RemAssign => todo!(),
            Operator::Shl => todo!(),
            Operator::Shr => todo!(),
            Operator::Rotl => todo!(),
            Operator::Rotr => todo!(),
            Operator::ShlAssign => todo!(),
            Operator::ShrAssign => todo!(),
            Operator::RotlAssign => todo!(),
            Operator::RotrAssign => todo!(),
            Operator::Lt => todo!(),
            Operator::Gt => todo!(),
            Operator::Ne => todo!(),
            Operator::Le => todo!(),
            Operator::Eq => todo!(),
            Operator::Ge => todo!(),
            Operator::Convert => todo!(),
            Operator::Transmute => todo!(),
            _ => todo!("not a binary operator"),
        };
        Ok(tree::Expr::Operation(tree::Operation { what, args }))
    }
}

impl hir::Unary<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        use crate::lex::token::Operator;
        let Self { op, operand, side } = self;
        let operand = operand.finalize(source, env, diagnostics)?;
        let args = Box::from([operand]);
        let what = match op {
            Operator::Not => todo!(),
            Operator::Exists => todo!(),
            Operator::Incr => todo!(),
            Operator::Neg => todo!(),
            Operator::Decr => todo!(),
            Operator::Coalesce => todo!(),
            Operator::Ref => todo!(),
            Operator::Dot => todo!(),
            Operator::Szof => todo!(),
            _ => todo!("not a unary operator"),
        };
        Ok(tree::Expr::Operation(tree::Operation { what, args }))
    }
}

impl hir::Subscript<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!()
    }
}

impl hir::FnCall<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!()
    }
}

impl hir::Directive {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!()
    }
}

impl hir::LinkedLiteral {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!()
    }
}

impl hir::LinkedVar {
    fn finalize(
        self,
        _source: &SourceMap,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!()
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

#[derive(Default)]
enum IfStmtIntoIter<S: hir::HirPass> {
    #[default]
    None,
    More(hir::IfStmt<S>),
    Last(hir::BlockStmt<S>),
}

impl<S: hir::HirPass> IfStmtIntoIter<S> {
    const fn new(stmt: hir::IfStmt<S>) -> Self {
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
        for (pred, then) in iter.by_ref() {
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
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        // TODO: label??
        let hir::LoopStmt { label, body } = self;
        body.finalize(source, env, diagnostics)
            .map(|block| tree::Loop { block })
    }
}

impl hir::CordStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        // TODO: label??
        let hir::CordStmt { label, body } = self;
        finalize_decls(body.decls, source, env, diagnostics).map(|mut stmts: Vec<_>| {
            stmts.push(tree::Stmt::Done(tree::Done {
                // only escape this loop
                label: None,
                value: tree::Expr::default(),
            }));
            tree::Loop {
                block: tree::Block {
                    stmts: stmts.into_boxed_slice(),
                },
            }
        })
    }
}

impl hir::RepStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        // TODO: label??
        let hir::RepStmt { label, pred, body } = self;
        let pred = pred.finalize(source, env, diagnostics)?;
        finalize_decls(body.decls, source, env, diagnostics).map(|mut stmts: VecDeque<_>| {
            stmts.push_front(tree::Stmt::Branch(tree::Branch {
                cases: Box::from([tree::Case {
                    pred,
                    then: tree::Block {
                        stmts: Box::from([tree::Stmt::Done(tree::Done {
                            // only escape this loop
                            label: None,
                            value: tree::Expr::default(),
                        })]),
                    },
                }]),
                default: None,
            }));
            tree::Loop {
                block: tree::Block {
                    stmts: Box::from_iter(stmts),
                },
            }
        })
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
        finalize_decls(self.decls, source, env, diagnostics).map(|x: Vec<_>| tree::Block {
            stmts: x.into_boxed_slice(),
        })
    }
}
