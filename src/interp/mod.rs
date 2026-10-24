//! UNDER CONSTRUCTION
//!
//! Less elegant and less performant than [`crate::exec::eval`], but also does not rely on `unsafe` code.

// TBD: should debug represent runtime debug or internal debug? (currently uses runtime debug)

use crate::{
    analyzer::{
        Directive, Expr, InterpAssignment, InterpBlockStmt, InterpCordStmt, InterpDeferNode,
        InterpExpr, InterpExprStmt, InterpForStmt, InterpHaltStmt, InterpIfStmt, InterpLoopStmt,
        InterpPrintStmt, InterpProgram, InterpRepStmt, InterpSkipStmt, InterpStatement, Literal,
        OrStmt,
    },
    exec::{debug_warn, error::Failure},
    interp::value::{InterpValue, InterpVarValue},
    scanner::token::value::Keyword,
    types::{byte, text},
};
use env::InterpEnvironment;
use std::num::NonZero;
use value::{
    BoolObject, ByteObject, CharObject, FailObject, FlptObject, InterpLabel, InterpOp, InterpVal,
    NoneObject, PrimitiveVal, SintObject, TextObject, UintObject,
};

pub mod env;
pub mod value;

#[derive(Debug, Default)]
pub enum ControlFlow<'src> {
    /// Keep going as usual
    #[default]
    Continue,
    /// Skip to the next iteration of the current loop
    Skip(Option<InterpLabel<'src>>),
    /// End the current loop
    Halt(Option<InterpLabel<'src>>, Option<InterpVarValue<'src>>),
    /// End the current function
    Give(InterpVarValue<'src>),
    /// Produce an exception
    Fail(Failure),
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
    ) -> Result<InterpValue<'env, 'src>, Failure> {
        match self {
            Self::Binary(_, fun, lhs, rhs) => {
                let lhs = lhs.interp_eval(env)?;
                let rhs = rhs.interp_eval(env)?;
                fun(lhs, rhs)
            }
            Self::UnaryPre(_, fun, rhs) => {
                let rhs = rhs.interp_eval(env)?;
                fun(rhs)
            }
            Self::UnaryPost(_, fun, lhs) => {
                let lhs = lhs.interp_eval(env)?;
                fun(lhs)
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
    fn input<R: std::io::BufRead>(stream: R) -> Result<Self, Failure>;
}

impl Inpt for text {
    fn input<R: std::io::BufRead>(mut stream: R) -> Result<Self, Failure> {
        let mut buf = String::new();
        // TODO: what if we want to receive just a character instead of an entire line?
        // TODO: handle error
        stream
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
    fn input<R: std::io::BufRead>(mut stream: R) -> Result<Self, Failure> {
        let mut buf = [0u8; char::MAX_LEN_UTF8];

        // SAFETY: buf is not empty
        let first = unsafe { buf.get_unchecked_mut(0) };

        if stream.read_exact(std::slice::from_mut(first)).is_err() {
            return Err(Failure::FailedRead);
        }

        let Some(n) = NonZero::new(core::str::utf8_char_width(*first)) else {
            return Err(Failure::FailedRead);
        };
        debug_assert!(n.get() <= buf.len(), "the full char must fit in buf");

        // SAFETY: buf.len() is the maximum utf8 char width (len>=n) and n is non-zero (n>=1)
        let seg = unsafe { buf.get_unchecked_mut(1..n.get()) };

        if stream.read_exact(seg).is_err() {
            return Err(Failure::FailedRead);
        }

        // SAFETY: buf.len() is the maximum utf8 char width (len>=n) and n is non-zero (n>=1)
        let seg = unsafe { buf.get_unchecked_mut(..n.get()) };

        if let Ok(s) = str::from_utf8(seg) {
            Ok(s.chars().next().expect("should have at least one char"))
        } else {
            Err(Failure::FailedRead)
        }
    }
}

impl Inpt for byte {
    fn input<R: std::io::BufRead>(mut stream: R) -> Result<Self, Failure> {
        let mut buf = b'\0';
        if stream.read_exact(std::slice::from_mut(&mut buf)).is_ok() {
            Ok(buf)
        } else {
            Err(Failure::FailedRead)
        }
    }
}

impl Directive {
    fn interp_eval(self) -> Result<InterpVal<'static>, Failure> {
        match self {
            Self::InptText => text::input(std::io::stdin().lock())
                .map(|x| InterpVal::Primitive(PrimitiveVal::Text(TextObject(x)))),

            Self::InptChar => char::input(std::io::stdin().lock())
                .map(|x| InterpVal::Primitive(PrimitiveVal::Char(CharObject(x)))),

            Self::InptByte => byte::input(std::io::stdin().lock())
                .map(|x| InterpVal::Primitive(PrimitiveVal::Byte(ByteObject(x)))),
        }
    }
}

impl<'src> InterpExpr<'src> {
    fn interp_eval<'brw>(
        &self,
        env: &'brw InterpEnvironment<'src>,
    ) -> Result<InterpValue<'brw, 'src>, Failure> {
        match self {
            Expr::Block(_expr) => todo!("block expressions"),
            Expr::Assignment(_) => unimplemented!("should be handled by outer statement"),
            Expr::Op(expr) => expr.interp_eval(env),
            Expr::Literal(expr) => Ok(InterpValue::Prvalue(expr.interp_eval())),
            Expr::Directive(expr) => expr.interp_eval().map(InterpValue::Prvalue),
            Expr::Variable(x) => Ok(InterpValue::Lvalue(
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
        let rvalue = self.rhs.interp_eval(env)?.to_owned();
        let lvalue = env
            .vars
            .get(&var.id)
            .expect("should be guaranteed by static analysis");
        *lvalue.borrow_mut() = rvalue;
        Ok(ControlFlow::Continue)
    }
}

impl<'src> InterpExprStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        if let Expr::Assignment(assignment) = &self.expr {
            assignment.interp_exec(env)
        } else {
            let res = self.expr.interp_eval(env)?.into_ref();
            let res = res.as_ref();
            if !matches!(res, InterpVal::Primitive(PrimitiveVal::None(_))) {
                // println!("WARNING: non-none expression result ignored: {res:?}"); // lint
            }
            Ok(ControlFlow::Continue)
        }
    }
}

impl<'src> InterpPrintStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        let buf = self.expr.interp_eval(env)?.into_ref();
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
    pred: &InterpExpr<'src>,
    env: &mut InterpEnvironment<'src>,
) -> Result<bool, Failure> {
    let buf = pred.interp_eval(env)?.into_ref();
    match <&bool>::try_from(buf.as_ref().ensure_ok()?) {
        Ok(&x) => Ok(x),
        Err(()) => panic!(
            "should be guaranteed by static analysis\
            \n expected: bool\
            \n    found: {buf:?}"
        ),
    }
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
                // TODO: handle value yeilded by skip
                ControlFlow::Continue | ControlFlow::Skip(None) => {}
                ControlFlow::Skip(Some(label)) if label_eq(self.label, label) => {}

                // TODO: handle value yeilded by halt
                ControlFlow::Halt(None, _) => break Ok(ControlFlow::Continue),
                ControlFlow::Halt(Some(label), _) if label_eq(self.label, label) => {
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
                // TODO: handle value yeilded by skip
                ControlFlow::Skip(None) => {}
                ControlFlow::Skip(Some(label)) if label_eq(self.label, label) => {}

                // TODO: handle value yeilded by halt
                ControlFlow::Continue | ControlFlow::Halt(None, _) => {
                    break Ok(ControlFlow::Continue);
                }
                ControlFlow::Halt(Some(label), _) if label_eq(self.label, label) => {
                    break Ok(ControlFlow::Continue);
                }

                flow => break Ok(flow),
            }
        }
    }
}

