//! The representation of the program that will actually be interpreted

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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum BuiltinOp {
    InptB,
    InptC,
    InptT,

    ExistsN,
    ExistsF,
    ExistsL,
    ExistsB,
    ExistsU,
    ExistsS,
    ExistsC,
    ExistsT,

    // TODO: how will other views be indexed?
    IndexT,
    SliceT,

    NotL,
    NotB,
    NotU,
    NotS,

    AndL,
    AndB,
    AndU,
    AndS,

    OrL,
    OrB,
    OrU,
    OrS,

    XorL,
    XorB,
    XorU,
    XorS,

    NandL,
    NandB,
    NandU,
    NandS,

    NorL,
    NorB,
    NorU,
    NorS,

    XnorL,
    XnorB,
    XnorU,
    XnorS,

    EqL,
    EqB,
    EqU,
    EqS,
    EqC,
    EqT,

    NeL,
    NeB,
    NeU,
    NeS,
    NeC,
    NeT,

    LtL,
    LtB,
    LtU,
    LtS,
    LtC,
    LtT,

    GtL,
    GtB,
    GtU,
    GtS,
    GtC,
    GtT,

    LeL,
    LeB,
    LeU,
    LeS,
    LeC,
    LeT,

    GeL,
    GeB,
    GeU,
    GeS,
    GeC,
    GeT,

    NegS,
    NegF,

    AddB,
    AddU,
    AddS,
    AddF,

    SubB,
    SubU,
    SubS,
    SubF,

    MulB,
    MulU,
    MulS,
    MulF,

    DivB,
    DivU,
    DivS,

    RemB,
    RemU,
    RemS,

    PowBB,
    PowBU,
    PowUB,
    PowUU,
    PowSB,
    PowSU,
    PowFB,
    PowFU,
    PowFF,

    ShlBB,
    ShlBU,
    ShlBS,
    ShlUB,
    ShlUU,
    ShlUS,
    ShlSB,
    ShlSU,
    ShlSS,

    ShrBB,
    ShrBU,
    ShrBS,
    ShrUB,
    ShrUU,
    ShrUS,
    ShrSB,
    ShrSU,
    ShrSS,

    RotlBB,
    RotlBU,
    RotlBS,
    RotlUB,
    RotlUU,
    RotlUS,
    RotlSB,
    RotlSU,
    RotlSS,

    RotrBB,
    RotrBU,
    RotrBS,
    RotrUB,
    RotrUU,
    RotrUS,
    RotrSB,
    RotrSU,
    RotrSS,
}

