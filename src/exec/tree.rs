//! The representation of the program that will actually be interpreted

pub use super::builtin_op::BuiltinOp;
use crate::{
    error::fail::Failure,
    exec::{EnvId, Value, env::Environment},
    todo_types::Todo,
    types::{byte, fail, flpt, none, sint, text, uint},
};

const STATIC_GUARANTEE: &str = "should be guaranteed by static analysis";

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Program {
    pub stmts: Box<[Stmt]>,
}

impl Program {
    pub fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.exec(env)?;
        }
        Ok(())
    }
}

fn eval_to_exec(res: Result<Value, Failure>) -> Result<(), Failure> {
    res.map(|x| {
        #[cfg(debug_assertions)]
        if !matches!(x, Value::None) {
            println!("WARNING: expression has a non-none result that is not handled: {x:?}");
        }
    })
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Stmt {
    Expr(Expr),
    Print(Print),
    Loop(Loop),
    Dfer(Dfer),
    Branch(Branch),
    Give(Give),
    Fail(Fail),
    Skip(Skip),
    Done(Done),
}

impl Stmt {
    pub fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        match self {
            Self::Expr(x) => x.eval(env),
            Self::Print(x) => x.eval(env),
            Self::Loop(x) => x.eval(env),
            Self::Dfer(x) => x.eval(env),
            Self::Branch(x) => x.eval(env),
            Self::Give(_) => todo!(),
            Self::Fail(x) => Err(x.eval(env)),
            Self::Skip(_) => todo!(),
            Self::Done(_) => todo!(),
        }
    }

    pub fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        eval_to_exec(self.eval(env))
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
    pub fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        match self {
            Self::Literal(x) => Ok(x.clone()),
            Self::Access(x) => Ok(Value::Var(*x)),
            Self::Operation(x) => x.eval(env),
        }
    }

    pub fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        eval_to_exec(self.eval(env))
    }

    pub fn eval_none(&self, env: &mut Environment) -> Result<none, Failure> {
        self.eval(env).map(|x| x.none().expect(STATIC_GUARANTEE))
    }

    pub fn eval_fail(&self, env: &mut Environment) -> Result<fail, Failure> {
        self.eval(env).map(|x| x.fail().expect(STATIC_GUARANTEE))
    }

    pub fn eval_bool(&self, env: &mut Environment) -> Result<bool, Failure> {
        self.eval(env).map(|x| x.bool().expect(STATIC_GUARANTEE))
    }

    pub fn eval_byte(&self, env: &mut Environment) -> Result<byte, Failure> {
        self.eval(env).map(|x| x.byte().expect(STATIC_GUARANTEE))
    }

    pub fn eval_uint(&self, env: &mut Environment) -> Result<uint, Failure> {
        self.eval(env).map(|x| x.uint().expect(STATIC_GUARANTEE))
    }

    pub fn eval_sint(&self, env: &mut Environment) -> Result<sint, Failure> {
        self.eval(env).map(|x| x.sint().expect(STATIC_GUARANTEE))
    }

    pub fn eval_flpt(&self, env: &mut Environment) -> Result<flpt, Failure> {
        self.eval(env).map(|x| x.flpt().expect(STATIC_GUARANTEE))
    }

    pub fn eval_char(&self, env: &mut Environment) -> Result<char, Failure> {
        self.eval(env).map(|x| x.char().expect(STATIC_GUARANTEE))
    }

    pub fn eval_text(&self, env: &mut Environment) -> Result<text, Failure> {
        self.eval(env).map(|x| x.text().expect(STATIC_GUARANTEE))
    }

    pub fn eval_var(&self, env: &mut Environment) -> Result<EnvId, Failure> {
        self.eval(env).map(|x| x.var().expect(STATIC_GUARANTEE))
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Operation {
    pub what: Callable,
    pub args: Box<[Expr]>,
}

impl Operation {
    pub fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Print {
    /// Promised by static analysis to output text
    pub msg: Expr,
}
impl Print {
    fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        self.exec(env).map(|()| Value::None)
    }

    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        self.msg.eval_text(env).map(|msg| print!("{msg}"))
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Loop {
    pub block: Block,
}

impl Loop {
    fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        loop {
            // TODO: skip/done/give
            self.block.eval(env)?;
        }
    }

    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        eval_to_exec(self.eval(env))
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Dfer {
    pub do_first: Block,
    pub ret_param: Option<EnvId>,
    pub on_pop: Block,
}

impl Dfer {
    fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        let res = self.do_first.eval(env);
        if let Some(_id) = self.ret_param {
            // assign res to return parameter variable in environment
            self.on_pop.eval(env)
        } else {
            self.on_pop.exec(env)?;
            res
        }
    }

    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        eval_to_exec(self.eval(env))
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Branch {
    pub cases: Box<[Case]>,
    /// Trailing `or` case
    pub default: Option<Block>,
}

impl Branch {
    fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        for case in &self.cases {
            if case.pred.eval_bool(env)? {
                return case.then.eval(env);
            }
        }
        self.default
            .as_ref()
            .map_or(Ok(Value::None), |then| then.eval(env))
    }

    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        eval_to_exec(self.eval(env))
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Case {
    /// Promised by static analysis to output a boolean
    pub pred: Expr,
    pub then: Block,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Block {
    pub stmts: Box<[Stmt]>,
}

impl Block {
    fn eval(&self, env: &mut Environment) -> Result<Value, Failure> {
        let mut res = Value::None;
        for stmt in &self.stmts {
            res = stmt.eval(env)?;
        }
        Ok(res)
    }

    fn exec(&self, env: &mut Environment) -> Result<(), Failure> {
        eval_to_exec(self.eval(env))
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Give {
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Fail {
    pub value: Expr,
}

impl Fail {
    fn eval(&self, env: &mut Environment) -> Failure {
        self.value
            .eval_text(env)
            .map_or_else(std::convert::identity, Failure::Other)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Skip {
    pub label: Option<Todo>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Done {
    pub label: Option<Todo>,
    pub value: Expr,
}