impl<'src> InterpForStmt<'src> {
    fn interp_exec(
        &self,
        _env: &mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
                // TODO: handle value yeilded by skip
                ControlFlow::Continue | ControlFlow::Skip(None) => (),
                ControlFlow::Skip(Some(label)) if label_eq(self.label, label) => (),

                // TODO: handle value yeilded by halt
                ControlFlow::Halt(None, _) => break Ok(ControlFlow::Continue),
                ControlFlow::Halt(Some(label), _) if label_eq(self.label, label) => {
                    break Ok(ControlFlow::Continue);
                }

                flow => break Ok(flow),
            }
        }
    }
}

impl<'src> InterpHaltStmt<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        self.value
            .as_ref()
            .map(|x| x.interp_eval(env).map(|v| env.get_ptr(v)))
            .transpose()
            .map(|value| ControlFlow::Halt(self.label, value))
    }
}

impl<'src> InterpSkipStmt<'src> {
    const fn interp_exec(&self, _env: &mut InterpEnvironment<'src>) -> ControlFlow<'src> {
        ControlFlow::Skip(self.label)
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

impl<'src> InterpDeferNode<'src> {
    fn interp_exec(&self, env: &mut InterpEnvironment<'src>) -> Result<ControlFlow<'src>, Failure> {
        // defer takes precedence over inner, unless inner fails or the defer has no output, in which case inner wins.

        let inner_res = self.inner.interp_exec(env);
        // TODO: pass inner_res into self.ret_param

        // even if inner errors, we still want to unwind
        let defer_res = self.on_pop.interp_exec(env);

        // TBD: what's the difference between Err and ControlFlow::Fail? does it need to be preserved?
        if let Err(inner_err) | Ok(ControlFlow::Fail(inner_err)) = inner_res {
            // inner error wins, since any defer errors may have cascaded from inner error.
            return Err(inner_err);
        }

        match defer_res {
            // TBD: what's the difference between Err and ControlFlow::Fail? does it need to be preserved?
            Err(defer_err) | Ok(ControlFlow::Fail(defer_err)) => {
                // inner didn't error, since it would have been handled earlier.
                // if defer fails, and inner doesn't, defer always wins.
                Err(defer_err)
            }

            Ok(ControlFlow::Continue) => {
                // defer has no output, inner wins.
                inner_res
            }

            Ok(defer_res) => {
                // defer has a non-error output, defer wins.
                if let Ok(ControlFlow::Skip(_) | ControlFlow::Halt(_, _) | ControlFlow::Give(_)) =
                    inner_res
                {
                    // TBD: what to do with the inner result? is discarding it the most logical plan?
                    debug_warn!(
                        "if `{0}` yeilds a result, then the result of the rest of the scope is consumed! \
                        give the `{0}` a parameter to pass the result into the defer block.",
                        Keyword::Defer
                    );
                }
                Ok(defer_res)
            }
        }
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
            Self::Skip(stmt) => Ok(stmt.interp_exec(env)),
            Self::Block(stmt) => stmt.interp_exec(env),
            Self::Defer(node) => node.interp_exec(env),
        }
    }
}

pub fn interpret<'src>(
    prgm: &InterpProgram<'src>,
    mut env: InterpEnvironment<'src>,
) -> Result<(), Failure> {
    for stmt in &prgm.stmts {
        let flow = stmt.interp_exec(&mut env)?;
        if !matches!(flow, ControlFlow::Continue) {
            todo!("should not halt without a loop or cord\n {flow:?}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod inpt_char_tests {
    use super::*;

    #[test]
    fn test_first_char() {
        let stream = b"abc".as_slice();
        assert_eq!(char::input(stream), Ok('a'));
    }

    #[test]
    fn test_leading_continuation() {
        let stream = b"\x80".as_slice();
        assert_eq!(char::input(stream), Err(Failure::FailedRead));
    }

    #[test]
    fn test_early_end() {
        let stream = b"".as_slice();
        assert_eq!(char::input(stream), Err(Failure::FailedRead));
    }

    #[test]
    fn test_bad_overlong() {
        let stream = b"\xC0\x9F".as_slice();
        assert_eq!(char::input(stream), Err(Failure::FailedRead));
    }

    #[test]
    fn test_oversize_multibyte() {
        let stream = b"\xF4\x90".as_slice();
        assert_eq!(char::input(stream), Err(Failure::FailedRead));
    }
}