impl BuiltinOp {
    pub fn eval(self, env: &mut Environment, args: &[Expr]) -> Result<Value, Failure> {
        use BuiltinOp::*;
        Ok(match (self, args) {
            (InptB, []) => todo!("byte input"),
            (InptC, []) => todo!("char input"),
            (InptT, []) => todo!("text input"),

            (ExistsN | ExistsF, [_]) => Value::Bool(false),
            (ExistsL, [x]) => Value::Bool(x.eval_bool(env)?),
            (ExistsB, [x]) => Value::Bool(x.eval_byte(env)? != 0),
            (ExistsU, [x]) => Value::Bool(x.eval_uint(env)? != 0),
            (ExistsS, [x]) => Value::Bool(x.eval_sint(env)? != 0),
            (ExistsC, [x]) => Value::Bool(x.eval_char(env)? != '\0'),
            (ExistsT, [x]) => Value::Bool(!x.eval_text(env)?.is_empty()),

            // TBD: is x a character index or a byte index?
            (IndexT, [x]) => todo!(),
            // TBD: range?
            (SliceT, [x]) => todo!(),

            (NotL, [x]) => Value::Bool(!x.eval_bool(env)?),
            (NotB, [x]) => Value::Byte(!x.eval_byte(env)?),
            (NotU, [x]) => Value::Uint(!x.eval_uint(env)?),
            (NotS, [x]) => Value::Sint(!x.eval_sint(env)?),

            // short-circuiting
            (AndL | OrL | XorL | NandL | NorL | XnorL, [a, b]) => {
                let res = match self {
                    AndL | NandL => a.eval_bool(env)? && b.eval_bool(env)?,
                    OrL | NorL => a.eval_bool(env)? || b.eval_bool(env)?,
                    XorL | XnorL => a.eval_bool(env)? != b.eval_bool(env)?,

                    _ => unreachable!(),
                };
                let is_neg = matches!(self, NandL | NorL | XnorL);
                Value::Bool(res ^ is_neg)
            }

            (AndB, [a, b]) => Value::Byte(a.eval_byte(env)? & b.eval_byte(env)?),
            (AndU, [a, b]) => Value::Uint(a.eval_uint(env)? & b.eval_uint(env)?),
            (AndS, [a, b]) => Value::Sint(a.eval_sint(env)? & b.eval_sint(env)?),

            (OrB, [a, b]) => Value::Byte(a.eval_byte(env)? | b.eval_byte(env)?),
            (OrU, [a, b]) => Value::Uint(a.eval_uint(env)? | b.eval_uint(env)?),
            (OrS, [a, b]) => Value::Sint(a.eval_sint(env)? | b.eval_sint(env)?),

            (XorB, [a, b]) => Value::Byte(a.eval_byte(env)? ^ b.eval_byte(env)?),
            (XorU, [a, b]) => Value::Uint(a.eval_uint(env)? ^ b.eval_uint(env)?),
            (XorS, [a, b]) => Value::Sint(a.eval_sint(env)? ^ b.eval_sint(env)?),

            (NandB, [a, b]) => Value::Byte(!(a.eval_byte(env)? & b.eval_byte(env)?)),
            (NandU, [a, b]) => Value::Uint(!(a.eval_uint(env)? & b.eval_uint(env)?)),
            (NandS, [a, b]) => Value::Sint(!(a.eval_sint(env)? & b.eval_sint(env)?)),

            (NorB, [a, b]) => Value::Byte(!(a.eval_byte(env)? | b.eval_byte(env)?)),
            (NorU, [a, b]) => Value::Uint(!(a.eval_uint(env)? | b.eval_uint(env)?)),
            (NorS, [a, b]) => Value::Sint(!(a.eval_sint(env)? | b.eval_sint(env)?)),

            (XnorB, [a, b]) => Value::Byte(!(a.eval_byte(env)? ^ b.eval_byte(env)?)),
            (XnorU, [a, b]) => Value::Uint(!(a.eval_uint(env)? ^ b.eval_uint(env)?)),
            (XnorS, [a, b]) => Value::Sint(!(a.eval_sint(env)? ^ b.eval_sint(env)?)),

            (EqL, [a, b]) => Value::Bool(a.eval_bool(env)? == b.eval_bool(env)?),
            (EqB, [a, b]) => Value::Bool(a.eval_byte(env)? == b.eval_byte(env)?),
            (EqU, [a, b]) => Value::Bool(a.eval_uint(env)? == b.eval_uint(env)?),
            (EqS, [a, b]) => Value::Bool(a.eval_sint(env)? == b.eval_sint(env)?),
            (EqC, [a, b]) => Value::Bool(a.eval_char(env)? == b.eval_char(env)?),
            (EqT, [a, b]) => Value::Bool(a.eval_text(env)? == b.eval_text(env)?),

            (NeL, [a, b]) => Value::Bool(a.eval_bool(env)? != b.eval_bool(env)?),
            (NeB, [a, b]) => Value::Bool(a.eval_byte(env)? != b.eval_byte(env)?),
            (NeU, [a, b]) => Value::Bool(a.eval_uint(env)? != b.eval_uint(env)?),
            (NeS, [a, b]) => Value::Bool(a.eval_sint(env)? != b.eval_sint(env)?),
            (NeC, [a, b]) => Value::Bool(a.eval_char(env)? != b.eval_char(env)?),
            (NeT, [a, b]) => Value::Bool(a.eval_text(env)? != b.eval_text(env)?),

            (LtB, [a, b]) => Value::Bool(a.eval_byte(env)? < b.eval_byte(env)?),
            (LtU, [a, b]) => Value::Bool(a.eval_uint(env)? < b.eval_uint(env)?),
            (LtS, [a, b]) => Value::Bool(a.eval_sint(env)? < b.eval_sint(env)?),
            (LtC, [a, b]) => Value::Bool(a.eval_char(env)? < b.eval_char(env)?),
            (LtT, [a, b]) => Value::Bool(a.eval_text(env)? < b.eval_text(env)?),

            (GtB, [a, b]) => Value::Bool(a.eval_byte(env)? > b.eval_byte(env)?),
            (GtU, [a, b]) => Value::Bool(a.eval_uint(env)? > b.eval_uint(env)?),
            (GtS, [a, b]) => Value::Bool(a.eval_sint(env)? > b.eval_sint(env)?),
            (GtC, [a, b]) => Value::Bool(a.eval_char(env)? > b.eval_char(env)?),
            (GtT, [a, b]) => Value::Bool(a.eval_text(env)? > b.eval_text(env)?),

            (LeB, [a, b]) => Value::Bool(a.eval_byte(env)? <= b.eval_byte(env)?),
            (LeU, [a, b]) => Value::Bool(a.eval_uint(env)? <= b.eval_uint(env)?),
            (LeS, [a, b]) => Value::Bool(a.eval_sint(env)? <= b.eval_sint(env)?),
            (LeC, [a, b]) => Value::Bool(a.eval_char(env)? <= b.eval_char(env)?),
            (LeT, [a, b]) => Value::Bool(a.eval_text(env)? <= b.eval_text(env)?),

            (GeB, [a, b]) => Value::Bool(a.eval_byte(env)? >= b.eval_byte(env)?),
            (GeU, [a, b]) => Value::Bool(a.eval_uint(env)? >= b.eval_uint(env)?),
            (GeS, [a, b]) => Value::Bool(a.eval_sint(env)? >= b.eval_sint(env)?),
            (GeC, [a, b]) => Value::Bool(a.eval_char(env)? >= b.eval_char(env)?),
            (GeT, [a, b]) => Value::Bool(a.eval_text(env)? >= b.eval_text(env)?),

            (NegS, [x]) => Value::Sint(-x.eval_sint(env)?),
            (NegF, [x]) => Value::Flpt(-x.eval_flpt(env)?),

            (AddB, [a, b]) => Value::Byte(a.eval_byte(env)? + b.eval_byte(env)?),
            (AddU, [a, b]) => Value::Uint(a.eval_uint(env)? + b.eval_uint(env)?),
            (AddS, [a, b]) => Value::Sint(a.eval_sint(env)? + b.eval_sint(env)?),
            (AddF, [a, b]) => Value::Flpt(a.eval_flpt(env)? + b.eval_flpt(env)?),

            (SubB, [a, b]) => Value::Byte(a.eval_byte(env)? - b.eval_byte(env)?),
            (SubU, [a, b]) => Value::Uint(a.eval_uint(env)? - b.eval_uint(env)?),
            (SubS, [a, b]) => Value::Sint(a.eval_sint(env)? - b.eval_sint(env)?),
            (SubF, [a, b]) => Value::Flpt(a.eval_flpt(env)? - b.eval_flpt(env)?),

            (MulB, [a, b]) => Value::Byte(a.eval_byte(env)? * b.eval_byte(env)?),
            (MulU, [a, b]) => Value::Uint(a.eval_uint(env)? * b.eval_uint(env)?),
            (MulS, [a, b]) => Value::Sint(a.eval_sint(env)? * b.eval_sint(env)?),
            (MulF, [a, b]) => Value::Flpt(a.eval_flpt(env)? * b.eval_flpt(env)?),

            (DivB, [a, b]) => Value::Byte(a.eval_byte(env)? / b.eval_byte(env)?),
            (DivU, [a, b]) => Value::Uint(a.eval_uint(env)? / b.eval_uint(env)?),
            (DivS, [a, b]) => Value::Sint(a.eval_sint(env)? / b.eval_sint(env)?),

            (RemB, [a, b]) => Value::Byte(a.eval_byte(env)? % b.eval_byte(env)?),
            (RemU, [a, b]) => Value::Uint(a.eval_uint(env)? % b.eval_uint(env)?),
            (RemS, [a, b]) => Value::Sint(a.eval_sint(env)? % b.eval_sint(env)?),

            (PowBB, [a, b]) => todo!(),
            (PowBU, [a, b]) => todo!(),
            (PowUB, [a, b]) => todo!(),
            (PowUU, [a, b]) => todo!(),
            (PowSB, [a, b]) => todo!(),
            (PowSU, [a, b]) => todo!(),
            (PowFB, [a, b]) => todo!(),
            (PowFU, [a, b]) => todo!(),
            (PowFF, [a, b]) => todo!(),

            (ShlBB, [a, b]) => todo!(),
            (ShlBU, [a, b]) => todo!(),
            (ShlBS, [a, b]) => todo!(),
            (ShlUB, [a, b]) => todo!(),
            (ShlUU, [a, b]) => todo!(),
            (ShlUS, [a, b]) => todo!(),
            (ShlSB, [a, b]) => todo!(),
            (ShlSU, [a, b]) => todo!(),
            (ShlSS, [a, b]) => todo!(),

            (ShrBB, [a, b]) => todo!(),
            (ShrBU, [a, b]) => todo!(),
            (ShrBS, [a, b]) => todo!(),
            (ShrUB, [a, b]) => todo!(),
            (ShrUU, [a, b]) => todo!(),
            (ShrUS, [a, b]) => todo!(),
            (ShrSB, [a, b]) => todo!(),
            (ShrSU, [a, b]) => todo!(),
            (ShrSS, [a, b]) => todo!(),

            (RotlBB, [a, b]) => todo!(),
            (RotlBU, [a, b]) => todo!(),
            (RotlBS, [a, b]) => todo!(),
            (RotlUB, [a, b]) => todo!(),
            (RotlUU, [a, b]) => todo!(),
            (RotlUS, [a, b]) => todo!(),
            (RotlSB, [a, b]) => todo!(),
            (RotlSU, [a, b]) => todo!(),
            (RotlSS, [a, b]) => todo!(),

            (RotrBB, [a, b]) => todo!(),
            (RotrBU, [a, b]) => todo!(),
            (RotrBS, [a, b]) => todo!(),
            (RotrUB, [a, b]) => todo!(),
            (RotrUU, [a, b]) => todo!(),
            (RotrUS, [a, b]) => todo!(),
            (RotrSB, [a, b]) => todo!(),
            (RotrSU, [a, b]) => todo!(),
            (RotrSS, [a, b]) => todo!(),

            // TODO: this will also match any new operators added. thats bad.
            _ => panic!("invalid operands; {STATIC_GUARANTEE}"),
        })
    }
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

#[derive(Debug, Clone, PartialEq, PartialOrd /*, Default*/)]
pub struct Skip {
    pub label: Todo,
}

#[derive(Debug, Clone, PartialEq, PartialOrd /*, Default*/)]
pub struct Done {
    pub label: Todo,
    pub value: Expr,
}
