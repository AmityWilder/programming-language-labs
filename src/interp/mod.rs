//! UNDER CONSTRUCTION
//!
//! Less elegant and less performant than [`crate::exec::eval`], but also does not rely on `unsafe` code.

// TBD: should debug represent runtime debug or internal debug? (currently uses runtime debug)

use crate::{
    analyzer::{
        Directive, Expr, InterpAssignment, InterpBlockStmt, InterpCordStmt, InterpExprStmt,
        InterpForStmt, InterpHaltStmt, InterpIfStmt, InterpLoopStmt, InterpPrintStmt,
        InterpProgram, InterpRepStmt, InterpSkipStmt, InterpStatement, Literal, OrStmt,
    },
    exec::error::Failure,
    interp::env::InterpVar,
    types::{byte, text},
};
use env::InterpEnvironment;
use std::borrow::Cow;
use value::{
    BoolObject, ByteObject, CharObject, FailObject, FlptObject, InterpLabel, InterpOp, InterpVal,
    NoneObject, PrimitiveVal, SintObject, TextObject, UintObject,
};

pub mod env;
pub mod value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ControlFlow<'src, E = Failure, T = ()> {
    /// Keep going as usual
    #[default]
    Continue,
    /// Skip to the next iteration of the current loop
    Skip(Option<InterpLabel<'src>>),
    /// End the current loop
    Halt(Option<InterpLabel<'src>>),
    /// End the current function
    Give(T),
    // /// End the current function, but keep its information (TBD: is this reasonable?)
    // Emit,
    /// Produce an exception
    Fail(E),
}

impl InterpVal<'_> {
    pub fn ensure_ok(&self) -> Result<&Self, Failure> {
        match self {
            Self::Uninit(_) => Err(Failure::Uninitialized),

            Self::OrNone(x) => match x.as_ref() {
                value::OrVal::Ok(x) => x.ensure_ok(),
                value::OrVal::Or(_) => Err(Failure::UnhandledOrNone),
            },
            Self::OrFail(x) => match x.as_ref() {
                value::OrVal::Ok(x) => x.ensure_ok(),
                value::OrVal::Or(e) => Err(e.0),
            },
            Self::OrNevr(x) => match x.as_ref() {
                value::OrVal::Ok(x) => x.ensure_ok(),
                value::OrVal::Or(_) => Err(Failure::UnhandledOrNevr),
            },

            // TBD: is this the right way to do this?
            Self::Primitive(PrimitiveVal::None(_)) => Err(Failure::UnhandledOrNone),
            Self::Primitive(PrimitiveVal::Fail(e)) => Err(e.0),
            Self::Primitive(PrimitiveVal::Nevr(_)) => Err(Failure::UnhandledOrNevr),

            x => Ok(x),
        }
    }
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

trait Inpt: Sized {
    fn input() -> Result<Self, Failure>;
}

impl Inpt for text {
    fn input() -> Result<Self, Failure> {
        let mut buf = String::new();
        // TODO: what if we want to receive just a character instead of an entire line?
        // TODO: handle error
        std::io::stdin()
            .read_line(&mut buf)
            .map_err(|_| Failure::FailedRead)?;
        if buf.ends_with('\n') {
            buf.pop();
            if buf.ends_with('\r') {
                buf.pop();
            }
        }
        Ok(buf)
    }
}

impl Inpt for char {
    fn input() -> Result<Self, Failure> {
        use std::io::Read;
        let mut buf = [0; char::MAX_LEN_UTF8];
        let mut s = Err(Failure::FailedRead);
        for i in 0..buf.len() {
            // TODO: handle error
            std::io::stdin()
                .read_exact(std::slice::from_mut(&mut buf[i]))
                .map_err(|_| Failure::FailedRead)?;
            s = str::from_utf8(&buf).map_err(|_| Failure::ParseChar);
            if s.is_ok() {
                break;
            }
        }
        s.map(|s| {
            s.chars()
                .next()
                .expect("loop would have failed with FailedRead if the stdin was empty")
        })
    }
}

impl Inpt for byte {
    fn input() -> Result<Self, Failure> {
        use std::io::Read;
        let mut buf = b'\0';
        std::io::stdin()
            .read_exact(std::slice::from_mut(&mut buf))
            .map_err(|_| Failure::FailedRead)
            .map(|()| buf)
    }
}

impl Directive {
    fn interp_eval(self) -> Result<InterpVal<'static>, Failure> {
        match self {
            Self::InptText => {
                text::input().map(|x| InterpVal::Primitive(PrimitiveVal::Text(TextObject(x))))
            }
            Self::InptChar => {
                char::input().map(|x| InterpVal::Primitive(PrimitiveVal::Char(CharObject(x))))
            }
            Self::InptByte => {
                byte::input().map(|x| InterpVal::Primitive(PrimitiveVal::Byte(ByteObject(x))))
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
            Expr::Directive(expr) => expr.interp_eval().map(Cow::Owned),
            Expr::Variable(x) => Ok(Cow::Borrowed(
                env.vars
                    .get(&x.id)
                    .expect("should be guaranteed by static analysis"),
            )),
        }
    }
}

impl<'src> InterpAssignment<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
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

impl<'src> InterpExprStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
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

impl<'src> InterpPrintStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        let buf = self.expr.interp_eval(env)?;
        let val: &text = buf
            .as_ref()
            .ensure_ok()?
            .try_into()
            .expect("should be guaranteed by static analysis");
        print!("{val}");
        Ok(ControlFlow::Continue)
    }
}

