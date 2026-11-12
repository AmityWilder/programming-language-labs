//! Runtime Interpretor Representation

use crate::{
    analyzer::static_env::VarId,
    error::fail::Failure,
    lex::token::Operator,
    types::{byte, flpt, sint, text, uint},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LabelId(u32);

#[derive(Debug, Clone, PartialEq, Hash, Default)]
pub enum CtrlFlow {
    #[default]
    Continue,
    Done {
        until: LabelId,
        value: Option<Expr>,
    },
    Skip {
        until: LabelId,
    },
}

#[derive(Debug)]
pub struct InterpEnvironment {
    // TODO
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Branch {
    /// Should be non-empty
    pub arms: Vec<BranchArm>,
    pub last: Option<Block>,
}

impl Branch {
    pub fn eval(&mut self, env: &mut InterpEnvironment) -> Result<CtrlFlow, Failure> {
        for arm in &mut self.arms {
            if arm.cond.eval() {
                arm.eval()
            }
        }
        if let Some(block) = self.last {
            block.eval()
        } else {
            Ok(CtrlFlow::Continue)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct BranchArm {
    pub cond: Expr,
    pub then: Block,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Loop {
    pub label: LabelId,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Stmt {
    Expr(Expr),
    Print(Expr),
    Branch(Branch),
    Loop(Loop),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Operation {
    pub op: Operator,
    pub operands: Box<[Expr]>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunVal {
    None,
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Flpt(flpt),
    Char(char),
    Text(text),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub val: RunVal,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Directive {
    Input,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Variable {
    pub id: VarId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Op(Operation),
    Lit(Literal),
    Dir(Directive),
    Var(Variable),
}

impl Expr {
    pub fn eval(&mut self, env: &mut InterpEnvironment) -> Result<RunVal, Failure> {
        match self {
            Expr::Op(operation) => todo!(),
            Expr::Lit(literal) => todo!(),
            Expr::Dir(directive) => todo!(),
            Expr::Var(variable) => todo!(),
        }
    }
}
