use super::{
    debug_info, debug_warn,
    value::{Value, ValueMut},
};
use crate::{
    analyzer::{
        Assignment, BlockStmt, Expr, ExprStmt, HaltStmt, IfStmt, Literal, LoopStmt, OrStmt,
        PrintStmt, RuntimeOp, RuntimeProgram, RuntimeVar, Statement, types::StaticType,
    },
    exec::{env::Environment, error::Failure},
    types::*,
};
#[cfg(not(debug_assertions))]
use std::marker::PhantomData;
use std::{mem::MaybeUninit, range::Range};

#[derive(Clone)]
enum EvalValueNoEnv<'lit, 'src, T: ?Sized = [MaybeUninit<u8>]> {
    Lit(Value<'lit, 'src, T>),
    Env(Range<usize>),
}

impl<'lit, 'src, T: ?Sized> std::fmt::Debug for EvalValueNoEnv<'lit, 'src, T>
where
    Value<'lit, 'src, T>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lit(arg0) => f.debug_tuple("Lit").field(arg0).finish(),
            Self::Env(arg0) => f.debug_tuple("Env").field(arg0).finish(),
        }
    }
}

#[derive(Clone)]
enum EvalValue<'lit, 'env, 'src, T: ?Sized = [MaybeUninit<u8>]> {
    Lit(Value<'lit, 'src, T>),
    Env(Range<usize>, Value<'env, 'src, T>),
}

impl<'lit, 'env, 'src, T: ?Sized> std::fmt::Debug for EvalValue<'lit, 'env, 'src, T>
where
    Value<'lit, 'src, T>: std::fmt::Debug,
    Value<'env, 'src, T>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lit(arg0) => f.debug_tuple("Lit").field(arg0).finish(),
            Self::Env(arg0, arg1) => f.debug_tuple("Env").field(arg0).field(arg1).finish(),
        }
    }
}

impl<'val, 'src> EvalValue<'val, 'val, 'src> {
    pub fn value(self) -> Value<'val, 'src> {
        match self {
            Self::Lit(value) | Self::Env(_, value) => value,
        }
    }

    #[cfg(debug_assertions)]
    pub fn tag_(&self) -> &StaticType<'src> {
        match self {
            Self::Lit(value) | Self::Env(_, value) => &value.tag_,
        }
    }
}

impl<'lit, 'env, 'src> EvalValue<'lit, 'env, 'src> {
    pub fn range(self) -> Option<Range<usize>> {
        match self {
            Self::Lit(_) => None,
            Self::Env(range, _) => Some(range),
        }
    }

    pub fn de_env(self) -> EvalValueNoEnv<'lit, 'src> {
        match self {
            Self::Lit(value) => EvalValueNoEnv::Lit(value),
            Self::Env(range, _) => EvalValueNoEnv::Env(range),
        }
    }
}