/// A predicate that static analysis should guarantee is a boolean
fn expect_bool<'src>(
    pred: &Expr<InterpOp<'src>, InterpVar<'src>>,
    env: &mut InterpEnvironment<'src>,
) -> Result<bool, Failure> {
    let buf = pred.interp_eval(env)?;
    Ok(
        *<&bool>::try_from(buf.as_ref().ensure_ok()?).unwrap_or_else(|()| {
            panic!(
                "should be guaranteed by static analysis\
                \n expected: bool\
                \n    found: {buf:?}"
            )
        }),
    )
}

impl<'src> InterpIfStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        let mut cond = self;
        loop {
            break if expect_bool(&cond.pred, env)? {
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

fn label_eq(mine: Option<InterpLabel<'_>>, flow: InterpLabel<'_>) -> bool {
    mine.is_some_and(|mine| mine.name == flow.name)
}

impl<'src> InterpLoopStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        loop {
            match self.block.interp_exec(env)? {
                // ControlFlow::Skip is consumed by the loop and only serves to let the inner expression skip
                ControlFlow::Continue | ControlFlow::Skip(None) => {}
                ControlFlow::Skip(Some(label)) if label_eq(self.label, label) => {}

                ControlFlow::Halt(None) => break Ok(ControlFlow::Continue),
                ControlFlow::Halt(Some(label)) if label_eq(self.label, label) => {
                    break Ok(ControlFlow::Continue);
                }

                flow => break Ok(flow),
            }
        }
    }
}

impl<'src> InterpCordStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        loop {
            match self.block.interp_exec(env)? {
                // ControlFlow::Skip is the only case in which the cord repeats
                ControlFlow::Skip(None) => {}
                ControlFlow::Skip(Some(label)) if label_eq(self.label, label) => {}

                ControlFlow::Continue | ControlFlow::Halt(None) => break Ok(ControlFlow::Continue),
                ControlFlow::Halt(Some(label)) if label_eq(self.label, label) => {
                    break Ok(ControlFlow::Continue);
                }

                flow => break Ok(flow),
            }
        }
    }
}

impl<'src> InterpForStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        let mut _iterable = self.iter.interp_eval(env)?.ensure_ok()?;
        todo!("how does an expression iterate?")
    }
}

impl<'src> InterpRepStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        loop {
            if !expect_bool(&self.pred, env)? {
                break Ok(ControlFlow::Continue);
            }

            // identical to "loop"
            match self.block.interp_exec(env)? {
                // ControlFlow::Skip is consumed by the loop and only serves to let the inner expression skip
                ControlFlow::Continue | ControlFlow::Skip(None) => (),
                ControlFlow::Skip(Some(label)) if label_eq(self.label, label) => (),

                ControlFlow::Halt(None) => break Ok(ControlFlow::Continue),
                ControlFlow::Halt(Some(label)) if label_eq(self.label, label) => {
                    break Ok(ControlFlow::Continue);
                }

                flow => break Ok(flow),
            }
        }
    }
}

impl<'src> InterpHaltStmt<'src> {
    #[expect(clippy::unnecessary_wraps, reason = "future-proof")]
    const fn interp_exec(
        &self,
        _env: &mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        Ok(ControlFlow::Halt(self.label))
    }
}

impl<'src> InterpSkipStmt<'src> {
    #[expect(clippy::unnecessary_wraps, reason = "future-proof")]
    const fn interp_exec(
        &self,
        _env: &mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        Ok(ControlFlow::Skip(self.label))
    }
}

impl<'src> InterpBlockStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        for stmt in &self.stmts {
            let res = stmt.interp_exec(env)?;
            if !matches!(res, ControlFlow::Continue) {
                return Ok(res);
            }
        }
        Ok(ControlFlow::Continue)
    }
}

impl<'src> InterpStatement<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        match self {
            Self::Expr(stmt) => stmt.interp_exec(env),
            Self::Print(stmt) => stmt.interp_exec(env),
            Self::If(stmt) => stmt.interp_exec(env),
            Self::Loop(stmt) => stmt.interp_exec(env),
            Self::Cord(stmt) => stmt.interp_exec(env),
            Self::For(stmt) => stmt.interp_exec(env),
            Self::Rep(stmt) => stmt.interp_exec(env),
            Self::Halt(stmt) => stmt.interp_exec(env),
            Self::Skip(stmt) => stmt.interp_exec(env),
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
