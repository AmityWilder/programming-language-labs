//! Type Intermediate Representation

use crate::{
    analyzer::static_env::{ItemId, TypeId, VarId},
    lex::token::{Operator, punc::OpSide},
    parse::ast,
    src_code::span::{FileId, RelSpan},
};

#[derive(Debug, Clone, PartialEq)]
pub struct HirUnit {
    pub file: FileId,
    pub stmts: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingPattern {
    pub var: VarId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Rec(DeclRec),
    Cat(DeclCat),
    Uni(DeclUni),
    Let(DeclLet),
    Pvt(DeclPvt),
    Defer(DeclDefer),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclRec {
    pub id: TypeId,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub id: ItemId, // TODO: these should be treated like unique items
    pub ty: ast::TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclCat {
    pub id: TypeId, // TODO: these should be treated like unique items
    pub discriminants: Vec<Discriminant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Discriminant {
    pub id: ItemId,
    pub val: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclUni {
    pub id: VarId,
    pub ty: Option<ast::TypeExpr>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary(Box<Binary>),
    Unary(Box<Unary>),
    Literal(Literal),
    Variable(Variable),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub op: Operator,
    pub lhs: Expr,
    pub rhs: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary {
    pub op: Operator,
    pub operand: Expr,
    pub side: OpSide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    // TODO
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub id: VarId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockExpr {
    If(Box<IfStmt>),
    Loop(Box<LoopStmt>),
    Cord(Box<CordStmt>),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclPvt {
    pub id: VarId,
    pub ty: Option<ast::TypeExpr>,
    pub init: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclLet {
    // TODO: complex bindings
    pub id: VarId,
    pub ty: Option<ast::TypeExpr>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclDefer {
    pub param: Option<VarId>,
    pub on_pop: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub pred: Expr,
    pub then: BlockStmt,
    pub or: Option<Box<OrStmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt {
    More(IfStmt),
    Last(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickExpr {
    pub arms: Vec<PickExprArm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickExprArm {
    pub pat: BindingPattern,
    pub expr: BlockExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickStmt {
    pub arms: Vec<PickStmtArm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickStmtArm {
    pub pat: BindingPattern,
    pub block: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub name: RelSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt {
    pub label: Option<Label>,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CordStmt {
    pub label: Option<Label>,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepStmt {
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoneStmt {
    pub label: Option<Label>,
    pub expr: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt {
    pub label: Option<Label>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    pub decls: Vec<Decl>,
}
