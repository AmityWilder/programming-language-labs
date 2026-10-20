//! UNDER CONSTRUCTION
//!
//! Less elegant and less performant than [`crate::exec::eval`], but also does not rely on `unsafe` code.

// TBD: should debug represent runtime debug or internal debug? (currently uses runtime debug)

use crate::{
    analyzer::{
        Assignment, BlockStmt, Expr, ExprStmt, IfStmt, InterpProgram, Literal, OrStmt, PrintStmt,
        Statement,
    },
    exec::error::Failure,
    types::text,
};
use env::Environment;
use std::borrow::Cow;
use value::{
    BoolObject, ByteObject, CharObject, FailObject, FlptObject, InterpOp, InterpVar, NoneObject,
    PrimitiveVar, SintObject, TextObject, UintObject,
};

pub mod env;
pub mod value;

impl<'src> InterpOp<'src> {
    fn interp_eval<'env>(&self, env: &'env Environment) -> Result<InterpVar<'src>, Failure> {
        match self {
            Self::Binary(_, fun, lhs, rhs) => {
                let lhs = lhs.interp_eval(env)?;
                let rhs = rhs.interp_eval(env)?;
                fun(&*lhs, &*rhs)
            }
            Self::UnaryPre(_, fun, rhs) => {
                let rhs = rhs.interp_eval(env)?;
                fun(&*rhs)
            }
            Self::UnaryPost(_, fun, lhs) => {
                let lhs = lhs.interp_eval(env)?;
                fun(&*lhs)
            }
        }
    }
}

impl<'src> Literal {
    fn interp_eval(&self) -> InterpVar<'src> {
        // yuck, boxes for things we have a reference to :(
        match self {
            Self::None => InterpVar::Primitive(value::PrimitiveVar::None(NoneObject(()))),
            Self::Nevr => unreachable!("cannot have nevr literal"), // TBD: will this change?
            Self::Fail(x) => InterpVar::Primitive(value::PrimitiveVar::Fail(FailObject(*x))),
            Self::Bool(x) => InterpVar::Primitive(value::PrimitiveVar::Bool(BoolObject(*x))),
            Self::Byte(x) => InterpVar::Primitive(value::PrimitiveVar::Byte(ByteObject(*x))),
            Self::Uint(x) => InterpVar::Primitive(value::PrimitiveVar::Uint(UintObject(*x))),
            Self::Sint(x) => InterpVar::Primitive(value::PrimitiveVar::Sint(SintObject(*x))),
            Self::Flpt(x) => InterpVar::Primitive(value::PrimitiveVar::Flpt(FlptObject(*x))),
            Self::Char(x) => InterpVar::Primitive(value::PrimitiveVar::Char(CharObject(*x))),
            Self::Text(x) => InterpVar::Primitive(value::PrimitiveVar::Text(TextObject(x.clone()))),
        }
    }
}

impl<'src> Expr<InterpOp<'src>, InterpVar<'src>> {
    fn interp_eval<'val>(
        &'val self,
        env: &'val Environment,
    ) -> Result<Cow<'val, InterpVar<'src>>, Failure> {
        match self {
            Expr::Assignment(_) => unimplemented!("should be handled by outer statement"),
            Expr::Op(expr) => expr.interp_eval(env).map(Cow::Owned),
            Expr::Literal(expr) => Ok(Cow::Owned(expr.interp_eval())),
            Expr::Variable(x) => Ok(Cow::Borrowed(x)),
        }
    }
}

impl<'src> Assignment<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        let var = match &mut self.lvalue {
            Expr::Variable(var) => var,
            Expr::Literal(_) => todo!("error: cannot assign to literal"),
            _ => todo!("mutable expression"),
        };
        let tmp = self.rhs.interp_eval(env)?;
        tmp.as_ref().clone_into(var);
        Ok(())
    }
}

impl<'src> ExprStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        if let Expr::Assignment(assignment) = &mut self.expr {
            assignment.interp_exec(env);
        } else {
            let res = self.expr.interp_eval(env)?;
            if !matches!(res.as_ref(), InterpVar::Primitive(PrimitiveVar::None(_))) {
                // lint
                println!("WARNING: non-none expression result ignored: {res:?}");
            }
        }
        Ok(())
    }
}

impl<'src> PrintStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        let buf = self.expr.interp_eval(env)?;
        let val: &text = buf
            .as_ref()
            .try_into()
            .expect("should be guaranteed by static analysis");
        print!("{val}");
        Ok(())
    }
}

impl<'src> IfStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        let mut cond = self;
        loop {
            let buf = cond.pred.interp_eval(env)?;
            let val: &bool = buf
                .as_ref()
                .try_into()
                .expect("should be guaranteed by static analysis");
            break if *val {
                &mut cond.then
            } else if let Some(or) = cond.or.as_deref_mut() {
                match or {
                    OrStmt::More(more) => {
                        cond = more;
                        continue;
                    }
                    OrStmt::Last(last) => last,
                }
            } else {
                break Ok(());
            }
            .interp_exec(env);
        }
    }
}

impl<'src> BlockStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &mut self.stmts {
            stmt.interp_exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Statement<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        match self {
            Self::Expr(stmt) => stmt.interp_exec(env),
            Self::Print(stmt) => stmt.interp_exec(env),
            Self::If(stmt) => stmt.interp_exec(env),
            Self::Block(stmt) => stmt.interp_exec(env),
        }
    }
}

pub fn interpret(mut prgm: InterpProgram<'_>) -> Result<(), Failure> {
    let mut env = Environment::new();
    for stmt in &mut prgm.stmts {
        stmt.interp_exec(&mut env)?;
    }
    Ok(())
}
