//! The representation of the program that will actually be interpreted

use super::ControlFlow;
pub use super::builtin_op::BuiltinOp;
use crate::{
    error::fail::Failure,
    exec::{EnvId, FunId, LblId, Value, env::Environment, funcs::FuncTable},
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
    pub funs: FuncTable,
    pub stmts: Box<[Stmt]>,
}

impl Program {
    pub fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &self.stmts {
            match stmt.eval(&self.funs, env) {
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
    pub fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        match self {
            Self::Expr(x) => x.eval(funs, env),
            Self::Loop(x) => x.eval(funs, env),
            Self::Dfer(x) => x.eval(funs, env),
            Self::Branch(x) => x.eval(funs, env),
            Self::Give(x) => x.eval(funs, env),
            Self::Fail(x) => ControlFlow::Err(x.eval(funs, env)),
            Self::Skip(x) => x.eval(funs, env),
            Self::Done(x) => x.eval(funs, env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    Access(EnvId),
    Operation(Operation),
    Block(Box<BlockExpr>),
}

impl Default for Expr {
    fn default() -> Self {
        Self::Literal(Value::None)
    }
}

impl Expr {
    pub fn eval(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, Value, Value> {
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
            Self::Operation(x) => x.eval(funs, env),
            Self::Block(x) => x.eval(funs, env),
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

    pub fn eval_none(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, none, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::None => (),
            x => panic!("expected none, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_fail(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, fail, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Fail(x) => x,
            x => panic!("expected fail, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_bool(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, bool, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Bool(x) => x,
            x => panic!("expected bool, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_byte(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, byte, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Byte(x) => x,
            x => panic!("expected byte, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_uint(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, uint, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Uint(x) => x,
            x => panic!("expected uint, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_sint(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, sint, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Sint(x) => x,
            x => panic!("expected sint, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_flpt(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, flpt, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Flpt(x) => x,
            x => panic!("expected flpt, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_char(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, char, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Char(x) => x,
            x => panic!("expected char, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_text(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, text, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Text(x) => x,
            x => panic!("expected text, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }

    pub fn eval_var(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
    ) -> ControlFlow<Failure, EnvId, Value> {
        self.eval(funs, env).map(|x| match x {
            Value::Var(x) => x,
            x => panic!("expected var, found {x:?}. {STATIC_GUARANTEE}"),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    pub what: Callable,
    pub args: Box<[Expr]>,
}

impl Operation {
    pub fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        self.what.eval(funs, env, &self.args)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Callable {
    Builtin(BuiltinOp),
    UserDefined(FunId),
}

impl Callable {
    pub fn eval(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
        args: &[Expr],
    ) -> ControlFlow<Failure, Value> {
        match self {
            Callable::Builtin(op) => op.eval(funs, env, args),
            // TODO: what about global variables?
            Callable::UserDefined(id) => funs[*id].eval(funs, env, args),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    pub params: Vec<EnvId>,
    pub body: Block,
}

impl Func {
    pub fn eval(
        &self,
        funs: &FuncTable,
        env: &mut Environment,
        args: &[Expr],
    ) -> ControlFlow<Failure, Value> {
        let Self { params, body } = self;
        let sp = env.stack_ptr();
        // TODO: offsets
        for (param, arg) in params.iter().zip(args) {
            let val = arg.eval(funs, env)?;
            env.insert(*param, val);
        }
        let res = match body.eval(funs, env) {
            res @ (ControlFlow::Ok(_) | ControlFlow::Err(_)) => res,
            ControlFlow::Return { value } => ControlFlow::Ok(value),

            ControlFlow::Skip { .. } => todo!("error: tried to `skip` out of a function"),
            ControlFlow::Break { .. } => {
                todo!("error: tried to `done` out of a function")
            }
        };
        // TODO: copy from stack
        let res = match res {
            ControlFlow::Ok(x) => ControlFlow::Ok(if let Value::Var(id) = x {
                env.get(id).expect("id should be valid").clone()
            } else {
                x
            }),
            res => res,
        };
        env.pop_stack(sp);
        res
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockExpr {
    Loop(Loop),
    Branch(Branch),
    Block(Block),
}

impl BlockExpr {
    pub fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        match self {
            Self::Loop(x) => x.eval(funs, env),
            Self::Branch(x) => x.eval(funs, env),
            Self::Block(x) => x.eval(funs, env),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub label: Option<LblId>,
    pub block: Block,
}

impl Loop {
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { label, block } = self;
        loop {
            match block.eval(funs, env) {
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
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let res = self.do_first.eval(funs, env);
        if let Some(id) = self.ret_param {
            match res {
                ControlFlow::Ok(value) => {
                    env.insert(id, value);
                    self.on_pop.eval(funs, env)
                }
                ControlFlow::Err(e) => {
                    env.insert(id, Value::Fail(e));
                    self.on_pop.eval(funs, env)
                }
                ControlFlow::Skip { label } => {
                    env.insert(id, Value::None);
                    let res = self.on_pop.eval(funs, env)?; // TODO: dfer should not
                    debug_assert_eq!(res, Value::None, "non-none dfer result in skip");
                    ControlFlow::Skip { label }
                }
                ControlFlow::Break { label, value } => {
                    env.insert(id, value);
                    let value = self.on_pop.eval(funs, env)?; // TODO: dfer should not
                    ControlFlow::Break { label, value }
                }
                ControlFlow::Return { value } => {
                    env.insert(id, value);
                    let value = self.on_pop.eval(funs, env)?; // TODO: dfer should not
                    ControlFlow::Return { value }
                }
            }
        } else {
            let on_pop_res = self.on_pop.eval(funs, env)?;
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
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        match self {
            Self::Cases(x) => x.eval(funs, env),
            Self::Table(x) => x.eval(funs, env),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cases {
    /// Trailing `or` case should have its predicate be a tautology
    pub cases: Box<[Case]>,
}

impl Cases {
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { cases } = self;
        for Case { pred, then } in cases {
            if pred.eval_bool(funs, env)? {
                return then.eval(funs, env);
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
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self {
            expr,
            transformer,
            thens,
        } = self;
        let val = expr.eval(funs, env)?;
        let idx = transformer.find(val);
        debug_assert!(idx < thens.len(), "{STATIC_GUARANTEE}");
        thens[idx].eval(funs, env)
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
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let mut res = Value::None;
        for stmt in &self.stmts {
            res = stmt.eval(funs, env)?;
        }
        ControlFlow::Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Give {
    pub value: Expr,
}

impl Give {
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { value } = self;
        match value.eval(funs, env) {
            ControlFlow::Ok(value) => ControlFlow::Return { value },
            ControlFlow::Err(e) => ControlFlow::Err(e),
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fail {
    pub value: Expr,
}

impl Fail {
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> Failure {
        match self.value.eval_text(funs, env) {
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
    fn eval(&self, _funs: &FuncTable, _env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { label } = self;
        ControlFlow::Skip { label: *label }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Done {
    pub label: Option<LblId>,
    pub value: Expr,
}

impl Done {
    fn eval(&self, funs: &FuncTable, env: &mut Environment) -> ControlFlow<Failure, Value> {
        let Self { label, value } = self;
        match value.eval(funs, env) {
            ControlFlow::Ok(value) => ControlFlow::Break {
                label: *label,
                value,
            },
            ControlFlow::Err(e) => ControlFlow::Err(e),
            _ => todo!(),
        }
    }
}
