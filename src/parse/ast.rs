use crate::{
    analyzer::types::bounds::Combinator,
    error::ExpectedToken,
    highlight::syntax::Syntax,
    lex::token::{Operator, TokenKind, keyword::Keyword, punc::OpSide},
    src_code::span::{FileId, RelSpan, uspan},
};

// TODO: exact-lexeme RelSpans could be replaced with uspans

pub trait Expectable {
    const EXPECTED: ExpectedToken;
}

/// MUST implement either [`Self::span`] or BOTH [`Self::span_start`] and [`Self::span_end`]
pub trait AstNode {
    #[inline]
    fn span_start(&self) -> uspan {
        self.span().start
    }

    #[inline]
    fn span_end(&self) -> uspan {
        self.span().end
    }

    #[inline]
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

pub trait VisitSpans<F: FnMut(RelSpan, Syntax)> {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F);
}

impl AstNode for RelSpan {
    fn span(&self) -> RelSpan {
        *self
    }
}

macro_rules! AstNodeEnum {
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
        impl $crate::parse::ast::AstNode for $Enum {
            fn span_start(&self) -> $crate::src_code::span::uspan {
                match self {
                    $(Self::$Variant(x) => x.span_start()),*
                }
            }

            fn span_end(&self) -> $crate::src_code::span::uspan {
                match self {
                    $(Self::$Variant(x) => x.span_end()),*
                }
            }

            fn span(&self) -> $crate::src_code::span::RelSpan {
                match self {
                    $(Self::$Variant(x) => x.span()),*
                }
            }
        }
    };
}

