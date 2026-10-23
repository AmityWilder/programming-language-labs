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
    exec::error::Failure,
    interp::value::{InterpValue, InterpVarValue},
    types::{byte, text},
};
use env::InterpEnvironment;
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
    Skip(Option<InterpLabel<'src>>, Option<InterpVarValue<'src>>),
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
        for i in 0..buf.len() {
            // SAFETY:
            //
            // Definition 1:
            // Let `S` be the array `[S_0, S_1, ..., S_n]` for some non-negative integer n. Then, `S.len() = n`.
            //
            // Definition 2:
            // Let `h,k` be integers. Then, `h..k` is the range of all integers `n` such that `h <= n < k`.
            //
            // Definition 3:
            // Let `S` be the array `[S_0, S_1, ..., S_n]` for some non-negative integer `n` and let `k` be an integer.
            // Then, `S.get_unchecked_mut(k)` is "sound" if and only if `0 <= k < n`.
            //
            // Definition 4:
            // Let `S` be the array `[S_0, S_1, ..., S_n]` for some non-negative integer `n` and let `h,k` be integers.
            // Then, `S.get_unchecked_mut(h..=k)` is sound if and only if `S.get_unchecked_mut(l)`
            // is sound for all integers `l` such that `h <= l <= k`.
            //
            // Theorem:
            // Let `S` be the array `[S_0, S_1, ..., S_n]` for some integer `n > 0`.
            // Let `k` be an integer such that `0 <= k < n`.
            // Then, `S.get_unchecked_mut(0..=k)` is sound.
            //
            // Proof.
            // Let `S` be the array `[S_0, S_1, ..., S_n]` for some integer `n > 0`.
            // Let `P(l)` be the statement "`S.get_unchecked_mut(0..=l)` is sound".
            // We will show that if `0 <= l < n`, then `P(l)` is true.
            //
            // Base case: `l = 0`
            // Since `0 <= 0` and `n > 0`, we know `0 <= 0 < n`.
            // Therefore, by definition 3, `S.get_unchecked_mut(0)` is sound.
            // Since `S.get_unchecked_mut(m)` is sound for all integers `m` such that `0 <= m <= 0`,
            // by definition 4, `S.get_unchecked_mut(0..=0)` is sound.
            // This proves that `P(0)` is true.
            //
            // Induction step:
            // Assume `P(0), P(1), ..., P(t)` are all true for some integer `t >= 0`.
            // In other words, `S.get_unchecked_mut(0..=t)` is sound.
            // So by definition, `S.get_unchecked_mut(w)` is sound for all integers `w` such that `0 <= w < t`.
            // We will show that if `t + 1 < n`, then `P(t + 1)` is true.
            // Since `t >= 0`, we know `t + 1 >= 0`.
            // If `t + 1 < n`, then `0 <= t + 1 < n`.
            // So by definition 3, `S.get_unchecked_mut(t + 1)` is sound.
            // Since `S.get_unchecked_mut(w)` is sound for all integers `w` such that `0 <= w < t`,
            // and `S.get_unchecked_mut(t + 1)` is sound, `S.get_unchecked_mut(0..=(t + 1))` is sound. □
            let seg = unsafe { buf.get_unchecked_mut(0..=i) };

            let [.., newest] = seg else {
                unreachable!("inclusive range can't be empty")
            };
            if std::io::stdin()
                .read_exact(std::slice::from_mut(newest))
                .is_err()
            {
                break;
            }

            // TBD: what if `seg` starts within a UTF-8 char?
            // then even if a char starting on the next byte is valid, the str as a whole will never be valid
            if let Ok(s) = str::from_utf8(&*seg) {
                let ch = s
                    .chars()
                    .next()
                    .expect("loop would have exited with FailedRead if the stdin was empty");
                return Ok(ch);
            }
        }
        Err(Failure::FailedRead)
    }
}

impl Inpt for byte {
    fn input() -> Result<Self, Failure> {
        use std::io::Read;
        let mut buf = b'\0';
        if std::io::stdin()
            .read_exact(std::slice::from_mut(&mut buf))
            .is_ok()
        {
            Ok(buf)
        } else {
            Err(Failure::FailedRead)
        }
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

impl<'src> InterpExpr<'src> {
    fn interp_eval<'brw>(
        &self,
        env: &'brw InterpEnvironment<'src>,
    ) -> Result<InterpValue<'brw, 'src>, Failure> {
        match self {
            Expr::Block(_expr) => todo!("block expression"),
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
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        loop {
            match self.block.interp_exec(env)? {
                // TODO: handle value yeilded by skip
                ControlFlow::Continue | ControlFlow::Skip(None, _) => {}
                ControlFlow::Skip(Some(label), _) if label_eq(self.label, label) => {}

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
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        loop {
            match self.block.interp_exec(env)? {
                // TODO: handle value yeilded by skip
                ControlFlow::Skip(None, _) => {}
                ControlFlow::Skip(Some(label), _) if label_eq(self.label, label) => {}

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
    fn interp_exec<'brw>(
        &self,
        _env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        todo!("how does an expression iterate?")
    }
}

impl<'src> InterpRepStmt<'src> {
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        loop {
            if !expect_bool(&self.pred, env)? {
                break Ok(ControlFlow::Continue);
            }

            // identical to "loop"
            match self.block.interp_exec(env)? {
                // TODO: handle value yeilded by skip
                ControlFlow::Continue | ControlFlow::Skip(None, _) => (),
                ControlFlow::Skip(Some(label), _) if label_eq(self.label, label) => (),

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
    #[expect(clippy::unnecessary_wraps, reason = "future-proof")]
    const fn interp_exec<'brw>(
        &self,
        _env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        Ok(ControlFlow::Halt(self.label, None)) // TODO
    }
}

impl<'src> InterpSkipStmt<'src> {
    #[expect(clippy::unnecessary_wraps, reason = "future-proof")]
    const fn interp_exec<'brw>(
        &self,
        _env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
        Ok(ControlFlow::Skip(self.label, None)) // TODO
    }
}

impl<'src> InterpBlockStmt<'src> {
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
        let res = self.inner.interp_exec(env);
        if let Some(param) = self.ret_param {
            // *env.vars
            //     .get_mut(param.id)
            //     .expect("should be guaranteed by static analysis") = InterpVal(res);
            self.on_pop.interp_exec(env)
        } else {
            // TBD: if both fail, which failure gets output?
            _ = self.on_pop.interp_exec(env);
            res
        }
    }
}

impl<'src> InterpStatement<'src> {
    fn interp_exec<'brw>(
        &self,
        env: &'brw mut InterpEnvironment<'src>,
    ) -> Result<ControlFlow<'src>, Failure> {
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
            panic!("should not halt without a loop or cord\n {flow:?}");
        }
    }
    Ok(())
}
