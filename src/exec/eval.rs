use super::value::{Value, ValueMut};
use crate::{
    analyzer::{
        Assignment, BlockStmt, Expr, ExprStmt, IfStmt, Literal, OrStmt, PrintStmt, Program,
        RunOperation, RunVariable, Statement, types::StaticType,
    },
    exec::{env::Environment, error::Failure},
    types::*,
};
#[cfg(not(debug_assertions))]
use std::marker::PhantomData;

/// Yeilds a value, but does not mutate state
pub trait Evaluate<'src> {
    fn eval<'val>(&'val self, env: &'val mut Environment<'_>)
    -> Result<Value<'val, 'src>, Failure>;
}

/// Mutates state, does not yeild a value
pub trait Execute<'src> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure>;
}

impl<'src> Execute<'src> for Assignment<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        let lvalue = match &self.lvalue {
            Expr::Variable(var) => var,
            Expr::Literal(_) => {
                panic!("cannot assign to literal - should be caught by static analysis")
            }
            _ => todo!("mutable expressions"),
        };
        // TODO: what if a user is trying to assign an earlier variable to a later variable?
        let (_lower, lvalue, mut upper) = env
            .get_mut(lvalue.range)
            .expect("should be handled by static analysis");
        let rvalue = self.rhs.eval(&mut upper)?;
        lvalue.copy_from_slice(rvalue.data);
        Ok(())
    }
}

impl<'src> Evaluate<'src> for RunOperation<'src> {
    fn eval<'val>(
        &'val self,
        env: &'val mut Environment<'_>,
    ) -> Result<Value<'val, 'src>, Failure> {
        match *self {
            RunOperation::Binary(ref ty_, ref def, out_size, ref lhs, ref rhs) => {
                let (out, mut env) = env
                    .temp(out_size)
                    .unwrap_or_else(|| todo!("error: out of memory"));
                let lhs = lhs.eval(&mut env)?;
                let rhs = rhs.eval(&mut env)?;
                def(
                    &env,
                    ValueMut::new(
                        out,
                        #[cfg(debug_assertions)]
                        ty_.clone(),
                    ),
                    lhs,
                    rhs,
                )?;
                Ok(Value::new(
                    out,
                    #[cfg(debug_assertions)]
                    ty_.clone(),
                ))
            }
            RunOperation::UnaryPre(ref ty_, ref def, out_size, ref rhs) => {
                let (out, mut env) = env
                    .temp(out_size)
                    .unwrap_or_else(|| todo!("error: out of memory"));
                let rhs = rhs.eval(&mut env)?;
                def(
                    &env,
                    ValueMut::new(
                        out,
                        #[cfg(debug_assertions)]
                        ty_.clone(),
                    ),
                    rhs,
                )?;
                Ok(Value::new(
                    out,
                    #[cfg(debug_assertions)]
                    ty_.clone(),
                ))
            }
            RunOperation::UnaryPost(ref ty_, ref def, out_size, ref lhs) => {
                let lhs = lhs.eval(env)?;
                let (out, env) = env
                    .temp(out_size)
                    .unwrap_or_else(|| todo!("error: out of memory"));
                def(
                    &env,
                    ValueMut::new(
                        out,
                        #[cfg(debug_assertions)]
                        ty_.clone(),
                    ),
                    lhs,
                )?;
                Ok(Value::new(
                    out,
                    #[cfg(debug_assertions)]
                    ty_.clone(),
                ))
            }
        }
    }
}

impl<'src> Evaluate<'src> for Literal {
    fn eval<'val>(
        &'val self,
        _env: &'val mut Environment<'_>,
    ) -> Result<Value<'val, 'src>, Failure> {
        match self {
            Literal::Nevr => unimplemented!("nevr cannot be accessed"), // TBD: should this be a Failure instead?
            Literal::None => Ok(Value::cast_from(&())),
            Literal::Fail(val) => Ok(Value::cast_from(val)),
            Literal::Bool(val) => Ok(Value::cast_from(val)),
            Literal::Byte(val) => Ok(Value::cast_from(val)),
            Literal::Uint(val) => Ok(Value::cast_from(val)),
            Literal::Sint(val) => Ok(Value::cast_from(val)),
            Literal::Flpt(val) => Ok(Value::cast_from(val)),
            Literal::Char(val) => Ok(Value::cast_from(val)),
            Literal::Text(val) => Ok(Value::cast_from(val)),
        }
    }
}

impl<'src> Evaluate<'src> for RunVariable<'src> {
    fn eval<'val>(
        &'val self,
        env: &'val mut Environment<'_>,
    ) -> Result<Value<'val, 'src>, Failure> {
        Ok(Value::new(
            env.get(self.range)
                .expect("should be guaranteed by static analysis"),
            #[cfg(debug_assertions)]
            self.ty_.clone(),
        ))
    }
}

impl<'src> Evaluate<'src> for Expr<RunOperation<'src>, RunVariable<'src>> {
    fn eval<'val>(
        &'val self,
        env: &'val mut Environment<'_>,
    ) -> Result<Value<'val, 'src>, Failure> {
        match self {
            Self::Assignment(_) => unreachable!("should be handled by outer statement"),
            Self::Op(x) => x.eval(env),
            Self::Literal(x) => x.eval(env),
            Self::Variable(x) => x.eval(env),
        }
    }
}

impl<'src> Execute<'src> for ExprStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        match &self.expr {
            Expr::Assignment(assignment) => assignment.exec(env),

            expr => expr.eval(env).map(|val| {
                #[cfg(debug_assertions)]
                if val.tag_ != StaticType::Primitive(crate::analyzer::types::PrimitiveType::None) {
                    const WARN_MUST_USE: bool = true;
                    if WARN_MUST_USE {
                        println!(
                            "warning: expression evaluating to {} is having its output ignored\n value: {val:?}\n expr: {expr:?}",
                            val.tag_
                        );
                    }
                }
            }),
        }
    }
}

impl<'src> Execute<'src> for PrintStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        let res = self.expr.eval(env)?;
        // SAFETY: static analyzer must ensure soundness
        let msg = unsafe { res.cast_into::<text>() };
        print!("{msg}");
        Ok(())
    }
}

impl<'src> Execute<'src> for IfStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        let mut if_stmt = self;
        loop {
            let res = if_stmt.pred.eval(env)?;
            // SAFETY: static analyzer must ensure soundness
            break if *unsafe { res.cast_into::<bool>() } {
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

impl<'src> Execute<'src> for BlockStmt<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Execute<'src> for Statement<RunOperation<'src>, RunVariable<'src>> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        match self {
            Self::Expr(stmt) => stmt.exec(env),
            Self::Print(stmt) => stmt.exec(env),
            Self::If(stmt) => stmt.exec(env),
            Self::Block(stmt) => stmt.exec(env),
        }
    }
}

impl<'src> Execute<'src> for Program<'src> {
    fn exec(&self, env: &mut Environment<'_>) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

pub fn execute(prgm: &Program<'_>) -> Result<(), Failure> {
    let mut mem = vec![0xeeu8; 1024];
    let mut env = Environment::new(mem.as_mut_slice(), 0); // TODO: how much memory do we give it?
    prgm.exec(&mut env)
}
