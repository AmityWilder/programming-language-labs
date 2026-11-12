//! Type Intermediate Representation

use crate::{
    analyzer::static_env::{ItemId, TypeId},
    parse::ast,
    src_code::span::{FileId, RelSpan},
};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct TirUnit {
    pub file: FileId,
    pub stmts: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct BindingPattern {
    pub name: RelSpan,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub enum Decl {
    Uni(DeclUni),
    Let(DeclLet),
    Pvt(DeclPvt),
    Defer(DeclDefer),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct DeclUni {
    pub name: RelSpan,
    pub ty: Option<TypeId>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub enum Expr {}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub enum BlockExpr {
    If(Box<IfStmt>),
    Loop(Box<LoopStmt>),
    Cord(Box<CordStmt>),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct DeclPvt {
    pub name: RelSpan,
    pub ty: Option<TypeId>,
    pub init: Expr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct DeclLet {
    pub binding: BindingPattern,
    pub ty: Option<TypeId>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct DeclDefer {}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub enum Stmt {
    Expr(ExprStmt),
    // TODO: im not too fond of the print keyword
    Print(PrintStmt),
    If(IfStmt),
    Pick(PickExpr),
    Loop(LoopStmt),
    Cord(CordStmt),
    Rep(RepStmt),
    For(ForStmt),
    Done(DoneStmt),
    Skip(SkipStmt),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ExprStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct PrintStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct IfStmt {
    pub pred: Expr,
    pub then: BlockStmt,
    pub or: Option<Box<OrStmt>>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub enum OrStmt {
    More(IfStmt),
    Last(BlockStmt),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct PickExpr {
    pub arms: Vec<PickExprArm>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct PickExprArm {
    pub pat: BindingPattern,
    pub expr: BlockExpr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct PickStmt {
    pub arms: Vec<PickStmtArm>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct PickStmtArm {
    pub pat: BindingPattern,
    pub block: BlockStmt,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct Label {
    pub name: RelSpan,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct LoopStmt {
    pub label: Option<Label>,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct CordStmt {
    pub label: Option<Label>,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct RepStmt {
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ForStmt {
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct DoneStmt {
    pub label: Option<Label>,
    pub expr: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct SkipStmt {
    pub label: Option<Label>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct BlockStmt {
    pub decls: Vec<Decl>,
}
