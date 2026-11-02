use crate::{
    analyzer::types::bounds::Combinator,
    lex::token::{Operator, TokenKind, punc::OpSide},
    scanner::token::value::Keyword,
    src_code::span::Span,
};

// TBD: can RelSpan be used instead of Span? will an AST ever contain spans from multiple separate files?

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ExprPrecedence {
    #[default]
    Statement,
    Assignment,
    Or,
    Xor,
    And,
    Equality,
    Comparison,
    Shift,
    Term,
    Factor,
    UnaryPre,
    Exponent,
    Conversion,
    UnaryPost,
    Primary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingPattern {
    // TBD: what else can a pattern be?
    pub ident: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub lhs: Expr,
    pub op: Operator,
    pub op_span: Span,
    pub rhs: Expr,
}

impl Binary {
    pub const fn precedence(&self) -> ExprPrecedence {
        if matches!(self.op, Operator::Sub) {
            ExprPrecedence::Term
        } else {
            self.op.precedence()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary {
    pub op: Operator,
    pub op_span: Span,
    pub operand: Expr,
    pub side: OpSide,
}

impl Unary {
    pub const fn precedence(&self) -> ExprPrecedence {
        if matches!(self.op, Operator::Neg) {
            ExprPrecedence::UnaryPre
        } else {
            self.op.precedence()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grouping {
    /// `(`
    pub open: Span,
    pub expr: Expr,
    /// `)`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    /// `[`
    pub open: Span,
    pub expr: Expr,
    /// `]`
    pub close: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum OrTypeKeyword {
    #[default]
    None,
    Nevr,
    Fail,
}

impl OrTypeKeyword {
    pub const fn new(value: Keyword) -> Option<Self> {
        Some(match value {
            Keyword::None => Self::None,
            Keyword::Nevr => Self::Nevr,
            Keyword::Fail => Self::Fail,
            _ => return None,
        })
    }

    pub const fn to_keyword(self) -> Keyword {
        match self {
            Self::None => Keyword::None,
            Self::Nevr => Keyword::Nevr,
            Self::Fail => Keyword::Fail,
        }
    }

    pub const fn as_str(self) -> &'static str {
        self.to_keyword().as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrType {
    // `|`
    pub pipe: Span,
    // `none`, `fail`, or `nevr`
    pub ty: OrTypeKeyword,
    pub ty_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TypenameKeyword {
    #[default]
    None,
    Nevr,
    Fail,
    Bool,
    Uint,
    Sint,
    Flpt,
    Char,
    Text,
}

/// TODO: link this to [`Keyword::is_type()`]
impl TypenameKeyword {
    pub const fn new(value: Keyword) -> Option<Self> {
        Some(match value {
            Keyword::None => Self::None,
            Keyword::Nevr => Self::Nevr,
            Keyword::Fail => Self::Fail,
            Keyword::Bool => Self::Bool,
            Keyword::Uint => Self::Uint,
            Keyword::Sint => Self::Sint,
            Keyword::Flpt => Self::Flpt,
            Keyword::Char => Self::Char,
            Keyword::Text => Self::Text,
            _ => return None,
        })
    }

    pub const fn to_keyword(self) -> Keyword {
        match self {
            Self::None => Keyword::None,
            Self::Nevr => Keyword::Nevr,
            Self::Fail => Keyword::Fail,
            Self::Bool => Keyword::Bool,
            Self::Uint => Keyword::Uint,
            Self::Sint => Keyword::Sint,
            Self::Flpt => Keyword::Flpt,
            Self::Char => Keyword::Char,
            Self::Text => Keyword::Text,
        }
    }

    pub const fn as_str(self) -> &'static str {
        self.to_keyword().as_str()
    }
}

impl std::fmt::Display for TypenameKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespacePathPart {
    /// Identifier
    pub name: Span,
    /// `::`
    pub sep: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericGroup {
    /// `<`
    pub open: Span,
    /// (type, `,`)
    /// only the last comma will ever be [`None`]
    pub list: Vec<(TypeExpr, Option<Span>)>,
    /// `>`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IffClause {
    /// `<=>`
    pub iff: Span,
    pub bound: TypeBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typename {
    pub path: Vec<NamespacePathPart>,
    pub name: Option<TypenameKeyword>,
    pub name_span: Span,
    pub args: Option<GenericGroup>,
}

/// A concrete type
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    /// Identifier
    pub name: Typename,
    pub or_ty: Option<OrType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComboBound {
    pub lhs: TypeBound,
    pub sep: Combinator,
    pub sep_span: Span,
    pub rhs: TypeBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegativeBound {
    /// `!`
    pub not: Span,
    pub rhs: TypeBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactTypeBound {
    /// `only`
    pub kw: Span,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColonBound {
    /// `:`
    pub colon: Span,
    pub bound: TypeBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberBound {
    /// `has`
    pub kw: Span,
    /// Identifier
    pub member: Span,
    pub req: Option<ColonBound>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeBound {
    /// `fun`
    pub kw: Span,
    /// `(`
    pub open: Span,
    /// (bound, `,`)
    /// only the last comma will ever be [`None`]
    pub args: Vec<(TypeExpr, Option<Span>)>,
    /// `)`
    pub close: Span,
    // TBD: should return be optional when none?
    /// `->`
    pub arrow: Span,
    pub ret: TypeExpr,
}

/// UNDER CONSTRUCTION
#[derive(Debug, Clone, PartialEq)]
pub struct OverloadBound {
    // TODO
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprBound {
    /// `for`
    pub kw: Span,
    /// Identifier
    pub name: Span,
    /// `:`
    pub colon: Span,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedBound {
    /// `(`
    pub open: Span,
    pub inner: TypeBound,
    /// `)`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBound {
    Combo(Box<ComboBound>),
    Not(Box<NegativeBound>),
    Exact(Box<ExactTypeBound>),
    Has(Box<MemberBound>),
    Shape(Box<ShapeBound>),
    Supports(Box<OverloadBound>),
    Satisfies(Box<ExprBound>),
    Group(Box<GroupedBound>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectiveKw {
    Inpt,
}

impl DirectiveKw {
    pub const fn new(value: Keyword) -> Option<Self> {
        match value {
            Keyword::Inpt => Some(Self::Inpt),
            _ => None,
        }
    }

    pub const fn to_keyword(self) -> Keyword {
        match self {
            Self::Inpt => Keyword::Inpt,
        }
    }

    pub const fn as_str(self) -> &'static str {
        self.to_keyword().as_str()
    }
}

impl std::fmt::Display for DirectiveKw {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub kw: DirectiveKw,
    pub kw_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// Identifier
    pub ident: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockExpr {
    If(Box<IfStmt>),
    /// Only `loop` or `cord`
    Loop(Box<LoopBlock>),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Block(BlockExpr),
    Binary(Box<Binary>),
    Unary(Box<Unary>),
    Literal(Literal),
    Directive(Directive),
    Variable(Variable),
    Grouping(Box<Grouping>),
    Type(Box<TypeExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrBody {
    More(IfStmt),
    Last(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrStmt {
    /// `or`
    pub or_kw: Span,
    pub body: OrBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    /// `if`
    pub if_kw: Span,
    pub pred: Expr,
    pub then: BlockStmt,
    pub or: Option<Box<OrStmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeferParam {
    /// `(`
    pub open: Span,
    pub param: BindingPattern,
    /// `)`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclDefer {
    /// `defer`
    pub kw: Span,
    pub param: Option<DeferParam>,
    pub on_pop: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoopKind {
    /// `kw` is `loop`
    Loop,

    /// `kw` is `cord`
    Cord,

    /// `kw` is `rep`
    Rep {
        /// The condition on which the loop will continue
        pred: Expr,
    },

    /// `kw` is `for`
    For {
        /// The variable `iter` items are stored in
        bind: BindingPattern,
        /// `in`
        in_kw: Span,
        /// An expression which produces an iterable value
        /// (i.e. a value which overloads `++`)
        iter: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopBlock {
    pub label: Option<ScopeLabel>,
    /// variant-dependent
    pub kw: Span,
    pub kind: LoopKind,
    /// the code that gets repeated
    pub block: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoneStmt {
    /// `done`
    pub kw: Span,
    pub label: Option<Label>,
    pub value: Option<Expr>,
    /// `;`
    pub semi: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt {
    /// `skip`
    pub kw: Span,
    pub label: Option<Label>,
    /// `;`
    pub semi: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    /// `{`
    pub open: Span,
    pub body: Vec<Declaration>,
    /// `}`
    pub close: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
    /// `;`
    pub semi: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt {
    /// `prnt`
    pub kw: Span,
    pub group: Grouping,
    /// `;`
    pub semi: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expr(ExprStmt),
    // TODO: im not too fond of the print keyword
    Print(PrintStmt),
    If(IfStmt),
    Loop(LoopBlock),
    Done(DoneStmt),
    Skip(SkipStmt),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub decls: Vec<Declaration>,
}

impl std::ops::Deref for Program {
    type Target = Vec<Declaration>;

    fn deref(&self) -> &Self::Target {
        &self.decls
    }
}

impl std::ops::DerefMut for Program {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.decls
    }
}

impl IntoIterator for Program {
    type Item = Declaration;
    type IntoIter = std::vec::IntoIter<Declaration>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.into_iter()
    }
}

impl<'prgm> IntoIterator for &'prgm Program {
    type Item = &'prgm Declaration;
    type IntoIter = std::slice::Iter<'prgm, Declaration>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.iter()
    }
}

impl<'prgm> IntoIterator for &'prgm mut Program {
    type Item = &'prgm mut Declaration;
    type IntoIter = std::slice::IterMut<'prgm, Declaration>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.iter_mut()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    /// `=`
    pub op: Span,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// `.`
    pub dot: Span,
    /// identifier
    pub name: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScopeLabel {
    pub label: Label,
    /// `:`
    pub colon: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    /// `:`
    pub colon: Span,
    // TODO: should type annotations be different from type expressions?
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum VarKeyword {
    #[default]
    Let,
    Uni,
    Pvt,
}

impl VarKeyword {
    pub const fn new(value: Keyword) -> Option<Self> {
        Some(match value {
            Keyword::Let => Self::Let,
            Keyword::Uni => Self::Uni,
            Keyword::Pvt => Self::Pvt,
            _ => return None,
        })
    }

    pub const fn to_keyword(self) -> Keyword {
        match self {
            Self::Let => Keyword::Let,
            Self::Uni => Keyword::Uni,
            Self::Pvt => Keyword::Pvt,
        }
    }

    pub const fn as_str(self) -> &'static str {
        self.to_keyword().as_str()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclVar {
    /// `uni`, `let`, or `pvt`
    pub kw: VarKeyword,
    pub kw_span: Span,
    /// Identifier
    pub binding: BindingPattern,
    pub ty: Option<TypeAnnotation>,
    pub assignment: Option<Assignment>,
    /// `;`
    pub semi: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Var(DeclVar),
    Defer(DeclDefer),
    Stmt(Statement),
}
