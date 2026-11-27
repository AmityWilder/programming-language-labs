use crate::{
    analyzer::{
        scope::hir,
        static_env::{LabelId, VarId, scope::ScopeEnvironment},
    },
    error::{CheckDiagnostics, Diagnostic, DiagnosticKind, Severity},
    exec::{EnvId, LblId, Value, tree},
    lex::token::keyword::Keyword,
    src_code::{map::SourceMap, span::FileId},
};
use std::collections::VecDeque;

// TODO: this skips a lot of passes

impl ScopeEnvironment {
    fn get_env_id(&self, id: VarId) -> EnvId {
        EnvId(id.get())
    }

    fn get_lbl_id(&self, id: LabelId) -> LblId {
        LblId(id.get())
    }
}

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
            unit.file,
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
    file: FileId,
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
            hir::Decl::Dfer(hir::DeclDfer { param, on_pop }) => {
                stmts.extend_one(tree::Stmt::Dfer(tree::Dfer {
                    ret_param: param.map(|x| env.get_env_id(x.var.0)),

                    on_pop: on_pop.finalize(source, file, env, diagnostics)?,

                    do_first: tree::Block {
                        stmts: finalize_decls::<_, Vec<_>>(decls, source, file, env, diagnostics)?
                            .into_boxed_slice(),
                    },
                }));
                break;
            }

            x => stmts.extend(x.finalize(source, file, env, diagnostics)?),
        }
    }
    Ok(stmts)
}

impl hir::Decl<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<tree::Stmt>, CheckDiagnostics> {
        match self {
            hir::Decl::Mod(_) | hir::Decl::Rec(_) => Ok(Vec::new()),

            hir::Decl::On(x) => x.finalize(source, file, env, diagnostics),
            hir::Decl::Cat(x) => x.finalize(source, file, env, diagnostics),
            hir::Decl::Fun(x) => x.finalize(source, file, env, diagnostics),

            hir::Decl::Uni(x) => x
                .finalize(source, file, env, diagnostics)
                .map(|x| Vec::from_iter(x.map(tree::Stmt::Expr))),
            hir::Decl::Let(x) => x
                .finalize(source, file, env, diagnostics)
                .map(|x| Vec::from_iter(x.map(tree::Stmt::Expr))),
            hir::Decl::Pvt(x) => x
                .finalize(source, file, env, diagnostics)
                .map(|x| [tree::Stmt::Expr(x)].to_vec()),

            hir::Decl::Dfer(_) => unimplemented!("should be handled by caller"),

            hir::Decl::Stmt(hir::Stmt::Block(x)) => x
                .finalize(source, file, env, diagnostics)
                .map(|x| x.stmts.to_vec()),

            hir::Decl::Stmt(x) => x.finalize(source, file, env, diagnostics).map(|x| vec![x]),
        }
    }
}

impl hir::DeclOn<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
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
        file: FileId,
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
        file: FileId,
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
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        self.init
            .map(|x| {
                Ok(tree::Expr::Operation(tree::Operation {
                    what: tree::Callable::Builtin(tree::BuiltinOp::Assign),
                    args: Box::from([
                        self.id.finalize(source, file, env, diagnostics)?,
                        x.finalize(source, file, env, diagnostics)?,
                    ]),
                }))
            })
            .transpose()
    }
}

impl hir::DeclLet<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        self.init
            .map(|x| {
                Ok(tree::Expr::Operation(tree::Operation {
                    what: tree::Callable::Builtin(tree::BuiltinOp::Assign),
                    args: Box::from([
                        self.id.finalize(source, file, env, diagnostics)?,
                        x.finalize(source, file, env, diagnostics)?,
                    ]),
                }))
            })
            .transpose()
    }
}

impl hir::DeclPvt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
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
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Stmt, CheckDiagnostics> {
        match self {
            Self::Expr(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Expr),

            Self::If(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Branch),
            Self::Pick(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Branch),

            Self::Loop(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Loop),
            Self::Cord(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Loop),
            Self::Rep(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Loop),
            Self::For(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Loop),

            Self::Give(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Give),
            Self::Fail(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Fail),
            Self::Done(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Done),
            Self::Skip(x) => x
                .finalize(source, file, env, diagnostics)
                .map(tree::Stmt::Skip),

            Self::Block(_) => unimplemented!("should be handled by caller"),
        }
    }
}

impl hir::ExprStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        self.0.finalize(source, file, env, diagnostics)
    }
}

impl hir::Expr<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        match self {
            hir::Expr::Block(x) => x.finalize(source, file, env, diagnostics),
            hir::Expr::Binary(x) => x.finalize(source, file, env, diagnostics),
            hir::Expr::Unary(x) => x.finalize(source, file, env, diagnostics),
            hir::Expr::Index(x) => x.finalize(source, file, env, diagnostics),
            hir::Expr::Call(x) => x.finalize(source, file, env, diagnostics),
            hir::Expr::Literal(x) => x.finalize(source, file, env, diagnostics),
            hir::Expr::Variable(x) => x.finalize(source, file, env, diagnostics),

