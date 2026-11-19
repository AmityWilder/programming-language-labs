//! Type Intermediate Representation

use crate::{
    analyzer::static_env::{FuncId, ItemId, ModId, TypeId, VarId},
    lex::token::{Operator, punc::OpSide},
    parse::ast,
    src_code::span::{FileId, RelSpan},
};

#[derive(Debug, Clone, PartialEq)]
pub struct HirUnit<V> {
    pub file: FileId,
    pub decls: Vec<Decl<V>>,
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
pub struct GenericGroup<V>(pub Box<[TypeExpr<V>]>);

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInstance<V> {
    pub name: Typename,
    pub args: Option<GenericGroup<V>>,
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
pub struct FallType<V> {
    pub then_ty: TypeExpr<V>,
    pub ty: OrType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefType<V> {
    // TBD: reference mutability?
    pub ty: TypeExpr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewType<V> {
    pub item: TypeExpr<V>,
    pub count: Option<Expr<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssocTypeSource<V> {
    Simple(Typename),
    Grouped(TypeExpr<V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssocType<V> {
    pub base: AssocTypeSource<V>,
    pub assoc: TypeInstance<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunType<V> {
    pub params: Box<[TypeExpr<V>]>,
    pub ret: Option<Box<TypeExpr<V>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpaqueType<V>(pub TypeBound<V>);

/// All
#[derive(Debug, Clone, PartialEq)]
pub struct ConjunctionBound<V>(pub Box<[TypeBound<V>]>);

/// Any
#[derive(Debug, Clone, PartialEq)]
pub struct DisjunctionBound<V>(pub Box<[TypeBound<V>]>);

/// One
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionBound<V>(pub Box<[TypeBound<V>]>);

/// Not
#[derive(Debug, Clone, PartialEq)]
pub struct NegativeBound<V>(pub TypeBound<V>);

/// Only
#[derive(Debug, Clone, PartialEq)]
pub struct ExactTypeBound<V>(pub TypeExpr<V>);

/// Has
#[derive(Debug, Clone, PartialEq)]
pub struct MemberBound<V> {
    /// Identifier
    pub member: RelSpan,
    pub req: Option<TypeBound<V>>,
}

/// Shape
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeBound<V>(pub FunType<V>);

/// Supports
#[derive(Debug, Clone, PartialEq)]
pub struct OverloadBound {
    // TODO
}

/// Satisfies
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBound<V> {
    // TBD: should this be a special ID since it never actually appears in the program?
    pub param: VarId,
    pub expr: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBound<V> {
    Conjunction(ConjunctionBound<V>),
    Disjunction(DisjunctionBound<V>),
    Selection(SelectionBound<V>),
    Not(Box<NegativeBound<V>>),
    Exact(ExactTypeBound<V>),
    Has(Box<MemberBound<V>>),
    Shape(ShapeBound<V>),
    Supports(Box<OverloadBound>),
    Satisfies(Box<ExprBound<V>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr<V> {
    Primitive(PrimitiveType),
    Instance(TypeInstance<V>),
    Fallible(Box<FallType<V>>),
    Ref(Box<RefType<V>>),
    View(Box<ViewType<V>>),
    Assoc(Box<AssocType<V>>),
    FunPtr(FunType<V>),
    Opaque(Box<OpaqueType<V>>),
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
pub enum Decl<V> {
    Mod(DeclMod),
    On(DeclOn<V>),
    Fun(DeclFun<V>),
    Rec(DeclRec<V>),
    Cat(DeclCat<V>),
    Uni(DeclUni<V>),
    Let(DeclLet<V>),
    Pvt(DeclPvt<V>),
    Dfer(DeclDfer<V>),
    Stmt(Stmt<V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclMod {
    pub id: ModId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeBoundClause<V> {
    pub target: TypeExpr<V>,
    pub bound: TypeBound<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IffClause<V>(pub Box<[TypeBoundClause<V>]>);

#[derive(Debug, Clone, PartialEq)]
pub struct DeclOn<V> {
    pub generics: Option<GenericGroup<V>>,
    pub target: TypeExpr<V>,
    pub iff: Option<IffClause<V>>,
    pub members: Box<[Decl<V>]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfParam {
    pub id: VarId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclFun<V> {
    pub id: FuncId,
    pub generics: Option<GenericGroup<V>>,
    pub self_param: Option<SelfParam>,
    pub params: Box<[FunParam<V>]>,
    pub ret: Option<TypeExpr<V>>,
    pub iff: Option<IffClause<V>>,
    pub body: Option<BlockStmt<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunParam<V> {
    /// Treated as a member of the function
    pub id: VarId,
    pub ty: TypeExpr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclRec<V> {
    pub id: TypeId,
    pub fields: Box<[Field<V>]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field<V> {
    pub id: ItemId, // TODO: these should be treated like unique items
    pub ty: TypeExpr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclCat<V> {
    pub id: TypeId, // TODO: these should be treated like unique items
    pub discriminants: Box<[Discriminant<V>]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Discriminant<V> {
    pub id: ItemId,
    pub val: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclUni<V> {
    pub id: VarId,
    pub ty: Option<TypeExpr<V>>,
    pub init: Option<Expr<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<V> {
    Block(BlockExpr<V>),
    Binary(Box<Binary<V>>),
    Unary(Box<Unary<V>>),
    Index(Box<Subscript<V>>),
    Call(Box<FnCall<V>>),
    Literal(ast::Literal),
    Directive(Directive),
    Variable(V),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary<V> {
    pub op: Operator,
    pub lhs: Expr<V>,
    pub rhs: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<V> {
    pub op: Operator,
    pub operand: Expr<V>,
    pub side: OpSide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subscript<V> {
    pub arr: Expr<V>,
    pub idx: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnCall<V> {
    pub fun: Expr<V>,
    pub args: Box<[Expr<V>]>,
}

pub use ast::Literal;

#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    Inpt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Variable {
    Unknown(ast::Variable),
    Found(VarId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostDeclVar(pub VarId);

#[derive(Debug, Clone, PartialEq)]
pub enum BlockExpr<V> {
    If(Box<IfStmt<V>>),
    Loop(Box<LoopStmt<V>>),
    Cord(Box<CordStmt<V>>),
    Block(BlockStmt<V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclPvt<V> {
    pub id: VarId,
    pub ty: Option<TypeExpr<V>>,
    pub init: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclLet<V> {
    // TODO: complex bindings
    pub id: VarId,
    pub ty: Option<TypeExpr<V>>,
    pub init: Option<Expr<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclDfer<V> {
    pub param: Option<VarId>,
    pub on_pop: BlockStmt<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<V> {
    Expr(ExprStmt<V>),
    // TODO: im not too fond of the print keyword
    Print(PrintStmt<V>),
    If(IfStmt<V>),
    Pick(PickExpr<V>),
    Loop(LoopStmt<V>),
    Cord(CordStmt<V>),
    Rep(RepStmt<V>),
    For(ForStmt<V>),
    Give(GiveStmt<V>),
    Fail(FailStmt<V>),
    Done(DoneStmt<V>),
    Skip(SkipStmt),
    Block(BlockStmt<V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt<V> {
    pub expr: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt<V> {
    pub expr: Expr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt<V> {
    pub pred: Expr<V>,
    pub then: BlockStmt<V>,
    pub or: Option<Box<OrStmt<V>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrStmt<V> {
    More(IfStmt<V>),
    Last(BlockStmt<V>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickExpr<V> {
    pub arms: Box<[PickArm<V>]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickArm<V> {
    pub pat: PickPattern,
    pub expr: BlockExpr<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    // TODO: create label IDs
    pub name: RelSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt<V> {
    pub label: Option<Label>,
    pub body: BlockStmt<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CordStmt<V> {
    pub label: Option<Label>,
    pub body: BlockStmt<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepStmt<V> {
    pub pred: Expr<V>,
    pub body: BlockStmt<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt<V> {
    pub var: BindingPattern,
    pub iter: Expr<V>,
    pub body: BlockStmt<V>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GiveStmt<V> {
    pub expr: Option<Expr<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FailStmt<V> {
    pub expr: Option<Expr<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoneStmt<V> {
    pub label: Option<Label>,
    pub expr: Option<Expr<V>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt {
    pub label: Option<Label>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<V>(pub Box<[Decl<V>]>);
