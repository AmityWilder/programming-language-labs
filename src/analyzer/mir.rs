use crate::{
    analyzer::{
        scope::hir,
        static_env::{LabelId, VarId, scope::ScopeEnvironment},
    },
    error::{CheckDiagnostics, Diagnostic, DiagnosticKind, Severity},
    exec::{EnvId, LblId, Value, tree},
    lex::token::{Operator, keyword::Keyword, punc::OpSide},
    src_code::{map::SourceMap, span::FileId},
    types::{byte, sint, uint},
};
use std::collections::{HashMap, VecDeque};

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
                .map(|x| Vec::from_iter(x.map(tree::Stmt::Expr))),

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
        _file: FileId,
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
        _file: FileId,
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
        _file: FileId,
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
        _file: FileId,
        _env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<tree::Expr>, CheckDiagnostics> {
        // when are constants initialized?
        Ok(None)
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
        _file: FileId,
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

        let what = tree::Callable::Builtin(match op {
            Operator::Assign => tree::BuiltinOp::Assign,

            Operator::AndAssign => tree::BuiltinOp::AndAssign,
            Operator::OrAssign => tree::BuiltinOp::OrAssign,
            Operator::XorAssign => tree::BuiltinOp::XorAssign,
            Operator::NandAssign => tree::BuiltinOp::NandAssign,
            Operator::NorAssign => tree::BuiltinOp::NorAssign,
            Operator::XnorAssign => tree::BuiltinOp::XnorAssign,
            Operator::AddAssign => tree::BuiltinOp::AddAssign,
            Operator::SubAssign => tree::BuiltinOp::SubAssign,
            Operator::MulAssign => tree::BuiltinOp::MulAssign,
            Operator::PowAssign => tree::BuiltinOp::PowAssign,
            Operator::DivAssign => tree::BuiltinOp::DivAssign,
            Operator::RemAssign => tree::BuiltinOp::RemAssign,
            Operator::ShlAssign => tree::BuiltinOp::ShlAssign,
            Operator::ShrAssign => tree::BuiltinOp::ShrAssign,
            Operator::RotlAssign => tree::BuiltinOp::RotlAssign,
            Operator::RotrAssign => tree::BuiltinOp::RotrAssign,

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

        let lhs = lhs.finalize(source, file, env, diagnostics)?;
        let rhs = rhs.finalize(source, file, env, diagnostics)?;
        let args = Box::from([lhs, rhs]);

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
        let Self {
            op,
            operand,
            side: _,
        } = self;
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
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        let Self { arr, idx } = self;
        Ok(tree::Expr::Operation(tree::Operation {
            what: tree::Callable::Builtin(tree::BuiltinOp::Index),
            args: Box::from([
                arr.finalize(source, file, env, diagnostics)?,
                idx.finalize(source, file, env, diagnostics)?,
            ]),
        }))
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

impl hir::LinkedLiteralValue {
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

impl hir::LinkedLiteral {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Expr, CheckDiagnostics> {
        self.val.finalize(source, file, env, diagnostics)
    }
}

impl hir::LinkedDecl {
    fn finalize(
        self,
        _source: &SourceMap,
        _file: FileId,
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
        _file: FileId,
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
        if let Some(default) = iter.default_case() {
            cases.push(tree::Case {
                pred: tree::Expr::Literal(Value::Bool(true)),
                then: default.finalize(source, file, env, diagnostics)?,
            });
        }
        Ok(tree::Branch::Cases(tree::Cases {
            cases: cases.into_boxed_slice(),
        }))
    }
}

impl hir::PickExpr<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Branch, CheckDiagnostics> {
        use hir::LinkedLiteralValue as LitVal;
        let Self { expr, arms } = self;
        let expr = expr.finalize(source, file, env, diagnostics)?;
        let mut arms = arms
            .into_iter()
            .map(|arm| Ok((arm.pat, arm.expr.finalize(source, file, env, diagnostics)?)))
            .collect::<Result<Vec<_>, _>>()?;

        arms.sort_by(|(a, _), (b, _)| match (a, b) {
            (hir::PickPattern::Literal(a), hir::PickPattern::Literal(b)) => {
                match (&a.val, &b.val) {
                    (LitVal::None, LitVal::None) => std::cmp::Ordering::Equal,
                    (LitVal::None, _) => std::cmp::Ordering::Less,
                    (_, LitVal::None) => std::cmp::Ordering::Greater,

                    (LitVal::Bool(a), LitVal::Bool(b)) => a.cmp(b),
                    (LitVal::Byte(a), LitVal::Byte(b)) => a.cmp(b),
                    (LitVal::Uint(a), LitVal::Uint(b)) => a.cmp(b),
                    (LitVal::Sint(a), LitVal::Sint(b)) => a.cmp(b),
                    (LitVal::Char(a), LitVal::Char(b)) => a.cmp(b),
                    (LitVal::Text(a), LitVal::Text(b)) => a.cmp(b),

                    (LitVal::Flpt(_), _) | (_, LitVal::Flpt(_)) => {
                        unimplemented!("cannot jump with floats")
                    }

                    _ => todo!("multi-type jump table"),
                }
            }

            _ => todo!("non-literal jump table"),
        });
        arms.dedup_by(|&mut (ref a, _), &mut (ref b, _)| {
            // note: `b` comes before `a`
            let is_eq = match (a, b) {
                (hir::PickPattern::Literal(a), hir::PickPattern::Literal(b)) => {
                    match (&a.val, &b.val) {
                        (LitVal::None, LitVal::None) => true,

                        (LitVal::Bool(a), LitVal::Bool(b)) => a == b,
                        (LitVal::Byte(a), LitVal::Byte(b)) => a == b,
                        (LitVal::Uint(a), LitVal::Uint(b)) => a == b,
                        (LitVal::Sint(a), LitVal::Sint(b)) => a == b,
                        (LitVal::Char(a), LitVal::Char(b)) => a == b,
                        (LitVal::Text(a), LitVal::Text(b)) => a == b,

                        (LitVal::Flpt(_), _) | (_, LitVal::Flpt(_)) => {
                            unimplemented!("cannot jump with floats")
                        }

                        _ => todo!("multi-type jump table"),
                    }
                }

                _ => todo!("non-literal jump table"),
            };
            if is_eq {
                let span = match a {
                    hir::PickPattern::Constant(_) => todo!("non-literal jump table"),
                    hir::PickPattern::Literal(x) => x.span.within_file(file),
                };
                diagnostics.push(Diagnostic::new(
                    span,
                    DiagnosticKind::Other {
                        severity: Severity::Warning,
                        category: crate::error::DiagnosticCategory::StaticAnalysis,
                        msg: "unreachable pattern".to_owned(),
                    },
                ));
            }
            is_eq
        });
        let (pats, thens) = arms.into_iter().collect::<(Vec<_>, Vec<_>)>();

        // CURRENT IMPLEMENTATION: all arms must be literals of the same type.
        // patterns are sorted and unique.

        let num_pats = pats.len();
        let mut iter = pats.into_iter();
        let mut transformer = match iter.next().expect("should be caught by parser") {
            hir::PickPattern::Literal(x) => match x.val {
                LitVal::None => todo!("jump with `none`"),

                // TODO: default case?
                LitVal::Bool(true) => todo!("error: not all cases covered: missing `fals`"),
                LitVal::Bool(false) => 'case: {
                    if let Some(x) = iter.next() {
                        match x {
                            hir::PickPattern::Literal(x) => {
                                if let hir::LinkedLiteralValue::Bool(b) = x.val {
                                    debug_assert_eq!(b, true, "should not contain duplicates");
                                    break 'case tree::JumpLookup::Bools;
                                } else {
                                    todo!("multi-type jump table")
                                }
                            }
                            _ => todo!("non-literal jump table"),
                        }
                    }
                    todo!("error: not all cases covered: missing `true`")
                }

                LitVal::Byte(min) => {
                    if let Some((max, (pat, x))) = iter
                        .enumerate()
                        .map(|(i, x)| {
                            (
                                min + byte::try_from(i)
                                    .expect("should not have more branches than integers"),
                                match x {
                                    hir::PickPattern::Literal(x) => {
                                        if let hir::LinkedLiteralValue::Byte(val) = x.val {
                                            (x, val)
                                        } else {
                                            todo!("multi-type jump table")
                                        }
                                    }
                                    _ => todo!("non-literal jump table"),
                                },
                            )
                        })
                        .find(|(prev, (pat, x))| *x != prev + 1)
                    {
                        tree::JumpLookup::SparseBytes(todo!(
                            "sparse lookup: found {x}, expected {}",
                            max + 1,
                        ))
                    } else {
                        tree::JumpLookup::AdjacentBytes(min)
                    }
                }

                LitVal::Uint(min) => {
                    if let Some((max, (pat, x))) = iter
                        .enumerate()
                        .map(|(i, x)| {
                            (
                                min + uint::try_from(i)
                                    .expect("should not have more branches than integers"),
                                match x {
                                    hir::PickPattern::Literal(x) => {
                                        if let hir::LinkedLiteralValue::Uint(val) = x.val {
                                            (x, val)
                                        } else {
                                            todo!("multi-type jump table")
                                        }
                                    }
                                    _ => todo!("non-literal jump table"),
                                },
                            )
                        })
                        .find(|(prev, (pat, x))| *x != prev + 1)
                    {
                        tree::JumpLookup::SparseUints(todo!(
                            "sparse lookup: found {x}, expected {}",
                            max + 1,
                        ))
                    } else {
                        tree::JumpLookup::AdjacentUints(min)
                    }
                }

                LitVal::Sint(min) => {
                    if let Some((max, (pat, x))) = iter
                        .enumerate()
                        .map(|(i, x)| {
                            (
                                min + sint::try_from(i)
                                    .expect("should not have more branches than integers"),
                                match x {
                                    hir::PickPattern::Literal(x) => {
                                        if let hir::LinkedLiteralValue::Sint(val) = x.val {
                                            (x, val)
                                        } else {
                                            todo!("multi-type jump table")
                                        }
                                    }
                                    _ => todo!("non-literal jump table"),
                                },
                            )
                        })
                        .find(|(max, (pat, x))| *x != max + 1)
                    {
                        tree::JumpLookup::SparseSints(todo!(
                            "sparse lookup: found {x}, expected {}",
                            max + 1,
                        ))
                    } else {
                        tree::JumpLookup::AdjacentSints(min)
                    }
                }

                LitVal::Char(min) => todo!("pick char"),

                LitVal::Text(x) => {
                    let mut map = HashMap::with_capacity(num_pats);
                    map.extend(
                        std::iter::once(x)
                            .chain(iter.map(|x| match x {
                                hir::PickPattern::Literal(x) => {
                                    if let LitVal::Text(x) = x.val {
                                        x
                                    } else {
                                        todo!("multi-type jump table")
                                    }
                                }
                                _ => todo!("non-literal jump table"),
                            }))
                            .enumerate()
                            .map(|(i, x)| (x, i)),
                    );
                    tree::JumpLookup::Strings(map)
                }

                LitVal::Flpt(_) => unimplemented!("cannot jump with floats"),
            },

            _ => todo!("non-literal jump table"),
        };

        Ok(tree::Branch::Table(tree::JumpTable {
            expr,
            transformer, // TODO
            thens: thens.into_boxed_slice(),
        }))
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
        let Self { label, body } = self;
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
        let Self { label, mut body } = self;
        body.decls
            .push_back(hir::Decl::Stmt(hir::Stmt::Done(hir::DoneStmt {
                // only escape this loop
                label: None,
                // TBD: should this output the previous statement value?
                expr: None,
            })));
        hir::LoopStmt { label, body }.finalize(source, file, env, diagnostics)
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
        let Self {
            label,
            pred,
            mut body,
        } = self;
        body.decls
            .push_front(hir::Decl::Stmt(hir::Stmt::If(hir::IfStmt {
                pred: hir::Expr::Unary(Box::new(hir::Unary {
                    op: Operator::Not,
                    operand: pred,
                    side: OpSide::Prefix,
                })),
                then: hir::BlockStmt {
                    decls: VecDeque::from([hir::Decl::Stmt(hir::Stmt::Done(hir::DoneStmt {
                        // only escape this loop
                        label: None,
                        expr: None,
                    }))]),
                },
                or: None,
            })));
        hir::LoopStmt { label, body }.finalize(source, file, env, diagnostics)
    }
}

impl hir::ForStmt<hir::FinalPass> {
    fn finalize(
        self,
        source: &SourceMap,
        file: FileId,
        env: &mut ScopeEnvironment,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Loop, CheckDiagnostics> {
        let Self {
            label,
            var: hir::BindingPattern { var },
            iter,
            mut body,
        } = self;
        // cant be `rep` because `rep` would put the predicate before everything the for loop inserts
        body.decls
            .push_front(hir::Decl::Stmt(hir::Stmt::If(hir::IfStmt {
                pred: hir::Expr::Unary(Box::new(hir::Unary {
                    op: Operator::Not,
                    operand: hir::Expr::Unary(Box::new(hir::Unary {
                        op: Operator::Exists,
                        operand: hir::Expr::Variable(hir::LinkedVar(var.0)),
                        side: OpSide::Prefix,
                    })),
                    side: OpSide::Prefix,
                })),
                then: hir::BlockStmt {
                    decls: VecDeque::from([hir::Decl::Stmt(hir::Stmt::Done(hir::DoneStmt {
                        // only escape this loop
                        label: None,
                        expr: None,
                    }))]),
                },
                or: None,
            })));
        body.decls.push_front(hir::Decl::Let(hir::DeclLet {
            id: var,
            ty: None,
            init: Some(hir::Expr::Unary(Box::new(hir::Unary {
                op: Operator::Incr,
                operand: iter,
                side: OpSide::Postfix,
            }))),
        }));
        hir::LoopStmt { label, body }.finalize(source, file, env, diagnostics)
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
        env: &mut ScopeEnvironment,
        _diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<tree::Skip, CheckDiagnostics> {
        let Self { label } = self;
        Ok(tree::Skip {
            label: label.map(|hir::LinkedLabel(id)| env.get_lbl_id(id)),
        })
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
        let Self { label, expr } = self;
        Ok(tree::Done {
            label: label.map(|hir::LinkedLabel(id)| env.get_lbl_id(id)),
            value: expr
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
