//! Runtime Interpretor Representation

use crate::{analyzer::static_env::VarId, error::fail::Failure, lex::token::Operator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LabelId(u32);

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Default)]
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
        self
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

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Literal {
    // TODO
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Directive {
    Input,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Variable {
    pub id: VarId,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Expr {
    Op(Operation),
    Lit(Literal),
    Dir(Directive),
    Var(Variable),
}
