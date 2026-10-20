//! UNDER CONSTRUCTION
//!
//! Less elegant and less performant than [`crate::exec::eval`], but also does not rely on `unsafe` code.

// TBD: should debug represent runtime debug or internal debug? (currently uses runtime debug)

use crate::{
    analyzer::{
        Assignment, BlockStmt, Expr, ExprStmt, IfStmt, InterpProgram, Literal, PrintStmt, Statement,
    },
    exec::error::Failure,
};
use env::Environment;
use value::{
    BoolObject, ByteObject, CharObject, FailObject, FlptObject, InterpOp, InterpVar, LangVar,
    NoneObject, SintObject, TextObject, UintObject,
};

pub mod env;
pub mod value;

impl<'src> InterpOp<'src, '_, '_> {
    fn interp_eval<'env>(
        &'inr self,
        env: &'env Environment,
    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
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
    fn interp_eval<'inr>(&'inr self) -> &'inr (dyn 'inr + LangVar) {
        // yuck, boxes for things we have a reference to :(
        match self {
            Self::None => value::NoneObject::new(const { &() }),
            Self::Nevr => unreachable!("cannot have nevr literal"), // TBD: will this change?
            Self::Fail(x) => FailObject::new(x),
            Self::Bool(x) => BoolObject::new(x),
            Self::Byte(x) => ByteObject::new(x),
            Self::Uint(x) => UintObject::new(x),
            Self::Sint(x) => SintObject::new(x),
            Self::Flpt(x) => FlptObject::new(x),
            Self::Char(x) => CharObject::new(x),
            Self::Text(x) => TextObject::new(x),
        }
    }
}

impl<'src, 'inr> InterpVar<'src, 'inr> {
    fn interp_eval(&'inr self) -> &(dyn 'inr + LangVar) {
        // TBD: what is env even for??
        self
    }
}

impl<'src> Expr<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_eval<'inr>(
        &'inr self,
        env: &'inr Environment,
    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
        match self {
            Expr::Assignment(_) => unimplemented!("should be handled by outer statement"),
            Expr::Op(expr) => expr.interp_eval(env),
            Expr::Literal(expr) => Ok(expr.interp_eval()),
            Expr::Variable(expr) => Ok(Box::from(expr.interp_eval(env))),
        }
    }
}

impl<'src> Assignment<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_exec<'inr>(
        &'inr mut self,
        env: &'inr mut Environment,
    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
        let var = match &mut self.lvalue {
            Expr::Variable(var) => var,
            Expr::Literal(_) => todo!("error: cannot assign to literal"),
            _ => todo!("mutable expression"),
        };
        let tmp = self.rhs.interp_eval(env)?;
        *var = tmp;
        todo!()
    }
}

impl<'src> ExprStmt<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        if let Expr::Assignment(assignment) = &mut self.expr {
            assignment.interp_exec(env);
        } else {
            let res = self.expr.interp_eval(env)?;
            if res.as_none().is_none() {
                // lint
                println!("WARNING: non-none expression result ignored: {res:?}");
            }
        }
        Ok(())
    }
}

impl<'src> PrintStmt<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        let val = self.expr.interp_eval(env)?;
        let txt = val.as_text().expect("should be caught by static analysis");
        print!("{txt}");
        Ok(())
    }
}

impl<'src> IfStmt<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        todo!()
    }
}

impl<'src> BlockStmt<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &mut self.stmts {
            stmt.interp_exec(env)?;
        }
        Ok(())
    }
}

impl<'src> Statement<InterpOp<'src, '_, '_>, InterpVar<'src, '_>> {
    fn interp_exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        match self {
            Self::Expr(stmt) => stmt.interp_exec(env),
            Self::Print(stmt) => stmt.interp_exec(env),
            Self::If(stmt) => stmt.interp_exec(env),
            Self::Block(stmt) => stmt.interp_exec(env),
        }
    }
}

pub fn interpret(mut prgm: InterpProgram<'_, '_>) -> Result<(), Failure> {
    let mut env = Environment::new();
    for stmt in &mut prgm.stmts {
        stmt.interp_exec(&mut env)?;
    }
    Ok(())
}
