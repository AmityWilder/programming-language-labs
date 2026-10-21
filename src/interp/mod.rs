//! UNDER CONSTRUCTION
//!
//! Less elegant and less performant than [`crate::exec::eval`], but also does not rely on `unsafe` code.

// TBD: should debug represent runtime debug or internal debug? (currently uses runtime debug)

use crate::{
    analyzer::{
        Assignment, BlockStmt, Directive, Expr, ExprStmt, HaltStmt, IfStmt, InterpProgram, Literal,
        LoopStmt, OrStmt, PrintStmt, SkipStmt, Statement,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ControlFlow<E = Failure, T = ()> {
    /// Keep going as usual
    #[default]
    Continue,
    /// Skip to the next iteration of the current loop
    Skip,
    /// End the current loop
    Halt,
    /// End the current function
    Give(T),
    // /// End the current function, but keep its information (TBD: is this reasonable?)
    // Emit,
    /// Produce an exception
    Fail(E),
}

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

impl Directive {
    fn interp_eval(self) -> InterpVal<'static> {
        match self {
            Self::Inpt => {
                let mut buf = String::new();
                // TODO: what if we want to receive just a character instead of an entire line?
                // TODO: handle error
                std::io::stdin().read_line(&mut buf);
                if buf.ends_with('\n') {
                    buf.pop();
                    if buf.ends_with('\r') {
                        buf.pop();
                    }
                }
                InterpVal::Primitive(PrimitiveVal::Text(TextObject(buf)))
            }
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
            Expr::Directive(expr) => Ok(Cow::Owned(expr.interp_eval())),
            Expr::Variable(x) => Ok(Cow::Borrowed(
                env.vars
                    .get(&x.id)
                    .expect("should be guaranteed by static analysis"),
            )),
        }
    }
}

impl<'src> Assignment<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
        let var = match &self.lvalue {
            Expr::Variable(var) => var,
            Expr::Assignment(_) | Expr::Literal(_) => {
                unreachable!("static analysis should prevent assignment to assignment/literal")
            }
            _ => todo!("mutable expressions"),
        };
        let rhs = self.rhs.interp_eval(env)?.into_owned();
        let lvalue = env
            .vars
            .get_mut(&var.id)
            .expect("should be guaranteed by static analysis");
        // println!("assigning {rhs:?} to {} ({lvalue:?})", var.ident);
        *lvalue = rhs;
        // println!("{} is now {lvalue:?}", var.ident);
        Ok(ControlFlow::Continue)
    }
}

impl<'src> ExprStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
        if let Expr::Assignment(assignment) = &self.expr {
            assignment.interp_exec(env)
        } else {
            let res = self.expr.interp_eval(env)?;
            if !matches!(res.as_ref(), InterpVal::Primitive(PrimitiveVal::None(_))) {
                // lint
                println!("WARNING: non-none expression result ignored: {res:?}");
            }
            Ok(ControlFlow::Continue)
        }
    }
}

impl<'src> PrintStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
        let buf = self.expr.interp_eval(env)?;
        let val: &text = buf
            .as_ref()
            .try_into()
            .expect("should be guaranteed by static analysis");
        print!("{val}");
        Ok(ControlFlow::Continue)
    }
}

impl<'src> IfStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
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
                break Ok(ControlFlow::Continue);
            }
            .interp_exec(env);
        }
    }
}

impl<'src> LoopStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
        loop {
            let res = self.block.interp_exec(env)?;
            match res {
                // ControlFlow::Skip is consumed by the loop and only serves to let the inner expression skip
                ControlFlow::Continue | ControlFlow::Skip => (),
                ControlFlow::Halt => break Ok(ControlFlow::Continue),
                flow @ (ControlFlow::Give(()) | ControlFlow::Fail(_)) => break Ok(flow),
            }
        }
    }
}

impl<'src> BlockStmt<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
        for stmt in &self.stmts {
            let res = stmt.interp_exec(env)?;
            if !matches!(res, ControlFlow::Continue) {
                return Ok(res);
            }
        }
        Ok(ControlFlow::Continue)
    }
}

impl<'src> Statement<InterpOp<'src>, InterpVar<'src>> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow, Failure> {
        match self {
            Self::Expr(stmt) => stmt.interp_exec(env),
            Self::Print(stmt) => stmt.interp_exec(env),
            Self::If(stmt) => stmt.interp_exec(env),
            Self::Loop(stmt) => stmt.interp_exec(env),
            Self::Halt(HaltStmt {}) => Ok(ControlFlow::Halt),
            Self::Skip(SkipStmt {}) => Ok(ControlFlow::Skip),
            Self::Block(stmt) => stmt.interp_exec(env),
        }
    }
}

pub fn interpret<'src>(
    prgm: &InterpProgram<'src>,
    mut env: InterpEnvironment<'src>,
) -> Result<(), Failure> {
    for stmt in &prgm.stmts {
        let flow = stmt.interp_exec(&mut env)?;
        assert_eq!(
            flow,
            ControlFlow::Continue,
            "should not halt without a loop or cord"
        );
    }
    Ok(())
}