            hir::Expr::Directive(x) => {
                diagnostics.push(Diagnostic::new(
                    x.span.within_file(file),
                    DiagnosticKind::Other {
                        severity: Severity::Info,
                        category: crate::error::DiagnosticCategory::StaticAnalysis,
                        msg: format!(
                            "directives are nullary operators. while this may appear to be a value, it is a shorthand for `{}()`",
                            match x.kind {
                                hir::DirectiveKind::Inpt => Keyword::Inpt,
                                hir::DirectiveKind::Prnt => Keyword::Prnt,
                            },
                        ),
                    },
                ));
                Ok(tree::Expr::Operation(tree::Operation {
                    what: tree::Callable::Builtin(x.finalize(source, file, env, diagnostics)?),
                    args: Box::default(),
                }))
            }
        }
    }
}

impl hir::BlockExpr<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!("mir block expression")
    }
}

impl hir::Binary<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        use crate::lex::token::Operator;
        let Self { op, lhs, rhs } = self;
        let lhs = lhs.finalize(source, file, env, diagnostics)?;
        let rhs = rhs.finalize(source, file, env, diagnostics)?;
        let args = Box::from([lhs, rhs]);

        let what = tree::Callable::Builtin(match op {
            Operator::Assign => tree::BuiltinOp::Assign,

            Operator::AndAssign
            | Operator::OrAssign
            | Operator::XorAssign
            | Operator::NandAssign
            | Operator::NorAssign
            | Operator::XnorAssign
            | Operator::AddAssign
            | Operator::SubAssign
            | Operator::MulAssign
            | Operator::PowAssign
            | Operator::DivAssign
            | Operator::RemAssign
            | Operator::ShlAssign
            | Operator::ShrAssign
            | Operator::RotlAssign
            | Operator::RotrAssign => todo!("mir assignment shorthand"),

            Operator::And => tree::BuiltinOp::And,
            Operator::Or => tree::BuiltinOp::Or,
            Operator::Nand => tree::BuiltinOp::Nand,
            Operator::Nor => tree::BuiltinOp::Nor,
            Operator::Xor => tree::BuiltinOp::Xor,
            Operator::Xnor => tree::BuiltinOp::Xnor,
            Operator::Add => tree::BuiltinOp::Add,
            Operator::Sub => tree::BuiltinOp::Sub,
            Operator::Mul => tree::BuiltinOp::Mul,
            Operator::Pow => tree::BuiltinOp::Pow,
            Operator::Div => tree::BuiltinOp::Div,
            Operator::Rem => tree::BuiltinOp::Rem,
            Operator::Shl => tree::BuiltinOp::Shl,
            Operator::Shr => tree::BuiltinOp::Shr,
            Operator::Rotl => tree::BuiltinOp::Rotl,
            Operator::Rotr => tree::BuiltinOp::Rotr,
            Operator::Lt => tree::BuiltinOp::Lt,
            Operator::Gt => tree::BuiltinOp::Gt,
            Operator::Ne => tree::BuiltinOp::Ne,
            Operator::Le => tree::BuiltinOp::Le,
            Operator::Eq => tree::BuiltinOp::Eq,
            Operator::Ge => tree::BuiltinOp::Ge,
            Operator::Convert => todo!("mir Convert operator"),
            Operator::Transmute => todo!("mir Transmute operator"),

            _ => todo!("not a binary operator"),
        });

        Ok(tree::Expr::Operation(tree::Operation { what, args }))
    }
}

impl hir::Unary<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        use crate::lex::token::Operator;
        let Self { op, operand, side } = self;
        let operand = operand.finalize(source, file, env, diagnostics)?;
        let args = Box::from([operand]);

        let what = tree::Callable::Builtin(match op {
            Operator::Not => tree::BuiltinOp::Not,
            Operator::Exists => tree::BuiltinOp::Exists,
            Operator::Incr => tree::BuiltinOp::Incr,
            Operator::Neg => tree::BuiltinOp::Neg,
            Operator::Decr => tree::BuiltinOp::Decr,
            Operator::Coalesce => todo!("mir Coalesce operator"),
            Operator::Ref => todo!("mir Ref operator"),
            Operator::Dot => todo!("mir Dot operator"),
            Operator::Szof => tree::BuiltinOp::Szof,

            _ => todo!("not a unary operator"),
        });

        Ok(tree::Expr::Operation(tree::Operation { what, args }))
    }
}

impl hir::Subscript<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        todo!("mir indexer")
    }
}

