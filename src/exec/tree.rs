//! The representation of the program that will actually be interpreted

use super::Type;
use crate::{
    error::fail::Failure,
    exec::{EnvId, Value},
    todo_types::Todo,
};

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

impl Program {
    pub fn run(&self) -> Result<(), Failure> {
        for stmt in &self.stmts {
            stmt.run()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Stmt {
    Expr(Expr),
    Print(Expr),
}

impl Stmt {
    pub fn run(&self) -> Result<(), Failure> {
        match self {
            Self::Expr(x) => x.eval().map(|x| {
                debug_assert_eq!(x, todo!("none"), "static analysis should convert to none")
            }),
            Self::Print(x) => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Expr {
    Operation(Operation),
    Literal(Literal),
    Access(Variable),
}

impl Expr {
    pub fn eval(&self) -> Result<Value, Failure> {
        match self {
            Self::Operation(x) => x.eval(),
            Self::Literal(x) => Ok(x.val.clone()),
            Self::Access(x) => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Operation {
    pub what: Callable,
    pub args: Vec<Expr>,
}

impl Operation {
    pub fn eval(&self) -> Result<Value, Failure> {
        match self.what {
            Callable::Builtin(op) => op.eval(&self.args),
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
    pub fn eval(self, args: &[Expr]) -> Result<Value, Failure> {
        const GUARANTEE: &str = "should be guaranteed by static analysis";
        use BuiltinOp::*;
        // short-circuiting
        Ok(
            if matches!(self, AndL | OrL | XorL | NandL | NorL | XnorL) {
                let mut args = args
                    .iter()
                    .map(|x| x.eval().map(|x| x.bool().expect(GUARANTEE)));
                let mut acc = args.next().expect("should have at least one element")?;
                match self {
                    AndL | NandL => {
                        for arg in args {
                            acc &= arg?;
                        }
                    }
                    OrL | NorL => {
                        for arg in args {
                            acc |= arg?;
                        }
                    }
                    XorL | XnorL => {
                        for arg in args {
                            acc ^= arg?;
                        }
                    }
                    _ => unreachable!(),
                }
                Value::Bool(acc ^ matches!(self, NandL | NorL | XnorL))
            } else {
                match (self, args) {
                    (InptB, []) => todo!(),
                    (InptC, []) => todo!(),
                    (InptT, []) => todo!(),

                    (ExistsN | ExistsF, [_]) => Value::Bool(false),
                    (ExistsL, [x]) => Value::Bool(x.eval()?.bool().expect(GUARANTEE)),
                    (ExistsB, [x]) => Value::Bool(x.eval()?.byte().expect(GUARANTEE) != 0),
                    (ExistsU, [x]) => Value::Bool(x.eval()?.uint().expect(GUARANTEE) != 0),
                    (ExistsS, [x]) => Value::Bool(x.eval()?.sint().expect(GUARANTEE) != 0),
                    (ExistsC, [x]) => Value::Bool(x.eval()?.char().expect(GUARANTEE) != '\0'),
                    (ExistsT, [x]) => Value::Bool(!x.eval()?.text().expect(GUARANTEE).is_empty()),

                    (NotL, [x]) => Value::Bool(!x.eval()?.bool().expect(GUARANTEE)),
                    (NotB, [x]) => Value::Byte(!x.eval()?.byte().expect(GUARANTEE)),
                    (NotU, [x]) => Value::Uint(!x.eval()?.uint().expect(GUARANTEE)),
                    (NotS, [x]) => Value::Sint(!x.eval()?.sint().expect(GUARANTEE)),

                    (AndL | OrL | XorL | NandL | NorL | XnorL, _) => {
                        unreachable!()
                    }

                    (AndB, [a, b]) => Value::Byte(todo!()),
                    (AndU, [a, b]) => Value::Uint(todo!()),
                    (AndS, [a, b]) => Value::Sint(todo!()),
                    (OrB, [a, b]) => Value::Byte(todo!()),
                    (OrU, [a, b]) => Value::Uint(todo!()),
                    (OrS, [a, b]) => Value::Sint(todo!()),
                    (XorB, [a, b]) => Value::Byte(todo!()),
                    (XorU, [a, b]) => Value::Uint(todo!()),
                    (XorS, [a, b]) => Value::Sint(todo!()),
                    (NandB, [a, b]) => Value::Byte(todo!()),
                    (NandU, [a, b]) => Value::Uint(todo!()),
                    (NandS, [a, b]) => Value::Sint(todo!()),
                    (NorB, [a, b]) => Value::Byte(todo!()),
                    (NorU, [a, b]) => Value::Uint(todo!()),
                    (NorS, [a, b]) => Value::Sint(todo!()),
                    (XnorB, [a, b]) => Value::Byte(todo!()),
                    (XnorU, [a, b]) => Value::Uint(todo!()),
                    (XnorS, [a, b]) => Value::Sint(todo!()),

                    (EqL, [a, b]) => Value::Bool(a.eval()? == b.eval()?),
                    (EqB, [a, b]) => Value::Bool(a.eval()? == b.eval()?),
                    (EqU, [a, b]) => Value::Bool(a.eval()? == b.eval()?),
                    (EqS, [a, b]) => Value::Bool(a.eval()? == b.eval()?),
                    (EqC, [a, b]) => Value::Bool(a.eval()? == b.eval()?),
                    (EqT, [a, b]) => Value::Bool(a.eval()? == b.eval()?),
                    (NeL, [a, b]) => Value::Bool(a.eval()? != b.eval()?),
                    (NeB, [a, b]) => Value::Bool(a.eval()? != b.eval()?),
                    (NeU, [a, b]) => Value::Bool(a.eval()? != b.eval()?),
                    (NeS, [a, b]) => Value::Bool(a.eval()? != b.eval()?),
                    (NeC, [a, b]) => Value::Bool(a.eval()? != b.eval()?),
                    (NeT, [a, b]) => Value::Bool(a.eval()? != b.eval()?),
                    (LtL, [a, b]) => Value::Bool(a.eval()? < b.eval()?),
                    (LtB, [a, b]) => Value::Bool(a.eval()? < b.eval()?),
                    (LtU, [a, b]) => Value::Bool(a.eval()? < b.eval()?),
                    (LtS, [a, b]) => Value::Bool(a.eval()? < b.eval()?),
                    (LtC, [a, b]) => Value::Bool(a.eval()? < b.eval()?),
                    (LtT, [a, b]) => Value::Bool(a.eval()? < b.eval()?),
                    (GtL, [a, b]) => Value::Bool(a.eval()? > b.eval()?),
                    (GtB, [a, b]) => Value::Bool(a.eval()? > b.eval()?),
                    (GtU, [a, b]) => Value::Bool(a.eval()? > b.eval()?),
                    (GtS, [a, b]) => Value::Bool(a.eval()? > b.eval()?),
                    (GtC, [a, b]) => Value::Bool(a.eval()? > b.eval()?),
                    (GtT, [a, b]) => Value::Bool(a.eval()? > b.eval()?),
                    (LeL, [a, b]) => Value::Bool(a.eval()? <= b.eval()?),
                    (LeB, [a, b]) => Value::Bool(a.eval()? <= b.eval()?),
                    (LeU, [a, b]) => Value::Bool(a.eval()? <= b.eval()?),
                    (LeS, [a, b]) => Value::Bool(a.eval()? <= b.eval()?),
                    (LeC, [a, b]) => Value::Bool(a.eval()? <= b.eval()?),
                    (LeT, [a, b]) => Value::Bool(a.eval()? <= b.eval()?),
                    (GeL, [a, b]) => Value::Bool(a.eval()? >= b.eval()?),
                    (GeB, [a, b]) => Value::Bool(a.eval()? >= b.eval()?),
                    (GeU, [a, b]) => Value::Bool(a.eval()? >= b.eval()?),
                    (GeS, [a, b]) => Value::Bool(a.eval()? >= b.eval()?),
                    (GeC, [a, b]) => Value::Bool(a.eval()? >= b.eval()?),
                    (GeT, [a, b]) => Value::Bool(a.eval()? >= b.eval()?),

                    (NegS, [x]) => todo!(),
                    (NegF, [x]) => todo!(),
                    (AddB, [a, b]) => todo!(),
                    (AddU, [a, b]) => todo!(),
                    (AddS, [a, b]) => todo!(),
                    (AddF, [a, b]) => todo!(),
                    (SubB, [a, b]) => todo!(),
                    (SubU, [a, b]) => todo!(),
                    (SubS, [a, b]) => todo!(),
                    (SubF, [a, b]) => todo!(),
                    (MulB, [a, b]) => todo!(),
                    (MulU, [a, b]) => todo!(),
                    (MulS, [a, b]) => todo!(),
                    (MulF, [a, b]) => todo!(),
                    (DivB, [a, b]) => todo!(),
                    (DivU, [a, b]) => todo!(),
                    (DivS, [a, b]) => todo!(),
                    (RemB, [a, b]) => todo!(),
                    (RemU, [a, b]) => todo!(),
                    (RemS, [a, b]) => todo!(),
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
                    _ => panic!("invalid operands; {GUARANTEE}"),
                }
            },
        )
    }

    pub const fn operand_count(self) -> usize {
        self.operands().len()
    }

    pub const fn operands(self) -> &'static [Type] {
        use BuiltinOp::*;
        use Type::*;
        #[expect(clippy::match_same_arms)]
        match self {
            InptB => &[],
            InptC => &[],
            InptT => &[],
            ExistsN => &[None],
            ExistsF => &[Flpt],
            ExistsL => &[Bool],
            ExistsB => &[Byte],
            ExistsU => &[Uint],
            ExistsS => &[Sint],
            ExistsC => &[Char],
            ExistsT => &[Text],
            NotL => &[Bool],
            NotB => &[Byte],
            NotU => &[Uint],
            NotS => &[Sint],
            AndL => &[Bool, Bool],
            AndB => &[Byte, Byte],
            AndU => &[Uint, Uint],
            AndS => &[Sint, Sint],
            OrL => &[Bool, Bool],
            OrB => &[Byte, Byte],
            OrU => &[Uint, Uint],
            OrS => &[Sint, Sint],
            XorL => &[Bool, Bool],
            XorB => &[Byte, Byte],
            XorU => &[Uint, Uint],
            XorS => &[Sint, Sint],
            NandL => &[Bool, Bool],
            NandB => &[Byte, Byte],
            NandU => &[Uint, Uint],
            NandS => &[Sint, Sint],
            NorL => &[Bool, Bool],
            NorB => &[Byte, Byte],
            NorU => &[Uint, Uint],
            NorS => &[Sint, Sint],
            XnorL => &[Bool, Bool],
            XnorB => &[Byte, Byte],
            XnorU => &[Uint, Uint],
            XnorS => &[Sint, Sint],
            EqL => &[Bool, Bool],
            EqB => &[Byte, Byte],
            EqU => &[Uint, Uint],
            EqS => &[Sint, Sint],
            EqC => &[Char, Char],
            EqT => &[Text, Text],
            NeL => &[Bool, Bool],
            NeB => &[Byte, Byte],
            NeU => &[Uint, Uint],
            NeS => &[Sint, Sint],
            NeC => &[Char, Char],
            NeT => &[Text, Text],
            LtL => &[Bool, Bool],
            LtB => &[Byte, Byte],
            LtU => &[Uint, Uint],
            LtS => &[Sint, Sint],
            LtC => &[Char, Char],
            LtT => &[Text, Text],
            GtL => &[Bool, Bool],
            GtB => &[Byte, Byte],
            GtU => &[Uint, Uint],
            GtS => &[Sint, Sint],
            GtC => &[Char, Char],
            GtT => &[Text, Text],
            LeL => &[Bool, Bool],
            LeB => &[Byte, Byte],
            LeU => &[Uint, Uint],
            LeS => &[Sint, Sint],
            LeC => &[Char, Char],
            LeT => &[Text, Text],
            GeL => &[Bool, Bool],
            GeB => &[Byte, Byte],
            GeU => &[Uint, Uint],
            GeS => &[Sint, Sint],
            GeC => &[Char, Char],
            GeT => &[Text, Text],
            NegS => &[Sint],
            NegF => &[Flpt],
            AddB => &[Byte, Byte],
            AddU => &[Uint, Uint],
            AddS => &[Sint, Sint],
            AddF => &[Flpt, Flpt],
            SubB => &[Byte, Byte],
            SubU => &[Uint, Uint],
            SubS => &[Sint, Sint],
            SubF => &[Flpt, Flpt],
            MulB => &[Byte, Byte],
            MulU => &[Uint, Uint],
            MulS => &[Sint, Sint],
            MulF => &[Flpt, Flpt],
            DivB => &[Byte, Byte],
            DivU => &[Uint, Uint],
            DivS => &[Sint, Sint],
            RemB => &[Byte, Byte],
            RemU => &[Uint, Uint],
            RemS => &[Sint, Sint],
            PowBB => &[Byte, Byte],
            PowBU => &[Byte, Uint],
            PowUB => &[Uint, Byte],
            PowUU => &[Uint, Uint],
            PowSB => &[Sint, Byte],
            PowSU => &[Sint, Uint],
            PowFB => &[Flpt, Byte],
            PowFU => &[Flpt, Uint],
            PowFF => &[Flpt, Flpt],
            ShlBB => &[Byte, Byte],
            ShlBU => &[Byte, Uint],
            ShlBS => &[Byte, Sint],
            ShlUB => &[Uint, Byte],
            ShlUU => &[Uint, Uint],
            ShlUS => &[Uint, Sint],
            ShlSB => &[Sint, Byte],
            ShlSU => &[Sint, Uint],
            ShlSS => &[Sint, Sint],
            ShrBB => &[Byte, Byte],
            ShrBU => &[Byte, Uint],
            ShrBS => &[Byte, Sint],
            ShrUB => &[Uint, Byte],
            ShrUU => &[Uint, Uint],
            ShrUS => &[Uint, Sint],
            ShrSB => &[Sint, Byte],
            ShrSU => &[Sint, Uint],
            ShrSS => &[Sint, Sint],
            RotlBB => &[Byte, Byte],
            RotlBU => &[Byte, Uint],
            RotlBS => &[Byte, Sint],
            RotlUB => &[Uint, Byte],
            RotlUU => &[Uint, Uint],
            RotlUS => &[Uint, Sint],
            RotlSB => &[Sint, Byte],
            RotlSU => &[Sint, Uint],
            RotlSS => &[Sint, Sint],
            RotrBB => &[Byte, Byte],
            RotrBU => &[Byte, Uint],
            RotrBS => &[Byte, Sint],
            RotrUB => &[Uint, Byte],
            RotrUU => &[Uint, Uint],
            RotrUS => &[Uint, Sint],
            RotrSB => &[Sint, Byte],
            RotrSU => &[Sint, Uint],
            RotrSS => &[Sint, Sint],
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Literal {
    pub val: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable {
    pub id: EnvId,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Loop {
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Dfer {
    pub do_first: Block,
    pub on_pop: Block,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Branch {
    /// Trailing `else` case has a tautological predicate
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Case {
    /// Promised by static analysis to output a boolean
    pub pred: Expr,
    pub then: Block,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}