impl<'src> RuntimeOp<'src> {
    fn eval<'lit, 'env>(
        &'lit self,
        env: &'env mut Environment<'_>,
    ) -> Result<(Range<usize>, Value<'env, 'src>), Failure> {
        match *self {
            RuntimeOp::Binary(ref out_ty_, ref def, out_size, ref lhs, ref rhs) => {
                // reserve the temporary so it doesn't get trampled by later operations
                let (out_range, _) = env
                    .temp(out_size)
                    .unwrap_or_else(|| todo!("error: out of memory"));

                let lhs = lhs.eval(env)?;
                let lhs_tag_ = lhs.tag_().clone();
                let lhs = lhs.de_env();

                let rhs = rhs.eval(env)?;
                let rhs_tag_ = rhs.tag_().clone();
                let rhs = rhs.de_env();

                env.stack_pop(out_range); // TBD: is it safe to pop it here? will it affect how we dereference them?

                debug_info!("evaluating op({lhs_tag_}, {rhs_tag_}) -> {out_ty_}");

                let (out, lhs, rhs) = match (lhs, rhs) {
                    (EvalValueNoEnv::Lit(lhs), EvalValueNoEnv::Lit(rhs)) => {
                        let out = env.get_mut(out_range).unwrap(/* TBD */);
                        (out, lhs.data, rhs.data)
                    }
                    (EvalValueNoEnv::Lit(lhs), EvalValueNoEnv::Env(rhs_range)) => {
                        debug_assert!(
                            out_range.start >= rhs_range.end || rhs_range.start >= out_range.end,
                            "memory overlap\n out_range: {out_range:?}\n rhs_range: {rhs_range:?}\n (lhs: literal)"
                        );
                        let [out, rhs] =
                            env.get_disjoint_mut([out_range, rhs_range]).unwrap(/* TBD */);
                        (out, lhs.data, &*rhs)
                    }
                    (EvalValueNoEnv::Env(lhs_range), EvalValueNoEnv::Lit(rhs)) => {
                        debug_assert!(
                            out_range.start >= lhs_range.end || lhs_range.start >= out_range.end,
                            "memory overlap\n out_range: {out_range:?}\n lhs_range: {lhs_range:?}\n (rhs: literal)"
                        );
                        let [out, lhs] =
                            env.get_disjoint_mut([out_range, lhs_range]).unwrap(/* TBD */);
                        (out, &*lhs, rhs.data)
                    }
                    (EvalValueNoEnv::Env(lhs_range), EvalValueNoEnv::Env(rhs_range)) => {
                        debug_assert!(
                            out_range.start >= lhs_range.end || lhs_range.start >= out_range.end,
                            "memory overlap\n out_range: {out_range:?}\n lhs_range: {lhs_range:?}\n (rhs_range: {rhs_range:?})"
                        );
                        debug_assert!(
                            out_range.start >= rhs_range.end || rhs_range.start >= out_range.end,
                            "memory overlap\n out_range: {out_range:?}\n rhs_range: {rhs_range:?}\n (lhs_range: {lhs_range:?})"
                        );
                        debug_assert!(
                            lhs_range.start >= rhs_range.end || rhs_range.start >= lhs_range.end,
                            "memory overlap\n lhs_range: {lhs_range:?}\n rhs_range: {rhs_range:?}\n (out_range: {out_range:?})"
                        );
                        let [out, lhs, rhs] = env.get_disjoint_mut([out_range, lhs_range, rhs_range]).unwrap(/* TBD */);
                        (out, &*lhs, &*rhs)
                    }
                };
                def(
                    ValueMut::new(
                        out,
                        #[cfg(debug_assertions)]
                        out_ty_.clone(),
                    ),
                    Value::new(
                        lhs,
                        #[cfg(debug_assertions)]
                        lhs_tag_,
                    ),
                    Value::new(
                        rhs,
                        #[cfg(debug_assertions)]
                        rhs_tag_,
                    ),
                )?;
                Ok((
                    out_range,
                    Value::new(
                        out,
                        #[cfg(debug_assertions)]
                        out_ty_.clone(),
                    ),
                ))
            }

            RuntimeOp::UnaryPre(ref out_ty_, ref def, out_size, ref rhs) => {
                todo!()
            }

            RuntimeOp::UnaryPost(ref out_ty_, ref def, out_size, ref lhs) => {
                todo!()
            }
        }
    }
}

impl<'src> Literal {
    fn eval(&self) -> Value<'_, 'src> {
        match self {
            Literal::Nevr => unimplemented!("nevr cannot be accessed"), // TBD: should this be a Failure instead?
            Literal::None => Value::cast_from(&()),
            Literal::Fail(val) => Value::cast_from(val),
            Literal::Bool(val) => Value::cast_from(val),
            Literal::Byte(val) => Value::cast_from(val),
            Literal::Uint(val) => Value::cast_from(val),
            Literal::Sint(val) => Value::cast_from(val),
            Literal::Flpt(val) => Value::cast_from(val),
            Literal::Char(val) => Value::cast_from(val),
            Literal::Text(val) => Value::cast_from(val),
        }
    }
}

impl<'src> RuntimeVar<'src> {
    fn eval<'env>(&self, env: &'env mut Environment<'_>) -> (Range<usize>, Value<'env, 'src>) {
        env.stack_push(self.range);
        let value = Value::new(
            env.get(self.range)
                .expect("should be guaranteed by static analysis"),
            #[cfg(debug_assertions)]
            self.ty_.clone(),
        );
        (self.range, value)
    }
}

