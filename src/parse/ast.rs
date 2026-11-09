use crate::{
    analyzer::types::bounds::Combinator,
    error::ExpectedToken,
    lex::token::{Operator, TokenKind, punc::OpSide},
    scanner::token::value::Keyword,
    src_code::span::{FileId, RelSpan, uspan},
};

pub trait Expectable {
    const EXPECTED: ExpectedToken;
}

pub trait AstNode {
    /// If [`Self::span_end`] is [`Some`], this should be too.
    fn span_start(&self) -> uspan;

    /// If [`Self::span_start`] is [`Some`], this should be too.
    fn span_end(&self) -> uspan;

    fn span(&self) -> RelSpan {
        let start = self.span_start();
        let end = self.span_end();
        debug_assert!(
            start <= end,
            "span should be ordered\
                \n start: {start}\
                \n   end: {end}"
        );
        RelSpan(std::range::Range { start, end })
    }
}

impl AstNode for RelSpan {
    fn span_start(&self) -> uspan {
        self.start
    }

    fn span_end(&self) -> uspan {
        self.end
    }

    fn span(&self) -> RelSpan {
        *self
    }
}

macro_rules! AstNode {
    derive() {
        $(#[$m:meta])*
        $vis:vis enum $Enum:ident {
            $(
                $(#[$vm:meta])*
                $Variant:ident(
                    $(#[$fm:meta])*
                    $Type:ty
                )
            ),* $(,)?
        }
    } => {
        impl AstNode for $Enum {
            fn span_start(&self) -> uspan {
                match self {
                    $(Self::$Variant(x) => x.span_start()),*
                }
            }

            fn span_end(&self) -> uspan {
                match self {
                    $(Self::$Variant(x) => x.span_end()),*
                }
            }

            fn span(&self) -> RelSpan {
                match self {
                    $(Self::$Variant(x) => x.span()),*
                }
            }
        }
    };
}

pub trait ExpectableExt: Sized {
    type Item;

    fn ok_or_expected(self) -> Result<Self::Item, ExpectedToken>;
}

impl<T: Expectable> ExpectableExt for Option<T> {
    type Item = T;

    fn ok_or_expected(self) -> Result<Self::Item, ExpectedToken> {
        self.ok_or(T::EXPECTED)
    }
}

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
    pub ident: RelSpan,
}

impl AstNode for BindingPattern {
    fn span_start(&self) -> uspan {
        self.ident.start
    }

    fn span_end(&self) -> uspan {
        self.ident.end
    }

    fn span(&self) -> RelSpan {
        self.ident
    }
}

impl Expectable for BindingPattern {
    const EXPECTED: ExpectedToken = ExpectedToken::Pattern;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub lhs: Expr,
    pub op: Operator,
    pub op_span: RelSpan,
    pub rhs: Expr,
}

impl AstNode for Binary {
    fn span_start(&self) -> uspan {
        self.lhs.span_start()
    }

    fn span_end(&self) -> uspan {
        self.rhs.span_end()
    }
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
    pub op_span: RelSpan,
    pub operand: Expr,
    pub side: OpSide,
}

impl AstNode for Unary {
    fn span_start(&self) -> uspan {
        match self.side {
            OpSide::Postfix => self.operand.span_start(),
            OpSide::Prefix => self.op_span.start,
        }
    }

    fn span_end(&self) -> uspan {
        match self.side {
            OpSide::Postfix => self.op_span.end,
            OpSide::Prefix => self.operand.span_end(),
        }
    }
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
    pub open: RelSpan,
    pub expr: Expr,
    /// `)`
    pub close: RelSpan,
}

impl AstNode for Grouping {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.open.end
    }
}

impl Expectable for Grouping {
    const EXPECTED: ExpectedToken = ExpectedToken::ParenExpr;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    /// `[`
    pub open: RelSpan,
    pub expr: Expr,
    /// `]`
    pub close: RelSpan,
}

impl AstNode for Sequence {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.open.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum OrTypeKeyword {
    #[default]
    None,
    Nevr,
    Fail,
}

impl Expectable for OrTypeKeyword {
    const EXPECTED: ExpectedToken = ExpectedToken::OrType;
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
impl TryFrom<Keyword> for OrTypeKeyword {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl TryFrom<TokenKind> for OrTypeKeyword {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        if let TokenKind::Keyword(value) = value {
            Self::try_from(value)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrType {
    // `|`
    pub pipe: RelSpan,
    // `none`, `fail`, or `nevr`
    pub ty: OrTypeKeyword,
    pub ty_span: RelSpan,
}

impl AstNode for OrType {
    fn span_start(&self) -> uspan {
        self.pipe.start
    }

    fn span_end(&self) -> uspan {
        self.ty_span.end
    }
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

impl TryFrom<Keyword> for TypenameKeyword {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl TryFrom<TokenKind> for TypenameKeyword {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        if let TokenKind::Keyword(value) = value {
            Self::try_from(value)
        } else {
            Err(())
        }
    }
}

impl TryFrom<Keyword> for Option<TypenameKeyword> {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        TypenameKeyword::new(value).map(Some).ok_or(())
    }
}

impl TryFrom<TokenKind> for Option<TypenameKeyword> {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        match value {
            TokenKind::Ident => Ok(None),
            TokenKind::Keyword(value) => Self::try_from(value),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespacePathPart {
    /// Identifier
    pub name: RelSpan,
    /// `::`
    pub sep: RelSpan,
}

impl AstNode for NamespacePathPart {
    fn span_start(&self) -> uspan {
        self.name.start
    }

    fn span_end(&self) -> uspan {
        self.sep.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericGroup {
    /// `<`
    pub open: RelSpan,
    pub list: Vec<TypeExpr>,
    /// `>`
    pub close: RelSpan,
}

impl AstNode for GenericGroup {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IffClause {
    /// `<=>`
    pub iff: RelSpan,
    pub bound: TypeBound,
}

impl AstNode for IffClause {
    fn span_start(&self) -> uspan {
        self.iff.start
    }

    fn span_end(&self) -> uspan {
        self.bound.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typename {
    pub path: Vec<NamespacePathPart>,
    /// [`None`] represents `Ident`
    pub name: Option<TypenameKeyword>,
    pub name_span: RelSpan,
}

impl AstNode for Typename {
    fn span_start(&self) -> uspan {
        self.path
            .first()
            .map_or(self.name_span.start, NamespacePathPart::span_start)
    }

    fn span_end(&self) -> uspan {
        self.name_span.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInstance {
    pub name: Typename,
    pub args: Option<GenericGroup>,
}

impl AstNode for TypeInstance {
    fn span_start(&self) -> uspan {
        self.name.span_start()
    }

    fn span_end(&self) -> uspan {
        if let Some(args) = &self.args {
            args.span_end()
        } else {
            self.name.span_end()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    /// Identifier
    pub then_ty: TypeInstance,
    pub or_ty: Option<OrType>,
}

impl AstNode for TypeExpr {
    fn span_start(&self) -> uspan {
        self.then_ty.span_start()
    }

    fn span_end(&self) -> uspan {
        if let Some(t) = &self.or_ty {
            t.span_end()
        } else {
            self.then_ty.span_end()
        }
    }
}

impl Expectable for TypeExpr {
    const EXPECTED: ExpectedToken = ExpectedToken::TypeExpr;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComboBound {
    pub lhs: TypeBound,
    pub sep: Combinator,
    pub sep_span: RelSpan,
    pub rhs: TypeBound,
}

impl AstNode for ComboBound {
    fn span_start(&self) -> uspan {
        self.lhs.span_start()
    }

    fn span_end(&self) -> uspan {
        self.rhs.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegativeBound {
    /// `!`
    pub not: RelSpan,
    pub rhs: TypeBound,
}

impl AstNode for NegativeBound {
    fn span_start(&self) -> uspan {
        self.not.start
    }

    fn span_end(&self) -> uspan {
        self.rhs.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExactTypeBound {
    /// `only`
    pub kw: RelSpan,
    pub ty: TypeExpr,
}

impl AstNode for ExactTypeBound {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.ty.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColonBound {
    /// `:`
    pub colon: RelSpan,
    pub bound: TypeBound,
}

impl AstNode for ColonBound {
    fn span_start(&self) -> uspan {
        self.colon.start
    }

    fn span_end(&self) -> uspan {
        self.bound.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberBound {
    /// `has`
    pub kw: RelSpan,
    /// Identifier
    pub member: RelSpan,
    pub req: Option<ColonBound>,
}

impl AstNode for MemberBound {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        if let Some(req) = &self.req {
            req.span_end()
        } else {
            self.member.end
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeBound {
    /// `fun`
    pub kw: RelSpan,
    /// `(`
    pub open: RelSpan,
    pub args: Vec<TypeExpr>,
    /// `)`
    pub close: RelSpan,
    // TBD: should return be optional when none?
    /// `->`
    pub arrow: RelSpan,
    pub ret: TypeExpr,
}

impl AstNode for ShapeBound {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.ret.span_end()
    }
}

/// UNDER CONSTRUCTION
#[derive(Debug, Clone, PartialEq)]
pub struct OverloadBound {
    // TODO
}

impl AstNode for OverloadBound {
    fn span_start(&self) -> uspan {
        todo!()
    }

    fn span_end(&self) -> uspan {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprBound {
    /// `for`
    pub kw: RelSpan,
    /// Identifier
    pub name: RelSpan,
    /// `:`
    pub colon: RelSpan,
    pub expr: Expr,
}

impl AstNode for ExprBound {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.expr.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedBound {
    /// `(`
    pub open: RelSpan,
    pub inner: TypeBound,
    /// `)`
    pub close: RelSpan,
}

impl AstNode for GroupedBound {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

#[derive(Debug, Clone, PartialEq, AstNode)]
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

impl Expectable for TypeBound {
    const EXPECTED: ExpectedToken = ExpectedToken::TypeBound;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub kind: TokenKind,
    pub span: RelSpan,
}

impl AstNode for Literal {
    fn span_start(&self) -> uspan {
        self.span.start
    }

    fn span_end(&self) -> uspan {
        self.span.end
    }

    fn span(&self) -> RelSpan {
        self.span
    }
}

impl Expectable for Literal {
    const EXPECTED: ExpectedToken = ExpectedToken::Literal;
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

impl TryFrom<Keyword> for DirectiveKw {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl TryFrom<TokenKind> for DirectiveKw {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        if let TokenKind::Keyword(value) = value {
            Self::try_from(value)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub kw: DirectiveKw,
    pub kw_span: RelSpan,
}

impl AstNode for Directive {
    fn span_start(&self) -> uspan {
        self.kw_span.start
    }

    fn span_end(&self) -> uspan {
        self.kw_span.end
    }

    fn span(&self) -> RelSpan {
        self.kw_span
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// Identifier
    pub ident: RelSpan,
}

impl AstNode for Variable {
    fn span_start(&self) -> uspan {
        self.ident.start
    }

    fn span_end(&self) -> uspan {
        self.ident.end
    }

    fn span(&self) -> RelSpan {
        self.ident
    }
}

#[derive(Debug, Clone, PartialEq, AstNode)]
pub enum BlockExpr {
    If(Box<IfStmt>),
    /// Only `loop` or `cord`
    Loop(Box<LoopBlock>),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq, AstNode)]
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

impl Expectable for Expr {
    const EXPECTED: ExpectedToken = ExpectedToken::Expr;
}

#[derive(Debug, Clone, PartialEq, AstNode)]
pub enum OrBody {
    More(IfStmt),
    Last(BlockStmt),
}

impl Expectable for OrBody {
    const EXPECTED: ExpectedToken = ExpectedToken::BlockOrIf;
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrStmt {
    /// `or`
    pub kw: RelSpan,
    pub body: OrBody,
}

impl AstNode for OrStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.body.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    /// `if`
    pub kw: RelSpan,
    pub pred: Expr,
    pub then: BlockStmt,
    pub or: Option<Box<OrStmt>>,
}

impl AstNode for IfStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        if let Some(or) = &self.or {
            or.span_end()
        } else {
            self.then.span_end()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeferParam {
    /// `(`
    pub open: RelSpan,
    pub param: BindingPattern,
    /// `)`
    pub close: RelSpan,
}

impl AstNode for DeferParam {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclDefer {
    /// `defer`
    pub kw: RelSpan,
    pub param: Option<DeferParam>,
    pub on_pop: BlockStmt,
}

impl AstNode for DeclDefer {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.on_pop.span_end()
    }
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
        in_kw: RelSpan,
        /// An expression which produces an iterable value
        /// (i.e. a value which overloads `++`)
        iter: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopBlock {
    pub label: Option<ScopeLabel>,
    /// variant-dependent
    pub kw: RelSpan,
    pub kind: LoopKind,
    /// the code that gets repeated
    pub block: BlockStmt,
}

impl AstNode for LoopBlock {
    fn span_start(&self) -> uspan {
        if let Some(label) = &self.label {
            label.span_start()
        } else {
            self.kw.start
        }
    }

    fn span_end(&self) -> uspan {
        self.block.span_end()
    }
}

impl Expectable for LoopBlock {
    const EXPECTED: ExpectedToken = ExpectedToken::LoopBlock;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoneStmt {
    /// `done`
    pub kw: RelSpan,
    pub label: Option<Label>,
    pub value: Option<Expr>,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for DoneStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt {
    /// `skip`
    pub kw: RelSpan,
    pub label: Option<Label>,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for SkipStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    /// `{`
    pub open: RelSpan,
    pub body: Vec<Decl>,
    /// `}`
    pub close: RelSpan,
}

impl AstNode for BlockStmt {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl Expectable for BlockStmt {
    const EXPECTED: ExpectedToken = ExpectedToken::BlockStmt;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for ExprStmt {
    fn span_start(&self) -> uspan {
        self.expr.span_start()
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStmt {
    /// `prnt`
    pub kw: RelSpan,
    pub group: Grouping,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for PrintStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

#[derive(Debug, Clone, PartialEq, AstNode)]
pub enum Stmt {
    Expr(ExprStmt),
    // TODO: im not too fond of the print keyword
    Print(PrintStmt),
    If(IfStmt),
    Loop(LoopBlock),
    Done(DoneStmt),
    Skip(SkipStmt),
    Block(BlockStmt),
}

impl Expectable for Stmt {
    const EXPECTED: ExpectedToken = ExpectedToken::Statement;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgramUnit {
    pub file: FileId,
    pub decls: Vec<Decl>,
}

impl std::ops::Deref for ProgramUnit {
    type Target = Vec<Decl>;

    fn deref(&self) -> &Self::Target {
        &self.decls
    }
}

impl std::ops::DerefMut for ProgramUnit {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.decls
    }
}

impl IntoIterator for ProgramUnit {
    type Item = Decl;
    type IntoIter = std::vec::IntoIter<Decl>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.into_iter()
    }
}

impl<'prgm> IntoIterator for &'prgm ProgramUnit {
    type Item = &'prgm Decl;
    type IntoIter = std::slice::Iter<'prgm, Decl>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.iter()
    }
}

impl<'prgm> IntoIterator for &'prgm mut ProgramUnit {
    type Item = &'prgm mut Decl;
    type IntoIter = std::slice::IterMut<'prgm, Decl>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.iter_mut()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    /// `=`
    pub op: RelSpan,
    pub expr: Expr,
}

impl AstNode for Assignment {
    fn span_start(&self) -> uspan {
        self.op.start
    }

    fn span_end(&self) -> uspan {
        self.expr.span_end()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// `.`
    pub dot: RelSpan,
    /// identifier
    pub name: RelSpan,
}

impl AstNode for Label {
    fn span_start(&self) -> uspan {
        self.dot.start
    }

    fn span_end(&self) -> uspan {
        self.name.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScopeLabel {
    pub label: Label,
    /// `:`
    pub colon: RelSpan,
}

impl AstNode for ScopeLabel {
    fn span_start(&self) -> uspan {
        self.label.span_start()
    }

    fn span_end(&self) -> uspan {
        self.colon.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    /// `:`
    pub colon: RelSpan,
    // TODO: should type annotations be different from type expressions?
    pub ty: TypeExpr,
}

impl AstNode for TypeAnnotation {
    fn span_start(&self) -> uspan {
        self.colon.start
    }

    fn span_end(&self) -> uspan {
        self.ty.span_end()
    }
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

impl TryFrom<Keyword> for VarKeyword {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl TryFrom<TokenKind> for VarKeyword {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        if let TokenKind::Keyword(value) = value {
            Self::try_from(value)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclVar {
    /// `uni`, `let`, or `pvt`
    pub kw: VarKeyword,
    pub kw_span: RelSpan,
    /// Identifier
    pub binding: BindingPattern,
    pub ty: Option<TypeAnnotation>,
    pub assignment: Option<Assignment>,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for DeclVar {
    fn span_start(&self) -> uspan {
        self.kw_span.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

#[derive(Debug, Clone, PartialEq, AstNode)]
pub enum Decl {
    Var(DeclVar),
    Defer(DeclDefer),
    Stmt(Stmt),
}

impl Expectable for Decl {
    const EXPECTED: ExpectedToken = ExpectedToken::Declaration;
}