macro_rules! VisitSpansEnum {
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
        impl<F: FnMut(RelSpan, Syntax)> $crate::parse::ast::VisitSpans<F> for $Enum {
            fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
                match self {
                    $(Self::$Variant(x) => x.visit_spans(brack_depth, visit)),*
                }
            }
        }
    };
}
// pub(crate) use VisitSpansEnum;

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
    MemberAccess,
    Primary,
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Binary {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.lhs.visit_spans(brack_depth, visit);
        visit(
            self.op_span,
            match self.op {
                Operator::Coalesce => Syntax::FlowKeyword,

                Operator::Exists
                | Operator::Convert
                | Operator::Transmute
                | Operator::Incr
                | Operator::Decr
                | Operator::Szof => Syntax::Keyword,

                _ => Syntax::Normal,
            },
        );
        self.rhs.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Unary {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        let op_syn = match self.op {
            Operator::Coalesce => Syntax::FlowKeyword,

            Operator::Exists
            | Operator::Convert
            | Operator::Transmute
            | Operator::Incr
            | Operator::Decr
            | Operator::Szof => Syntax::Keyword,

            _ => Syntax::Normal,
        };
        match self.side {
            OpSide::Postfix => {
                self.operand.visit_spans(brack_depth, visit);
                visit(self.op_span, op_syn);
            }
            OpSide::Prefix => {
                visit(self.op_span, op_syn);
                self.operand.visit_spans(brack_depth, visit);
            }
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
pub struct Subscript {
    pub arr: Expr,
    /// `[`
    pub open: RelSpan,
    pub index: Expr,
    /// `]`
    pub close: RelSpan,
}

impl AstNode for Subscript {
    fn span_start(&self) -> uspan {
        self.arr.span_start()
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Subscript {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.arr.visit_spans(brack_depth, visit);
        visit(self.open, Syntax::Bracket(brack_depth));
        self.index.visit_spans(brack_depth.wrapping_add(1), visit);
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

impl Subscript {
    pub const fn precedence() -> ExprPrecedence {
        ExprPrecedence::UnaryPost
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnCall {
    pub fun: Expr,
    /// `(`
    pub open: RelSpan,
    /// Comma-separated, but the commas arent kept
    pub args: Vec<Expr>,
    /// `)`
    pub close: RelSpan,
}

impl AstNode for FnCall {
    fn span_start(&self) -> uspan {
        self.fun.span_start()
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FnCall {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.fun.visit_spans(brack_depth, visit);
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for arg in &self.args {
            arg.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

impl FnCall {
    pub const fn precedence() -> ExprPrecedence {
        ExprPrecedence::UnaryPost
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Grouping {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        self.expr.visit_spans(brack_depth.wrapping_add(1), visit);
        visit(self.close, Syntax::Bracket(brack_depth));
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Sequence {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        self.expr.visit_spans(brack_depth.wrapping_add(1), visit);
        visit(self.close, Syntax::Bracket(brack_depth));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TypenameKeyword {
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

/// TODO: link this to [`Keyword::is_type()`]
impl TypenameKeyword {
    pub const fn new(value: Keyword) -> Option<Self> {
        Some(match value {
            Keyword::None => Self::None,
            Keyword::Nevr => Self::Nevr,
            Keyword::Fail => Self::Fail,
            Keyword::Bool => Self::Bool,
            Keyword::Byte => Self::Byte,
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
            Self::Byte => Keyword::Byte,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for NamespacePathPart {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.name, Syntax::Modname);
        visit(self.sep, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParams {
    /// `<`
    pub open: RelSpan,
    pub list: Vec<RelSpan>,
    /// `>`
    pub close: RelSpan,
}

impl AstNode for GenericParams {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for GenericParams {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        for &item in &self.list {
            visit(item, Syntax::Typename);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericArgs {
    /// `<`
    pub open: RelSpan,
    pub list: Vec<TypeExpr>,
    /// `>`
    pub close: RelSpan,
}

impl AstNode for GenericArgs {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for GenericArgs {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for item in &self.list {
            item.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeBoundClause {
    pub ty: TypeExpr,
    /// `:`
    pub colon: RelSpan,
    pub bound: TypeBound,
}

impl AstNode for TypeBoundClause {
    fn span_start(&self) -> uspan {
        self.ty.span_start()
    }

    fn span_end(&self) -> uspan {
        self.bound.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for TypeBoundClause {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.ty.visit_spans(brack_depth, visit);
        visit(self.colon, Syntax::Normal);
        self.bound.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IffClause {
    /// `<=>`
    pub iff: RelSpan,
    /// Comma separated (comma is not stored)
    pub bounds: Vec<TypeBoundClause>,
}

impl AstNode for IffClause {
    fn span_start(&self) -> uspan {
        self.iff.start
    }

    fn span_end(&self) -> uspan {
        self.bounds
            .last()
            .map_or(self.iff.end, TypeBoundClause::span_end)
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for IffClause {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.iff, Syntax::Keyword);
        for bound in &self.bounds {
            bound.visit_spans(brack_depth, visit);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimitiveType {
    pub ty: TypenameKeyword,
    pub span: RelSpan,
}

impl AstNode for PrimitiveType {
    fn span(&self) -> RelSpan {
        self.span
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for PrimitiveType {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.span, Syntax::Primitive);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typename {
    pub path: Vec<NamespacePathPart>,
    /// Identifier
    pub name: RelSpan,
}

impl AstNode for Typename {
    fn span_start(&self) -> uspan {
        self.path
            .first()
            .map_or(self.name.start, NamespacePathPart::span_start)
    }

    fn span_end(&self) -> uspan {
        self.name.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Typename {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        for part in &self.path {
            part.visit_spans(brack_depth, visit);
        }
        visit(self.name, Syntax::Typename);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInstance {
    pub name: Typename,
    pub args: Option<GenericArgs>,
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for TypeInstance {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.name.visit_spans(brack_depth, visit);
        if let Some(args) = &self.args {
            args.visit_spans(brack_depth, visit);
        }
    }
}

/// Fallible Type
#[derive(Debug, Clone, PartialEq)]
pub struct FallType {
    pub then_ty: Box<TypeExpr>,
    // `|`
    pub pipe: RelSpan,
    // `none`, `fail`, or `nevr`
    pub ty: OrTypeKeyword,
    pub ty_span: RelSpan,
}

impl AstNode for FallType {
    fn span_start(&self) -> uspan {
        self.then_ty.span_start()
    }

    fn span_end(&self) -> uspan {
        self.ty_span.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FallType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.then_ty.visit_spans(brack_depth, visit);
        visit(self.pipe, Syntax::Keyword); // TBD: what syntax should this be?
        visit(self.ty_span, Syntax::Primitive);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefType {
    /// `@`
    pub sym: RelSpan,
    // TBD: reference mutability?
    pub ty: Box<TypeExpr>,
}

impl AstNode for RefType {
    fn span_start(&self) -> uspan {
        self.sym.span_start()
    }

    fn span_end(&self) -> uspan {
        self.ty.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for RefType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.sym, Syntax::Keyword); // TBD: what syntax should this be?
        self.ty.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrCount {
    /// `;`
    pub semi: RelSpan,
    pub count: Expr,
}

impl AstNode for ArrCount {
    fn span_start(&self) -> uspan {
        self.semi.start
    }

    fn span_end(&self) -> uspan {
        self.count.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ArrCount {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.semi, Syntax::Normal);
        self.count.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewType {
    /// `[`
    pub open: RelSpan,
    pub item: Box<TypeExpr>,
    pub count: Option<ArrCount>,
    /// `]`
    pub close: RelSpan,
}

impl AstNode for ViewType {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ViewType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        self.item.visit_spans(inner_depth, visit);
        if let Some(count) = &self.count {
            count.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedType {
    /// `<`
    pub open: RelSpan,
    pub inner: TypeExpr,
    /// `>`
    pub close: RelSpan,
}

impl AstNode for GroupedType {
    fn span_start(&self) -> uspan {
        self.open.span_start()
    }

    fn span_end(&self) -> uspan {
        self.close.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for GroupedType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        self.inner.visit_spans(brack_depth.wrapping_add(1), visit);
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum AssocTypeSource {
    Simple(Typename),
    Grouped(GroupedType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssocType {
    pub base: AssocTypeSource,
    /// `::`
    pub sep: RelSpan,
    pub assoc: TypeInstance,
}

impl AstNode for AssocType {
    fn span_start(&self) -> uspan {
        self.base.span_start()
    }

    fn span_end(&self) -> uspan {
        self.assoc.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for AssocType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.base.visit_spans(brack_depth, visit);
        visit(self.sep, Syntax::Normal);
        self.assoc.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunRet {
    /// `->`
    pub arrow: RelSpan,
    pub ty: TypeExpr,
}

impl AstNode for FunRet {
    fn span_start(&self) -> uspan {
        self.arrow.start
    }

    fn span_end(&self) -> uspan {
        self.ty.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FunRet {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.arrow, Syntax::Normal);
        self.ty.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunType {
    /// `fun`
    pub kw: RelSpan,
    /// `(`
    pub open: RelSpan,
    pub params: Vec<TypeExpr>,
    /// `)`
    pub close: RelSpan,
    pub ret: Option<Box<FunRet>>,
}

impl AstNode for FunType {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        if let Some(ret) = &self.ret {
            ret.span_end()
        } else {
            self.close.end
        }
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FunType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for param in &self.params {
            param.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
        if let Some(ret) = self.ret.as_deref() {
            ret.visit_spans(brack_depth, visit);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpaqueType {
    /// `:`
    pub colon: RelSpan,
    pub bound: TypeBound,
}

impl AstNode for OpaqueType {
    fn span_start(&self) -> uspan {
        self.colon.start
    }

    fn span_end(&self) -> uspan {
        self.bound.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for OpaqueType {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.colon, Syntax::Keyword);
        self.bound.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum TypeExpr {
    Primitive(PrimitiveType),
    Instance(TypeInstance),
    Fallible(FallType),
    Ref(RefType),
    View(ViewType),
    Assoc(Box<AssocType>),
    FunPtr(FunType),
    Opaque(OpaqueType),
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ComboBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.lhs.visit_spans(brack_depth, visit);
        visit(self.sep_span, Syntax::Normal);
        self.rhs.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for NegativeBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.not, Syntax::Normal);
        self.rhs.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ExactTypeBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        self.ty.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ColonBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.colon, Syntax::Normal);
        self.bound.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for MemberBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.member, Syntax::Param);
        if let Some(req) = &self.req {
            req.visit_spans(brack_depth, visit);
        }
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for OverloadBound {
    fn visit_spans(&self, _brack_depth: u8, _visit: &mut F) {
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ExprBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.name, Syntax::Param);
        visit(self.colon, Syntax::Keyword);
        self.expr.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for GroupedBound {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        self.inner.visit_spans(brack_depth.wrapping_add(1), visit);
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum TypeBound {
    Combo(Box<ComboBound>),
    Not(Box<NegativeBound>),
    Exact(Box<ExactTypeBound>),
    Has(Box<MemberBound>),
    Shape(Box<FunType>),
    Supports(Box<OverloadBound>),
    Satisfies(Box<ExprBound>),
    Group(Box<GroupedBound>),
}

impl Expectable for TypeBound {
    const EXPECTED: ExpectedToken = ExpectedToken::TypeBound;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum LiteralKind {
    #[default]
    None,
    Bool(bool),
    Number,
    Char,
    Text,
    Macro,
}

impl LiteralKind {
    pub const fn new(value: TokenKind) -> Option<Self> {
        Some(match value {
            TokenKind::Keyword(Keyword::None) => Self::None,
            TokenKind::Keyword(Keyword::True) => Self::Bool(true),
            TokenKind::Keyword(Keyword::Fals) => Self::Bool(false),
            TokenKind::NumberLit => Self::Number,
            TokenKind::CharLit => Self::Char,
            TokenKind::TextLit => Self::Text,
            TokenKind::Macro => Self::Macro,
            _ => return None,
        })
    }

    pub const fn to_token_kind(self) -> TokenKind {
        match self {
            Self::None => TokenKind::Keyword(Keyword::None),
            Self::Bool(b) => TokenKind::Keyword(if b { Keyword::True } else { Keyword::Fals }),
            Self::Number => TokenKind::NumberLit,
            Self::Char => TokenKind::CharLit,
            Self::Text => TokenKind::TextLit,
            Self::Macro => TokenKind::Macro,
        }
    }
}

impl TryFrom<TokenKind> for LiteralKind {
    type Error = ();

    fn try_from(value: TokenKind) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

impl From<LiteralKind> for TokenKind {
    fn from(value: LiteralKind) -> Self {
        value.to_token_kind()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Literal {
    pub kind: LiteralKind,
    pub span: RelSpan,
}

impl AstNode for Literal {
    fn span(&self) -> RelSpan {
        self.span
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Literal {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(
            self.span,
            match self.kind {
                LiteralKind::None | LiteralKind::Bool(_) => Syntax::LanguageDefined,
                LiteralKind::Number => Syntax::NumberLiteral,
                LiteralKind::Char => Syntax::CharLiteral,
                LiteralKind::Text => Syntax::TextLiteral,
                LiteralKind::Macro => Syntax::MacroName,
            },
        );
    }
}

impl Expectable for Literal {
    const EXPECTED: ExpectedToken = ExpectedToken::Literal;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectiveKw {
    Inpt,
    Prnt,
}

impl DirectiveKw {
    pub const fn new(value: Keyword) -> Option<Self> {
        match value {
            Keyword::Inpt => Some(Self::Inpt),
            Keyword::Prnt => Some(Self::Prnt),
            _ => None,
        }
    }

    pub const fn to_keyword(self) -> Keyword {
        match self {
            Self::Inpt => Keyword::Inpt,
            Self::Prnt => Keyword::Prnt,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Directive {
    pub kw: DirectiveKw,
    pub kw_span: RelSpan,
}

impl AstNode for Directive {
    fn span(&self) -> RelSpan {
        self.kw_span
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Directive {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.kw_span, Syntax::LanguageDefined);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModPathPart {
    /// Identifier
    pub ident: RelSpan,
    /// `::`
    pub sep: RelSpan,
}

impl AstNode for ModPathPart {
    fn span_start(&self) -> uspan {
        self.ident.start
    }

    fn span_end(&self) -> uspan {
        self.ident.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ModPathPart {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.ident, Syntax::Modname);
        visit(self.sep, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// Identifier
    pub ident: RelSpan,
}

impl AstNode for Member {
    fn span(&self) -> RelSpan {
        self.ident
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Member {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.ident, Syntax::Variable); // TODO: it depends; a member could be a function
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// [`None`] if `me`, otherwise not a `me` variable
    pub path: Option<Vec<ModPathPart>>,
    /// Identifier or `me`
    pub ident: RelSpan,
}

impl Variable {
    pub const fn is_me(&self) -> bool {
        self.path.is_none()
    }
}

impl AstNode for Variable {
    fn span_start(&self) -> uspan {
        self.path
            .as_ref()
            .and_then(|x| x.first())
            .map_or(self.ident.start, ModPathPart::span_start)
    }

    fn span_end(&self) -> uspan {
        self.ident.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Variable {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        if let Some(path) = &self.path {
            for part in path {
                part.visit_spans(brack_depth, visit);
            }
        }
        visit(
            self.ident,
            if self.is_me() {
                Syntax::LanguageVariable
            } else {
                Syntax::Variable
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum BlockExpr {
    If(Box<IfStmt>),
    Pick(Box<PickStmt>),
    /// Only `loop` or `cord`
    Loop(Box<LoopBlock>),
    Block(BlockStmt),
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum Expr {
    Block(BlockExpr),
    Binary(Box<Binary>),
    Unary(Box<Unary>),
    Index(Box<Subscript>),
    Call(Box<FnCall>),
    Literal(Literal),
    Directive(Directive),
    Member(Member),
    Variable(Variable),
    Grouping(Box<Grouping>),
    Type(Box<TypeExpr>),
}

impl Expectable for Expr {
    const EXPECTED: ExpectedToken = ExpectedToken::Expr;
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for OrStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        self.body.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for IfStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        self.pred.visit_spans(brack_depth, visit);
        self.then.visit_spans(brack_depth, visit);
        if let Some(or) = &self.or {
            or.visit_spans(brack_depth, visit);
        }
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum PickPattern {
    Constant(Variable),
    Literal(Literal),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickArm {
    pub pattern: PickPattern,
    /// `=>`
    pub arrow: RelSpan,
    pub then: Expr,
}

impl AstNode for PickArm {
    fn span_start(&self) -> uspan {
        self.pattern.span_start()
    }

    fn span_end(&self) -> uspan {
        self.then.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for PickArm {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.pattern.visit_spans(brack_depth, visit);
        visit(self.arrow, Syntax::Normal);
        self.then.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickStmt {
    /// `pick`
    pub kw: RelSpan,
    pub expr: Expr,
    /// `{`
    pub open: RelSpan,
    pub arms: Vec<PickArm>,
    /// `}`
    pub close: RelSpan,
}

impl AstNode for PickStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.close.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for PickStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        self.expr.visit_spans(brack_depth, visit);
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for arm in &self.arms {
            arm.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DferParam {
    /// `(`
    pub open: RelSpan,
    pub param: BindingPattern,
    /// `)`
    pub close: RelSpan,
}

impl AstNode for DferParam {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DferParam {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        self.param.visit_spans(brack_depth.wrapping_add(1), visit);
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclDfer {
    /// `dfer`
    pub kw: RelSpan,
    pub param: Option<DferParam>,
    pub on_pop: BlockStmt,
}

impl AstNode for DeclDfer {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.on_pop.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DeclDfer {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        if let Some(param) = &self.param {
            param.visit_spans(brack_depth, visit);
        }
        self.on_pop.visit_spans(brack_depth, visit);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for LoopBlock {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        if let Some(label) = &self.label {
            label.visit_spans(brack_depth, visit);
        }
        visit(self.kw, Syntax::FlowKeyword);
        match &self.kind {
            LoopKind::Loop | LoopKind::Cord => {}
            LoopKind::Rep { pred } => {
                pred.visit_spans(brack_depth, visit);
            }
            LoopKind::For { bind, in_kw, iter } => {
                bind.visit_spans(brack_depth, visit);
                visit(*in_kw, Syntax::FlowKeyword);
                iter.visit_spans(brack_depth, visit);
            }
        }
        self.block.visit_spans(brack_depth, visit);
    }
}

impl Expectable for LoopBlock {
    const EXPECTED: ExpectedToken = ExpectedToken::LoopBlock;
}

#[derive(Debug, Clone, PartialEq)]
pub struct FailStmt {
    /// `fail`
    pub kw: RelSpan,
    pub value: Option<Expr>,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for FailStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FailStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        if let Some(value) = &self.value {
            value.visit_spans(brack_depth, visit);
        }
        visit(self.semi, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GiveStmt {
    /// `give`
    pub kw: RelSpan,
    pub value: Option<Expr>,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for GiveStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for GiveStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        if let Some(value) = &self.value {
            value.visit_spans(brack_depth, visit);
        }
        visit(self.semi, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitStmt {
    /// `emit`
    pub kw: RelSpan,
    pub value: Option<Expr>,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for EmitStmt {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for EmitStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        if let Some(value) = &self.value {
            value.visit_spans(brack_depth, visit);
        }
        visit(self.semi, Syntax::Normal);
    }
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DoneStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        if let Some(label) = &self.label {
            label.visit_spans(brack_depth, visit);
        }
        if let Some(value) = &self.value {
            value.visit_spans(brack_depth, visit);
        }
        visit(self.semi, Syntax::Normal);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for SkipStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::FlowKeyword);
        if let Some(label) = &self.label {
            label.visit_spans(brack_depth, visit);
        }
        visit(self.semi, Syntax::Normal);
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for BlockStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for decl in &self.body {
            decl.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ExprStmt {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.expr.visit_spans(brack_depth, visit);
        visit(self.semi, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum Stmt {
    Expr(ExprStmt),
    If(IfStmt),
    Pick(PickStmt),
    Loop(LoopBlock),
    Fail(FailStmt),
    Give(GiveStmt),
    Emit(EmitStmt),
    Done(DoneStmt),
    Skip(SkipStmt),
    Block(BlockStmt),
}

impl Expectable for Stmt {
    const EXPECTED: ExpectedToken = ExpectedToken::Statement;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub units: Vec<ProgramUnit>,
}

impl std::ops::Deref for Program {
    type Target = Vec<ProgramUnit>;

    fn deref(&self) -> &Self::Target {
        &self.units
    }
}

impl std::ops::DerefMut for Program {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.units
    }
}

impl IntoIterator for Program {
    type Item = ProgramUnit;
    type IntoIter = std::vec::IntoIter<ProgramUnit>;

    fn into_iter(self) -> Self::IntoIter {
        self.units.into_iter()
    }
}

impl<'prgm> IntoIterator for &'prgm Program {
    type Item = &'prgm ProgramUnit;
    type IntoIter = std::slice::Iter<'prgm, ProgramUnit>;

    fn into_iter(self) -> Self::IntoIter {
        self.units.iter()
    }
}

impl<'prgm> IntoIterator for &'prgm mut Program {
    type Item = &'prgm mut ProgramUnit;
    type IntoIter = std::slice::IterMut<'prgm, ProgramUnit>;

    fn into_iter(self) -> Self::IntoIter {
        self.units.iter_mut()
    }
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

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum Decl {
    Mod(DeclMod),
    On(DeclOn),
    Var(DeclVar),
    Fun(DeclFun),
    Type(DeclType),
    Dfer(DeclDfer),
    Stmt(Stmt),
}

impl Expectable for Decl {
    const EXPECTED: ExpectedToken = ExpectedToken::Declaration;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeclMod {
    // TBD: visibility?
    /// `mod`
    pub kw: RelSpan,
    /// Identifier
    pub name: RelSpan,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for DeclMod {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DeclMod {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.name, Syntax::Modname);
        visit(self.semi, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclOn {
    /// `on`
    pub kw: RelSpan,
    pub params: Option<GenericParams>,
    pub ty: TypeExpr,
    pub iff: Option<IffClause>,
    pub body: BlockStmt,
}

impl AstNode for DeclOn {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.body.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DeclOn {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        if let Some(params) = &self.params {
            params.visit_spans(brack_depth, visit);
        }
        self.ty.visit_spans(brack_depth, visit);
        if let Some(iff) = &self.iff {
            iff.visit_spans(brack_depth, visit);
        }
        self.body.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclVar {
    /// `uni`, `let`, or `pvt`
    pub kw: VarKeyword,
    pub kw_span: RelSpan,
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DeclVar {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw_span, Syntax::Keyword);
        self.binding.visit_spans(brack_depth, visit); // TODO: highlight depending on keyword
        if let Some(ty) = &self.ty {
            ty.visit_spans(brack_depth, visit);
        }
        if let Some(assignment) = &self.assignment {
            assignment.visit_spans(brack_depth, visit);
        }
        visit(self.semi, Syntax::Normal);
    }
}

/// Same syntax as [`Field`], for now.
/// Guaranteed not to use visibility modifiers in the future, and uses [`Syntax::Param`] instead of [`Syntax::Field`].
#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    /// Ident
    pub name: RelSpan,
    pub ty: TypeAnnotation,
}

impl AstNode for FnParam {
    fn span_start(&self) -> uspan {
        self.name.start
    }

    fn span_end(&self) -> uspan {
        self.ty.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FnParam {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.name, Syntax::Param);
        self.ty.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfParam {
    /// `me`
    pub span: RelSpan,
}

impl AstNode for SelfParam {
    fn span(&self) -> RelSpan {
        self.span
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for SelfParam {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.span, Syntax::LanguageVariable);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnSignature {
    /// `fun`
    pub kw: RelSpan,
    /// Ident
    pub name: RelSpan,
    pub generics: Option<GenericParams>,
    /// `(`
    pub open: RelSpan,
    pub self_param: Option<SelfParam>,
    /// Comma-separated (commas are not stored)
    pub params: Vec<FnParam>,
    /// `)`
    pub close: RelSpan,
    pub ret: Option<FunRet>,
    pub iff: Option<IffClause>,
}

impl AstNode for FnSignature {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        if let Some(iff) = &self.iff {
            iff.span_end()
        } else if let Some(ret) = &self.ret {
            ret.span_end()
        } else {
            self.close.end
        }
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FnSignature {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.name, Syntax::Callable);
        if let Some(generics) = &self.generics {
            generics.visit_spans(brack_depth, visit);
        }
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        if let Some(self_param) = &self.self_param {
            self_param.visit_spans(brack_depth, visit);
        }
        for param in &self.params {
            param.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
        if let Some(ret) = &self.ret {
            ret.visit_spans(brack_depth, visit);
        }
        if let Some(iff) = &self.iff {
            iff.visit_spans(brack_depth, visit);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefFun {
    pub head: FnSignature,
    pub body: BlockStmt,
}

impl AstNode for DefFun {
    fn span_start(&self) -> uspan {
        self.head.span_start()
    }

    fn span_end(&self) -> uspan {
        self.body.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DefFun {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.head.visit_spans(brack_depth, visit);
        self.body.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnounceFun {
    pub head: FnSignature,
    /// `;`
    pub semi: RelSpan,
}

impl AstNode for AnnounceFun {
    fn span_start(&self) -> uspan {
        self.head.span_start()
    }

    fn span_end(&self) -> uspan {
        self.semi.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for AnnounceFun {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.head.visit_spans(brack_depth, visit);
        visit(self.semi, Syntax::Normal);
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum DeclFun {
    Declare(AnnounceFun),
    Define(DefFun),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingPattern {
    // TBD: what else can a pattern be?
    pub ident: RelSpan,
}

impl AstNode for BindingPattern {
    fn span(&self) -> RelSpan {
        self.ident
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for BindingPattern {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.ident, Syntax::Variable);
    }
}

impl Expectable for BindingPattern {
    const EXPECTED: ExpectedToken = ExpectedToken::Pattern;
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for TypeAnnotation {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.colon, Syntax::Normal);
        self.ty.visit_spans(brack_depth, visit);
    }
}

impl Expectable for TypeAnnotation {
    const EXPECTED: ExpectedToken = ExpectedToken::TypeAnnotation;
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Assignment {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.op, Syntax::Normal);
        self.expr.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Label {
    fn visit_spans(&self, _brack_depth: u8, visit: &mut F) {
        visit(self.dot, Syntax::Normal);
        visit(self.name, Syntax::Label);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for ScopeLabel {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        self.label.visit_spans(brack_depth, visit);
        visit(self.colon, Syntax::Normal);
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
pub struct DeclRec {
    // TODO: visibility?
    /// `rec`
    pub kw: RelSpan,
    /// Identifier
    pub name: RelSpan,
    pub generics: Option<GenericParams>,
    pub body: FieldSet,
}

impl AstNode for DeclRec {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.body.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DeclRec {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.name, Syntax::Typename);
        if let Some(generics) = &self.generics {
            generics.visit_spans(brack_depth, visit);
        }
        self.body.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    // TODO: visibility?
    /// Identifier
    pub name: RelSpan,
    pub ty: TypeAnnotation,
}

impl AstNode for Field {
    fn span_start(&self) -> uspan {
        self.name.start
    }

    fn span_end(&self) -> uspan {
        self.ty.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for Field {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.name, Syntax::Field);
        self.ty.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSet {
    /// `{`
    pub open: RelSpan,
    /// Comma-separated, but commas are not stored
    pub fields: Vec<Field>,
    /// `}`
    pub close: RelSpan,
}

impl AstNode for FieldSet {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for FieldSet {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for field in &self.fields {
            field.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatDisc {
    /// Identifier
    pub name: RelSpan,
    pub val: Option<Assignment>,
}

impl AstNode for CatDisc {
    fn span_start(&self) -> uspan {
        self.name.start
    }

    fn span_end(&self) -> uspan {
        if let Some(val) = &self.val {
            val.span_end()
        } else {
            self.name.span_end()
        }
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for CatDisc {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.name, Syntax::Typename);
        if let Some(val) = &self.val {
            val.visit_spans(brack_depth, visit);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatDiscSet {
    /// `{`
    pub open: RelSpan,
    /// Comma-separated, but commas aren't kept
    pub items: Vec<CatDisc>,
    /// `}`
    pub close: RelSpan,
}

impl AstNode for CatDiscSet {
    fn span_start(&self) -> uspan {
        self.open.start
    }

    fn span_end(&self) -> uspan {
        self.close.end
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for CatDiscSet {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.open, Syntax::Bracket(brack_depth));
        let inner_depth = brack_depth.wrapping_add(1);
        for item in &self.items {
            item.visit_spans(inner_depth, visit);
        }
        visit(self.close, Syntax::Bracket(brack_depth));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclCat {
    /// `cat`
    pub kw: RelSpan,
    /// Identifier
    pub name: RelSpan,
    pub discriminants: CatDiscSet,
}

impl AstNode for DeclCat {
    fn span_start(&self) -> uspan {
        self.kw.start
    }

    fn span_end(&self) -> uspan {
        self.discriminants.span_end()
    }
}

impl<F: FnMut(RelSpan, Syntax)> VisitSpans<F> for DeclCat {
    fn visit_spans(&self, brack_depth: u8, visit: &mut F) {
        visit(self.kw, Syntax::Keyword);
        visit(self.name, Syntax::Typename);
        self.discriminants.visit_spans(brack_depth, visit);
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeEnum, VisitSpansEnum)]
pub enum DeclType {
    Rec(DeclRec),
    Cat(DeclCat),
    // TODO: subtypes
    // TODO: supertypes
    // TODO: category types
    // TBD: aliases?
}
