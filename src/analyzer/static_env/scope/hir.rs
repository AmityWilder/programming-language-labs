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
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PrimitiveType {
    #[default]
    None,
    Nevr,
    Fail,
    Bool,
    Byte,
    Uint,
    Sint,
    Flpt,
    Char,
    Text,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespacePathPart {
    /// Identifier
    pub name: RelSpan, // TODO
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typename {
    pub path: Box<[NamespacePathPart]>,
    /// Identifier
    pub name: RelSpan, // TODO
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericGroup(pub Box<[TypeExpr]>);

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInstance {
    pub name: Typename,
    pub args: Option<GenericGroup>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum OrType {
    #[default]
    None,
    Fail,
    Nevr,
}

/// Fallible Type
#[derive(Debug, Clone, PartialEq)]
pub struct FallType {
    pub then_ty: TypeExpr,
    pub ty: OrType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefType {
    // TBD: reference mutability?
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewType {
    pub item: TypeExpr,
    pub count: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssocTypeSource {
    Simple(Typename),
    Grouped(TypeExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssocType {
    pub base: AssocTypeSource,
    pub assoc: TypeInstance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunType {
    pub params: Box<[TypeExpr]>,
    pub ret: Option<Box<TypeExpr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Primitive(PrimitiveType),
    Instance(TypeInstance),
    Fallible(Box<FallType>),
    Ref(Box<RefType>),
    View(Box<ViewType>),
    Assoc(Box<AssocType>),
    FunPtr(FunType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingPattern {
    pub var: VarId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickPattern {
    // TODO
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
    Block(BlockExpr),
    Binary(Box<Binary>),
    Unary(Box<Unary>),
    Literal(ast::Literal),
    Directive(Directive),
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
pub enum Directive {
    Inpt,
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
    pub arms: Vec<PickArm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickArm {
    pub pat: PickPattern,
    pub expr: BlockExpr,
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
    pub pred: Expr,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub var: BindingPattern,
    pub iter: Expr,
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
