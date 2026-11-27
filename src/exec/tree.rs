//! The representation of the program that will actually be interpreted

use super::ControlFlow;
pub use super::builtin_op::BuiltinOp;
use crate::{
    error::fail::Failure,
    exec::{EnvId, LblId, Value, env::Environment},
    todo_types::Todo,
    types::{byte, fail, flpt, none, sint, text, uint},
};
use std::collections::HashMap;

const STATIC_GUARANTEE: &str = "should be guaranteed by static analysis";

#[cfg(debug_assertions)]
thread_local! {
    /// For debugging
    static DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub stmts: Box<[Stmt]>,
}

impl Program {
    pub fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &self.stmts {
            match stmt.eval(env) {
                ControlFlow::Ok(res) => {
                    debug_assert_eq!(res, Value::None, "non-none output from statement");
                }
                ControlFlow::Err(e) => return Err(e),

                ControlFlow::Skip { .. } => todo!("error: attempted to `skip` outside of a loop"),
                ControlFlow::Break { .. } => todo!("error: attempted to `done` outside of a loop"),
                ControlFlow::Return { .. } => {
                    todo!("error: attempted to `give` outside of a function")
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Loop(Loop),
    Dfer(Dfer),
    Branch(Branch),
    Give(Give),
    Fail(Fail),
    Skip(Skip),
    Done(Done),
}

impl Stmt {
    pub fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        match self {
            Self::Expr(x) => x.eval(env),
            Self::Loop(x) => x.eval(env),
            Self::Dfer(x) => x.eval(env),
            Self::Branch(x) => x.eval(env),
            Self::Give(x) => x.eval(env),
            Self::Fail(x) => ControlFlow::Err(x.eval(env)),
            Self::Skip(x) => x.eval(env),
            Self::Done(x) => x.eval(env),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Expr {
    Literal(Value),
    Access(EnvId),
    Operation(Operation),
}

impl Default for Expr {
    fn default() -> Self {
        Self::Literal(Value::None)
    }
}

impl Expr {
    pub fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value, Value> {
        if cfg!(all(debug_assertions, false)) {
            for _ in 0..DEPTH.get() {
                print!("  ");
            }
            println!("<evaluating: {self:?}>");
            DEPTH.update(|n| n + 1);
        }
        let res = match self {
            Self::Literal(x) => ControlFlow::Ok(x.clone()),
            Self::Access(x) => ControlFlow::Ok(Value::Var(*x)),
            Self::Operation(x) => x.eval(env),
        };
        if cfg!(all(debug_assertions, false)) {
            DEPTH.update(|n| n - 1);
            for _ in 0..DEPTH.get() {
                print!("  ");
            }
            println!(" <result: {res:?}>");
        }
        res
    }

    pub fn eval_none(&self, env: &mut Environment) -> ControlFlow<Failure, none, Value> {
        self.eval(env).map(|x| match x {
            Value::None => (),
            x => panic!("expected none, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_fail(&self, env: &mut Environment) -> ControlFlow<Failure, fail, Value> {
        self.eval(env).map(|x| match x {
            Value::Fail(x) => x,
            x => panic!("expected fail, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_bool(&self, env: &mut Environment) -> ControlFlow<Failure, bool, Value> {
        self.eval(env).map(|x| match x {
            Value::Bool(x) => x,
            x => panic!("expected bool, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_byte(&self, env: &mut Environment) -> ControlFlow<Failure, byte, Value> {
        self.eval(env).map(|x| match x {
            Value::Byte(x) => x,
            x => panic!("expected byte, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_uint(&self, env: &mut Environment) -> ControlFlow<Failure, uint, Value> {
        self.eval(env).map(|x| match x {
            Value::Uint(x) => x,
            x => panic!("expected uint, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_sint(&self, env: &mut Environment) -> ControlFlow<Failure, sint, Value> {
        self.eval(env).map(|x| match x {
            Value::Sint(x) => x,
            x => panic!("expected sint, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_flpt(&self, env: &mut Environment) -> ControlFlow<Failure, flpt, Value> {
        self.eval(env).map(|x| match x {
            Value::Flpt(x) => x,
            x => panic!("expected flpt, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_char(&self, env: &mut Environment) -> ControlFlow<Failure, char, Value> {
        self.eval(env).map(|x| match x {
            Value::Char(x) => x,
            x => panic!("expected char, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_text(&self, env: &mut Environment) -> ControlFlow<Failure, text, Value> {
        self.eval(env).map(|x| match x {
            Value::Text(x) => x,
            x => panic!("expected text, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }

    pub fn eval_var(&self, env: &mut Environment) -> ControlFlow<Failure, EnvId, Value> {
        self.eval(env).map(|x| match x {
            Value::Var(x) => x,
            x => panic!("expected var, found {x:?}. {}", STATIC_GUARANTEE),
        })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Operation {
    pub what: Callable,
    pub args: Box<[Expr]>,
}

impl Operation {
    pub fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        match self.what {
            Callable::Builtin(op) => op.eval(env, &self.args),
            Callable::UserDefined(_fun) => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Callable {
    Builtin(BuiltinOp),
    UserDefined(Todo),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub label: Option<LblId>,
    pub block: Block,
}

impl Loop {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { label, block } = self;
        loop {
            match block.eval(env) {
                ControlFlow::Ok(res) => {
                    debug_assert_eq!(res, Value::None, "non-none loop body");
                }

                ControlFlow::Skip { label: target } if target.is_none() || target == *label => {
                    // TODO: jump to the iterator if this is a for loop
                }

                ControlFlow::Break {
                    label: target,
                    value,
                } if target.is_none() || target == *label => {
                    break ControlFlow::Ok(value);
                }

                res => break res,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dfer {
    pub do_first: Block,
    pub ret_param: Option<EnvId>,
    pub on_pop: Block,
}

impl Dfer {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let res = self.do_first.eval(env);
        if let Some(id) = self.ret_param {
            match res {
                ControlFlow::Ok(value) => {
                    env.insert(id, value);
                    self.on_pop.eval(env)
                }
                ControlFlow::Err(e) => {
                    env.insert(id, Value::Fail(e));
                    self.on_pop.eval(env)
                }
                ControlFlow::Skip { label } => {
                    env.insert(id, Value::None);
                    let res = self.on_pop.eval(env)?; // TODO: dfer should not
                    debug_assert_eq!(res, Value::None, "non-none dfer result in skip");
                    ControlFlow::Skip { label }
                }
                ControlFlow::Break { label, value } => {
                    env.insert(id, value);
                    let value = self.on_pop.eval(env)?; // TODO: dfer should not
                    ControlFlow::Break { label, value }
                }
                ControlFlow::Return { value } => {
                    env.insert(id, value);
                    let value = self.on_pop.eval(env)?; // TODO: dfer should not
                    ControlFlow::Return { value }
                }
            }
        } else {
            let on_pop_res = self.on_pop.eval(env)?;
            debug_assert_eq!(
                on_pop_res,
                Value::None,
                "on_pop has a non-none output, but doesn't accept the return as a parameter"
            );
            res
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Branch {
    Cases(Cases),
    Table(JumpTable),
}

impl Branch {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        match self {
            Self::Cases(x) => x.eval(env),
            Self::Table(x) => x.eval(env),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cases {
    /// Trailing `or` case should have its predicate be a tautology
    pub cases: Box<[Case]>,
}

impl Cases {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { cases } = self;
        for Case { pred, then } in cases {
            if pred.eval_bool(env)? {
                return then.eval(env);
            }
        }
        ControlFlow::Ok(Value::None)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    /// Promised by static analysis to output a boolean
    pub pred: Expr,
    pub then: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JumpTable {
    pub expr: Expr,
    pub transformer: JumpLookup,
    pub thens: Box<[Block]>,
}

impl JumpTable {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self {
            expr,
            transformer,
            thens,
        } = self;
        let val = expr.eval(env)?;
        let idx = transformer.find(val);
        debug_assert!(idx < thens.len(), "{STATIC_GUARANTEE}");
        thens[idx].eval(env)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum JumpLookup {
    Bools,

    AdjacentBytes(byte),
    AdjacentUints(uint),
    AdjacentSints(sint),
    AdjacentChars(char),

    SparseBytes(Todo),
    SparseUints(Todo),
    SparseSints(Todo),
    SparseChars(Todo),

    Strings(HashMap<String, usize>),
}

impl JumpLookup {
    fn find(&self, val: Value) -> usize {
        match *self {
            Self::Bools => val.bool().expect(STATIC_GUARANTEE).into(),

            Self::AdjacentBytes(min) => usize::from(val.byte().expect(STATIC_GUARANTEE) - min),
            Self::AdjacentUints(min) => val.uint().expect(STATIC_GUARANTEE) - min,
            Self::AdjacentSints(min) => {
                usize::try_from(val.sint().expect(STATIC_GUARANTEE) - min).expect(STATIC_GUARANTEE)
            }
            Self::AdjacentChars(min) => {
                usize::try_from(u32::from(val.char().expect(STATIC_GUARANTEE)) - u32::from(min))
                    .expect(STATIC_GUARANTEE)
            }

            Self::SparseBytes(_) => todo!(),
            Self::SparseUints(_) => todo!(),
            Self::SparseSints(_) => todo!(),
            Self::SparseChars(_) => todo!(),

            Self::Strings(ref map) => map
                .get(&val.text().expect(STATIC_GUARANTEE))
                .copied()
                .expect("all cases should be covered"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub stmts: Box<[Stmt]>,
}

impl Block {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let mut res = Value::None;
        for stmt in &self.stmts {
            res = stmt.eval(env)?;
        }
        ControlFlow::Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Give {
    pub value: Expr,
}

impl Give {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { value } = self;
        match value.eval(env) {
            ControlFlow::Ok(value) => ControlFlow::Return { value },
            ControlFlow::Err(e) => ControlFlow::Err(e),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Fail {
    pub value: Expr,
}

impl Fail {
    fn eval(&self, env: &mut Environment) -> Failure {
        match self.value.eval_text(env) {
            ControlFlow::Ok(x) => Failure::Other(x),
            ControlFlow::Err(e) => e,
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Skip {
    pub label: Option<LblId>,
}

impl Skip {
    fn eval(&self, _env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { label } = self;
        ControlFlow::Skip { label: *label }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Done {
    pub label: Option<LblId>,
    pub value: Expr,
}

impl Done {
    fn eval(&self, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { label, value } = self;
        match value.eval(env) {
            ControlFlow::Ok(value) => ControlFlow::Break {
                label: *label,
                value,
            },
            ControlFlow::Err(e) => ControlFlow::Err(e),
            _ => todo!(),
        }
    }
}