impl hir::FnCall<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        let Self { fun, args } = self;
        Ok(match fun {
            hir::Expr::Block(_)
            | hir::Expr::Binary(_)
            | hir::Expr::Unary(_)
            | hir::Expr::Index(_)
            | hir::Expr::Call(_) => todo!("indirect call"),

            hir::Expr::Directive(x) => tree::Expr::Operation(tree::Operation {
                what: x
                    .finalize(source, file, env, diagnostics)
                    .map(tree::Callable::Builtin)?,
                args: args
                    .into_iter()
                    .map(|x| x.finalize(source, file, env, diagnostics))
                    .collect::<Result<_, _>>()?,
            }),

            hir::Expr::Variable(_) => todo!("user-defined functions"),

            hir::Expr::Literal(_) => todo!("error: can't call literal"),
        })
    }
}

impl hir::Directive {
    fn finalize(
        self,
        _source: &SourceMap,
        _file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::BuiltinOp, CheckDiagnostics> {
        let Self { span: _, kind } = self;
        Ok(match kind {
            hir::DirectiveKind::Inpt => tree::BuiltinOp::Inpt,
            hir::DirectiveKind::Prnt => tree::BuiltinOp::Prnt,
        })
    }
}

impl hir::LinkedLiteral {
    fn finalize(
        self,
        _source: &SourceMap,
        _file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        Ok(tree::Expr::Literal(match self {
            Self::None => Value::None,
            Self::Bool(x) => Value::Bool(x),
            Self::Byte(x) => Value::Byte(x),
            Self::Uint(x) => Value::Uint(x),
            Self::Sint(x) => Value::Sint(x),
            Self::Flpt(x) => Value::Flpt(x),
            Self::Char(x) => Value::Char(x),
            Self::Text(x) => Value::Text(x),
        }))
    }
}

impl hir::LinkedDecl {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        let Self(id) = self;
        Ok(tree::Expr::Access(env.get_env_id(id)))
    }
}

impl hir::LinkedVar {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        let Self(id) = self;
        Ok(tree::Expr::Access(env.get_env_id(id)))
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
        if matches!(self, Self::More(_)) {
            let Self::More(x) = std::mem::take(self) else {
                unreachable!("guarded by matches!()")
            };
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
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Branch, CheckDiagnostics> {
        let mut cases = Vec::new();
        let mut iter = IfStmtIntoIter::new(self);
        for (pred, then) in iter.by_ref() {
            let case = tree::Case {
                pred: pred.finalize(source, file, env, diagnostics)?,
                then: then.finalize(source, file, env, diagnostics)?,
            };
            cases.push(case);
        }
        let default = iter
            .default_case()
            .map(|x| x.finalize(source, file, env, diagnostics))
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
        file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Branch, CheckDiagnostics> {
        todo!("mir pick expression")
    }
}

impl hir::LoopStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        // TODO: label??
        let hir::LoopStmt { label, body } = self;
        let label = label.map(|hir::LinkedLabel(id)| env.get_lbl_id(id));
        body.finalize(source, file, env, diagnostics)
            .map(|block| tree::Loop { label, block })
    }
}

impl hir::CordStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        // TODO: label??
        let hir::CordStmt { label, body } = self;
        let label = label.map(|hir::LinkedLabel(id)| env.get_lbl_id(id));
        finalize_decls(body.decls, source, file, env, diagnostics).map(|mut stmts: Vec<_>| {
            stmts.push(tree::Stmt::Done(tree::Done {
                // only escape this loop
                label: None,
                value: tree::Expr::default(),
            }));
            tree::Loop {
                label,
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
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        // TODO: label??
        let hir::RepStmt { label, pred, body } = self;
        let label = label.map(|hir::LinkedLabel(id)| env.get_lbl_id(id));
        let pred = pred.finalize(source, file, env, diagnostics)?;
        finalize_decls(body.decls, source, file, env, diagnostics).map(|mut stmts: VecDeque<_>| {
            stmts.push_front(tree::Stmt::Branch(tree::Branch {
                cases: Box::from([tree::Case {
                    pred: tree::Expr::Operation(tree::Operation {
                        what: tree::Callable::Builtin(tree::BuiltinOp::Not),
                        args: Box::from([pred]),
                    }),
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
                label,
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
        file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        todo!("mir for loop")
    }
}

impl hir::GiveStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Give, CheckDiagnostics> {
        Ok(tree::Give {
            value: self
                .expr
                .map(|x| x.finalize(source, file, env, diagnostics))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl hir::FailStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Fail, CheckDiagnostics> {
        Ok(tree::Fail {
            value: self
                .expr
                .map(|x| x.finalize(source, file, env, diagnostics))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl hir::SkipStmt<hir::FinalPass> {
    fn finalize(
        self,
        _source: &SourceMap,
        file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Skip, CheckDiagnostics> {
        // TODO: label
        Ok(tree::Skip { label: None })
    }
}

impl hir::DoneStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Done, CheckDiagnostics> {
        // TODO: label
        Ok(tree::Done {
            label: None,
            value: self
                .expr
                .map(|x| x.finalize(source, file, env, diagnostics))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl hir::BlockStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Block, CheckDiagnostics> {
        finalize_decls(self.decls, source, file, env, diagnostics).map(|x: Vec<_>| tree::Block {
            stmts: x.into_boxed_slice(),
        })
    }
}