impl<'src> Expr<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn eval<'lit, 'env>(
        &'lit self,
        env: &'env mut Environment<'_>,
    ) -> Result<EvalValue<'lit, 'env, 'src>, Failure> {
        match self {
            Self::Op(x) => x.eval(env).map(|(range, val)| EvalValue::Env(range, val)),

            Self::Literal(x) => Ok(EvalValue::Lit(x.eval())),

            Self::Directive(_) => todo!(),

            Self::Variable(x) => {
                let (range, val) = x.eval(env);
                Ok(EvalValue::Env(range, val))
            }

            // TBD: should assignment even be stored in Expr?
            Self::Assignment(_) => unreachable!("should be handled by outer ExprStmt"),
        }
    }
}

impl<'src> Assignment<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        let lvalue = match &self.lvalue {
            Expr::Variable(var) => var,
            Expr::Literal(_) => {
                panic!("cannot assign to literal - should be caught by static analysis")
            }
            _ => todo!("mutable expressions"),
        };
        let rhs = self.rhs.eval(env)?;
        match rhs {
            EvalValue::Lit(rhs) => {
                let lvalue = env
                    .get_mut(lvalue.range)
                    .unwrap(/* TBD */)
                .copy_from_slice(rhs.data);
            }

            EvalValue::Env(rhs_range, _) => {
                let [lvalue, &mut ref rhs] = env
                    .get_disjoint_mut([lvalue.range, rhs_range])
                    .unwrap(/* TBD */);
                lvalue.copy_from_slice(rhs);
            }
        }
        Ok(())
    }
}

impl<'src> ExprStmt<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        match &self.expr {
            Expr::Assignment(assignment) => assignment.exec(env),

            expr => expr.eval(env).map(|val| {
                let val = val.value();
                #[cfg(debug_assertions)]
                if val.tag_ != StaticType::Primitive(crate::analyzer::types::PrimitiveType::None) {
                    const WARN_MUST_USE: bool = true;
                    if WARN_MUST_USE {
                        debug_warn!(
                            "expression evaluating to {} is having its output ignored\n value: {val:?}\n expr: {expr:?}",
                            val.tag_
                        );
                    }
                }
            }),
        }
    }
}

impl<'src> PrintStmt<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        let res = self.expr.eval(env)?.value();
        // SAFETY: static analyzer must ensure soundness
        let msg = unsafe { res.cast_into::<text>() };
        // SAFETY: eval() promises to initialize its return
        let msg = unsafe { msg.assume_init() };
        print!("{msg}");
        Ok(())
    }
}

impl<'src> IfStmt<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        let mut if_stmt = self;
        loop {
            let res = if_stmt.pred.eval(env)?.value();
            // SAFETY: static analyzer must ensure soundness
            let val = unsafe { res.cast_into::<bool>() };
            // SAFETY: eval promises to initialize its output
            break if unsafe { val.assume_init() } {
                &if_stmt.then
            } else if let Some(or) = if_stmt.or.as_deref() {
                match or {
                    OrStmt::More(next) => {
                        if_stmt = next;
                        continue;
                    }
                    OrStmt::Last(block_stmt) => block_stmt,
                }
            } else {
                break Ok(());
            }
            .exec(env);
        }
    }
}

impl<'src> LoopStmt<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        loop {
            self.block.exec(env)?;
        }
    }
}

impl HaltStmt {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        todo!()
    }
}

impl<'src> BlockStmt<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Statement<RuntimeOp<'src>, RuntimeVar<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        match self {
            Self::Expr(stmt) => stmt.exec(env),
            Self::Print(stmt) => stmt.exec(env),
            Self::If(stmt) => stmt.exec(env),
            Self::Loop(stmt) => stmt.exec(env),
            Self::Cord(_) => todo!(),
            Self::For(_) => todo!(),
            Self::Rep(_) => todo!(),
            Self::Halt(_) => todo!(),
            Self::Skip(_) => todo!(),
            Self::Block(stmt) => stmt.exec(env),
        }
    }
}

pub fn execute(prgm: &RuntimeProgram<'_>) -> Result<(), Failure> {
    const UNINIT_VALUE: u8 = 0xee;
    const BUF_SIZE: usize = 1024; // TODO: how much memory do we give it?
    let mut buf = vec![UNINIT_VALUE; BUF_SIZE].into_boxed_slice();
    let mut env = Environment::new(buf.as_mut());
    for stmt in &prgm.stmts {
        stmt.exec(&mut env)?;
    }
    Ok(())
}
