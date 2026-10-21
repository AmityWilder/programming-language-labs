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
    interp::env::InterpVar,
    types::text,
};
use env::InterpEnvironment;
use std::borrow::Cow;
use value::{
    BoolObject, ByteObject, CharObject, FailObject, FlptObject, InterpOp, InterpVal, NoneObject,
    PrimitiveVal, SintObject, TextObject, UintObject,
};

pub mod env;
pub mod value;

impl<'src> InterpOp<'src> {
    fn interp_eval<'env>(
        &self,
        env: &'env InterpEnvironment<'src>,
    ) -> Result<InterpVal<'src>, Failure> {
        match self {
            Self::Binary(_, fun, lhs, rhs) => {
                let lhs = lhs.interp_eval(env)?;
                let rhs = rhs.interp_eval(env)?;
                fun(&lhs, &rhs)
            }
            Self::UnaryPre(_, fun, rhs) => {
                let rhs = rhs.interp_eval(env)?;
                fun(&rhs)
            }
            Self::UnaryPost(_, fun, lhs) => {
                let lhs = lhs.interp_eval(env)?;
                fun(&lhs)
            }
        }
    }
}

impl Literal {
    fn interp_eval(&self) -> InterpVal<'static> {
        // yuck, boxes for things we have a reference to :(
        match self {
            Self::None => InterpVal::Primitive(value::PrimitiveVal::None(NoneObject(()))),
            Self::Nevr => unreachable!("cannot have nevr literal"), // TBD: will this change?
            Self::Fail(x) => InterpVal::Primitive(value::PrimitiveVal::Fail(FailObject(*x))),
            Self::Bool(x) => InterpVal::Primitive(value::PrimitiveVal::Bool(BoolObject(*x))),
            Self::Byte(x) => InterpVal::Primitive(value::PrimitiveVal::Byte(ByteObject(*x))),
            Self::Uint(x) => InterpVal::Primitive(value::PrimitiveVal::Uint(UintObject(*x))),
            Self::Sint(x) => InterpVal::Primitive(value::PrimitiveVal::Sint(SintObject(*x))),
            Self::Flpt(x) => InterpVal::Primitive(value::PrimitiveVal::Flpt(FlptObject(*x))),
            Self::Char(x) => InterpVal::Primitive(value::PrimitiveVal::Char(CharObject(*x))),
            Self::Text(x) => InterpVal::Primitive(value::PrimitiveVal::Text(TextObject(x.clone()))),
        }
    }
}

impl<'src> Expr<InterpOp<'src>, InterpVar<'src>> {
    fn interp_eval<'val>(
        &'val self,
        env: &'val InterpEnvironment<'src>,
    ) -> Result<Cow<'val, InterpVal<'src>>, Failure> {
        match self {
            Expr::Assignment(_) => unimplemented!("should be handled by outer statement"),
            Expr::Op(expr) => expr.interp_eval(env).map(Cow::Owned),
            Expr::Literal(expr) => Ok(Cow::Owned(expr.interp_eval())),
            Expr::Variable(x) => Ok(Cow::Borrowed(
                env.vars
                    .get(&x.id)
                    .expect("should be guaranteed by static analysis"),
            )),
        }
    }
}

impl<'src> Assignment<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<(), Failure> {
        let var = match &self.lvalue {
            Expr::Variable(var) => var,
            Expr::Assignment(_) | Expr::Literal(_) => {
                unreachable!("static analysis should prevent assignment to assignment/literal")
            }
            Expr::Op(_) => todo!("mutable expressions"),
        };
        let rhs = self.rhs.interp_eval(env)?.into_owned();
        let lvalue = env
            .vars
            .get_mut(&var.id)
            .expect("should be guaranteed by static analysis");
        // println!("assigning {rhs:?} to {} ({lvalue:?})", var.ident);
        *lvalue = rhs;
        // println!("{} is now {lvalue:?}", var.ident);
        Ok(())
    }
}

impl<'src> ExprStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<(), Failure> {
        if let Expr::Assignment(assignment) = &self.expr {
            assignment.interp_exec(env)
        } else {
            let res = self.expr.interp_eval(env)?;
            if !matches!(res.as_ref(), InterpVal::Primitive(PrimitiveVal::None(_))) {
                // lint
                println!("WARNING: non-none expression result ignored: {res:?}");
            }
            Ok(())
        }
    }
}

impl<'src> PrintStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<(), Failure> {
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
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<(), Failure> {
        let mut cond = self;
        loop {
            let buf = cond.pred.interp_eval(env)?;
            let val: &bool = buf
                .as_ref()
                .try_into()
                .expect("should be guaranteed by static analysis");
            break if *val {
                &cond.then
            } else if let Some(or) = cond.or.as_deref() {
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
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.interp_exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Statement<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<(), Failure> {
        match self {
            Self::Expr(stmt) => stmt.interp_exec(env),
            Self::Print(stmt) => stmt.interp_exec(env),
            Self::If(stmt) => stmt.interp_exec(env),
            Self::Block(stmt) => stmt.interp_exec(env),
        }
    }
}

pub fn interpret<'src>(
    prgm: &InterpProgram<'src>,
    mut env: InterpEnvironment<'src>,
) -> Result<(), Failure> {
    for stmt in &prgm.stmts {
        stmt.interp_exec(&mut env)?;
    }
    Ok(())
}
