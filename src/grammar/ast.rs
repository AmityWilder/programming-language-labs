use crate::{
    analyzer::types::bounds::Combinator,
    error::OpSide,
    scanner::token::{
        ExpansionData, Lexeme, Lexical, SpecToken, Token,
        punc::Punctuation,
        value::{Keyword, LexValue, Operator, TokenEq},
    },
};
use std::range::Range;

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

/// Allows the AST to combine the full range of its tokens, for use in errors.
///
/// Implementors must define either
/// - [`Self::get_range`]
/// - both [`Self::get_range_start`] and [`Self::get_range_end`]
/// - or all three
///
/// Consider implementing [`AstNodeFinal`] if the conversion cannot fail.
pub trait AstNode<'src> {
    /// For when there is a less expensive implementation for ranges that are one-sided
    ///
    /// Returns [`None`] if the range is empty
    ///
    /// If this method returns [`Some`], [`Self::get_range_end`] must also return [`Some`].
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.get_range(source).map(|range| range.start)
    }

    /// For when there is a less expensive implementation for ranges that are one-sided
    ///
    /// Returns [`None`] if the range is empty
    ///
    /// If this method returns [`Some`], [`Self::get_range_start`] must also return [`Some`].
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.get_range(source).map(|range| range.end)
    }

    /// Returns [`None`] if the range is empty
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.get_range_start(source).map(|start| {
            let end = self
                .get_range_end(source)
                .expect("Some-ness of get_range_end should match that of get_range_start");
            Range { start, end }
        })
    }

    /// Only considered a macro range if the ENTIRE EXPRESSION is from the same macro expansion
    // TBD: what if part of it is from a nested macro?
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>>;
}

pub trait AstNodeFinal<'src>: AstNode<'src> {
    /// # Panics
    /// This method may panic if [`Self::get_range`] returns [`None`]
    fn range(&self, source: &'src str) -> Range<usize> {
        self.get_range(source)
            .expect("final range should not be empty")
    }
}

/// For types that are guaranteed to have a range
macro_rules! final_ast_node {
    derive() {
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident<$lt:lifetime> $def:tt
    } => {
        impl<$lt> $crate::grammar::ast::AstNodeFinal<$lt> for $Name<$lt> {}
    };

    derive() {
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident<$lt:lifetime> $def:tt
    } => {
        impl<$lt> $crate::grammar::ast::AstNodeFinal<$lt> for $Name<$lt> {}
    };
}
pub(crate) use final_ast_node as AstNodeF;

macro_rules! simple_ast_node_enum {
    derive() {
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident<$lt:lifetime> {$(
            $(#[$vmeta:meta])*
            $Variant:ident($Type:ty)
        ),* $(,)?}
    } => {
        impl<$lt> $crate::grammar::ast::AstNode<$lt> for $Name<$lt> {
            fn get_range_start(&self, source: &'src str) -> Option<usize> {
                match self {
                    $(Self::$Variant(inner) => inner.get_range_start(source)),*
                }
            }

            fn get_range_end(&self, source: &'src str) -> Option<usize> {
                match self {
                    $(Self::$Variant(inner) => inner.get_range_end(source)),*
                }
            }

            fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
                match self {
                    $(Self::$Variant(inner) => inner.get_range(source)),*
                }
            }

            fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
                match self {
                    $(Self::$Variant(inner) => inner.expansion(source)),*
                }
            }
        }
    };
}
pub(crate) use simple_ast_node_enum as AstEnum;

/// Passthrough references
impl<'src, T: ?Sized> AstNode<'src> for &T
where
    T: AstNode<'src>,
{
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        (*self).get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (*self).get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (*self).get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (*self).expansion(source)
    }
}

/// Passthrough references
impl<'src, T: ?Sized> AstNode<'src> for Box<T>
where
    T: AstNode<'src>,
{
    #[inline]
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        (**self).get_range_start(source)
    }

    #[inline]
    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (**self).get_range_end(source)
    }

    #[inline]
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (**self).get_range(source)
    }

    #[inline]
    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (**self).expansion(source)
    }
}

/// Between two nodes, takes the start of the first and the end of the second.
/// All nodes between can be omitted, which is why only tuples of 2 are defined.
impl<'src, T, U> AstNode<'src> for (T, U)
where
    T: AstNode<'src>,
    U: AstNode<'src>,
{
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.0
            .get_range_start(source)
            .or_else(|| self.1.get_range_start(source))
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.1
            .get_range_end(source)
            .or_else(|| self.0.get_range_end(source))
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match (self.0.get_range_start(source), self.1.get_range_end(source)) {
            (None, None) => None,
            (Some(start), None) => {
                let end = self
                    .0
                    .get_range_end(source)
                    .expect("Some-ness of get_range_end should match that of get_range_start");
                Some(Range { start, end })
            }
            (None, Some(end)) => {
                let start = self
                    .1
                    .get_range_start(source)
                    .expect("Some-ness of get_range_start should match that of get_range_end");
                Some(Range { start, end })
            }
            (Some(start), Some(end)) => Some(Range { start, end }),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        let exp = (self.0.expansion(source)?, self.1.expansion(source)?);
        (exp.0 == exp.1).then(|| {
            if exp.0.arg == exp.1.arg {
                exp.0
            } else {
                ExpansionData {
                    range: exp.0.range,
                    arg: None,
                }
            }
        })
    }
}

impl<'src, T, U> AstNodeFinal<'src> for (T, U)
where
    T: AstNodeFinal<'src>,
    U: AstNodeFinal<'src>,
{
}

impl<'src, T> AstNode<'src> for Option<T>
where
    T: AstNode<'src>,
{
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.as_ref().and_then(|x| x.get_range_start(source))
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.as_ref().and_then(|x| x.get_range_end(source))
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.as_ref().and_then(|x| x.get_range(source))
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.as_ref().and_then(|x| x.expansion(source))
    }
}

impl<'src, T> AstNode<'src> for [T]
where
    T: AstNode<'src>,
{
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.first().and_then(|x| x.get_range_start(source))
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.last().and_then(|x| x.get_range_end(source))
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            [] => None,
            [only] => only.get_range(source),
            [first, .., last] => (first, last).get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self {
            [] => None,
            [only] => only.expansion(source),
            [first, .., last] => (first, last).expansion(source),
        }
    }
}

impl<'src> AstNode<'src> for str {
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        source.substr_range(self)
    }

    fn expansion(&self, _: &'src str) -> Option<ExpansionData<'src>> {
        None
    }
}

impl<'src> AstNode<'src> for Lexeme<'src> {
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        Some(self.lex_range(source))
    }

    fn expansion(&self, _: &'src str) -> Option<ExpansionData<'src>> {
        self.mac
    }
}
impl<'src> AstNodeFinal<'src> for Lexeme<'src> {}

impl<'src, T> AstNode<'src> for SpecToken<'src, T> {
    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        Some(self.lex_range(source))
    }

    fn expansion(&self, _: &'src str) -> Option<ExpansionData<'src>> {
        self.mac
    }
}
impl<'src, T> AstNodeFinal<'src> for SpecToken<'src, T> {}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct BindingPattern<'src> {
    // TBD: what else can a pattern be?
    pub ident: Lexeme<'src>,
}

impl<'src> AstNode<'src> for BindingPattern<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.ident.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ident.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.ident.get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.ident.expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Binary<'src> {
    pub lhs: Expr<'src>,
    pub op: SpecToken<'src, Operator>,
    pub rhs: Expr<'src>,
}

impl Binary<'_> {
    pub const fn precedence(&self) -> ExprPrecedence {
        if matches!(self.op.val, Operator::Sub) {
            ExprPrecedence::Term
        } else {
            self.op.val.precedence()
        }
    }
}

impl<'src> AstNode<'src> for Binary<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.lhs.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.rhs.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.lhs, &self.rhs).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check op because it's between them, so it must be in the same expansion if the other two are
        (&self.lhs, &self.rhs).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Unary<'src> {
    pub op: SpecToken<'src, Operator>,
    pub operand: Expr<'src>,
    pub side: OpSide,
}

impl Unary<'_> {
    pub const fn precedence(&self) -> ExprPrecedence {
        if matches!(self.op.val, Operator::Neg) {
            ExprPrecedence::UnaryPre
        } else {
            self.op.val.precedence()
        }
    }
}

impl<'src> AstNode<'src> for Unary<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        match self.side {
            OpSide::Postfix => self.operand.get_range_start(source),
            OpSide::Prefix => self.op.get_range_start(source),
        }
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        match self.side {
            OpSide::Postfix => self.op.get_range_end(source),
            OpSide::Prefix => self.operand.get_range_end(source),
        }
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self.side {
            OpSide::Postfix => (&self.operand, &self.op).get_range(source),
            OpSide::Prefix => (&self.op, &self.operand).get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self.side {
            OpSide::Postfix => (&self.operand, &self.op).expansion(source),
            OpSide::Prefix => (&self.op, &self.operand).expansion(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Grouping<'src> {
    /// `(`
    pub open: Lexeme<'src>,
    pub expr: Expr<'src>,
    /// `)`
    pub close: Lexeme<'src>,
}

impl<'src> AstNode<'src> for Grouping<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check expr because it's between open and close,
        // and therefore must be in the same expansion if the other two are.
        // this is also cheaper, since now we don't have to recursively check the inner expressions :)
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Sequence<'src> {
    /// `[`
    pub open: Lexeme<'src>,
    pub expr: Expr<'src>,
    /// `]`
    pub close: Lexeme<'src>,
}

impl<'src> AstNode<'src> for Sequence<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        // don't need to check expr because it's between open and close,
        // and therefore must be in the same expansion if the other two are.
        // this is also cheaper, since now we don't have to recursively check the inner expressions :)
        (&self.open, &self.close).expansion(source)
    }
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

impl<'src> TryFrom<SpecToken<'src, Keyword>> for SpecToken<'src, OrTypeKeyword> {
    type Error = ();

    fn try_from(
        SpecToken { lex, mac, val }: SpecToken<'src, Keyword>,
    ) -> Result<Self, Self::Error> {
        OrTypeKeyword::new(val)
            .map(|val| Self { lex, mac, val })
            .ok_or(())
    }
}
impl<'src> TryFrom<Token<'src>> for SpecToken<'src, OrTypeKeyword> {
    type Error = ();

    fn try_from(value: Token<'src>) -> Result<Self, Self::Error> {
        <SpecToken<'src, Keyword>>::try_from(value)?.try_into()
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct OrType<'src> {
    // `|`
    pub pipe: Lexeme<'src>,
    // `none`, `fail`, or `nevr`
    pub ty: SpecToken<'src, OrTypeKeyword>,
}

impl<'src> AstNode<'src> for OrType<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.pipe.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ty.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.pipe, &self.ty).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.pipe, &self.ty).expansion(source)
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'src> TryFrom<SpecToken<'src, Keyword>> for SpecToken<'src, TypenameKeyword> {
    type Error = ();

    fn try_from(
        SpecToken { lex, mac, val }: SpecToken<'src, Keyword>,
    ) -> Result<Self, Self::Error> {
        TypenameKeyword::new(val)
            .map(|val| Self { lex, mac, val })
            .ok_or(())
    }
}
impl<'src> TryFrom<Token<'src>> for SpecToken<'src, TypenameKeyword> {
    type Error = ();

    fn try_from(value: Token<'src>) -> Result<Self, Self::Error> {
        <SpecToken<'src, Keyword>>::try_from(value)?.try_into()
    }
}
impl<'src> TryFrom<SpecToken<'src, Keyword>> for SpecToken<'src, Option<TypenameKeyword>> {
    type Error = ();

    fn try_from(
        SpecToken { lex, mac, val }: SpecToken<'src, Keyword>,
    ) -> Result<Self, Self::Error> {
        TypenameKeyword::new(val)
            .map(|val| Self {
                lex,
                mac,
                val: Some(val),
            })
            .ok_or(())
    }
}
/// NOTE: Success does not guarantee it is the name of a type! It only guarantees it CAN be!
impl<'src> TryFrom<Token<'src>> for SpecToken<'src, Option<TypenameKeyword>> {
    type Error = ();

    fn try_from(Token { lex, mac, val }: Token<'src>) -> Result<Self, Self::Error> {
        match val {
            LexValue::Identifier => Ok(SpecToken {
                lex,
                mac,
                val: None,
            }),
            LexValue::Keyword(val) => SpecToken::<'src, Keyword> { lex, mac, val }.try_into(),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct NamespacePathPart<'src> {
    /// Identifier
    pub name: Lexeme<'src>,
    /// `::`
    pub sep: Lexeme<'src>,
}

impl<'src> AstNode<'src> for NamespacePathPart<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.name.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.sep.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.name, &self.sep).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.name, &self.sep).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct GenericGroup<'src> {
    /// `<`
    pub open: Lexeme<'src>,
    /// (type, `,`)
    /// only the last comma will ever be [`None`]
    pub list: Vec<(TypeExpr<'src>, Option<Lexeme<'src>>)>,
    /// `>`
    pub close: Lexeme<'src>,
}

impl<'src> AstNode<'src> for GenericGroup<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct IffClause<'src> {
    /// `<=>`
    pub iff: Lexeme<'src>,
    pub bound: TypeBound<'src>,
}

impl<'src> AstNode<'src> for IffClause<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.iff.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.bound.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.iff, &self.bound).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.iff, &self.bound).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Typename<'src> {
    pub path: Vec<NamespacePathPart<'src>>,
    pub name: SpecToken<'src, Option<TypenameKeyword>>,
    pub args: Option<GenericGroup<'src>>,
}

impl<'src> AstNode<'src> for Typename<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        (self.path.as_slice(), &self.name).get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (&self.name, self.args.as_ref()).get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        ((self.path.as_slice(), &self.name), self.args.as_ref()).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        ((self.path.as_slice(), &self.name), self.args.as_ref()).expansion(source)
    }
}

/// A concrete type
#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct TypeExpr<'src> {
    /// Identifier
    pub name: Typename<'src>,
    pub or_ty: Option<OrType<'src>>,
}

impl<'src> AstNode<'src> for TypeExpr<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.name.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (&self.name, self.or_ty.as_ref()).get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.name, self.or_ty.as_ref()).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.name, self.or_ty.as_ref()).expansion(source)
    }
}

// uses `TokenEq` instead of `Operator` because i cant make up my mind about the syntax
impl Combinator {
    pub const fn new(value: TokenEq) -> Option<Self> {
        Some(match value {
            TokenEq::Punctuation(Punctuation::Add) => Self::All,
            TokenEq::Punctuation(Punctuation::Or) => Self::Any,
            TokenEq::Punctuation(Punctuation::Div) => Self::One,
            _ => return None,
        })
    }

    pub const fn to_token_eq(self) -> TokenEq {
        match self {
            Self::All => TokenEq::Punctuation(Punctuation::Add),
            Self::Any => TokenEq::Punctuation(Punctuation::Or),
            Self::One => TokenEq::Punctuation(Punctuation::Div),
        }
    }

    pub const fn as_str(self) -> &'static str {
        self.to_token_eq().as_str()
    }
}

impl<'src> TryFrom<SpecToken<'src, TokenEq>> for SpecToken<'src, Combinator> {
    type Error = ();

    fn try_from(
        SpecToken { lex, mac, val }: SpecToken<'src, TokenEq>,
    ) -> Result<Self, Self::Error> {
        Combinator::new(val)
            .map(|val| Self { lex, mac, val })
            .ok_or(())
    }
}
impl<'src> TryFrom<Token<'src>> for SpecToken<'src, Combinator> {
    type Error = ();

    fn try_from(value: Token<'src>) -> Result<Self, Self::Error> {
        <SpecToken<'src, TokenEq>>::try_from(value)?.try_into()
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ComboBound<'src> {
    pub lhs: TypeBound<'src>,
    pub sep: SpecToken<'src, Combinator>,
    pub rhs: TypeBound<'src>,
}

impl<'src> AstNode<'src> for ComboBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.lhs.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.rhs.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.lhs, &self.rhs).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.lhs, &self.rhs).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct NegativeBound<'src> {
    /// `!`
    pub not: Lexeme<'src>,
    pub rhs: TypeBound<'src>,
}

impl<'src> AstNode<'src> for NegativeBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.not.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.rhs.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.not, &self.rhs).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.not, &self.rhs).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ExactTypeBound<'src> {
    /// `only`
    pub kw: Lexeme<'src>,
    pub ty: TypeExpr<'src>,
}

impl<'src> AstNode<'src> for ExactTypeBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ty.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.ty).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.ty).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ColonBound<'src> {
    /// `:`
    pub colon: Lexeme<'src>,
    pub bound: TypeBound<'src>,
}

impl<'src> AstNode<'src> for ColonBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.colon.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.bound.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.colon, &self.bound).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.colon, &self.bound).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct MemberBound<'src> {
    /// `has`
    pub kw: Lexeme<'src>,
    /// Identifier
    pub member: Lexeme<'src>,
    pub req: Option<ColonBound<'src>>,
}

impl<'src> AstNode<'src> for MemberBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (&self.member, self.req.as_ref()).get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, (&self.member, self.req.as_ref())).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, (&self.member, self.req.as_ref())).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ShapeBound<'src> {
    /// `fun`
    pub kw: Lexeme<'src>,
    /// `(`
    pub open: Lexeme<'src>,
    /// (bound, `,`)
    /// only the last comma will ever be [`None`]
    pub args: Vec<(TypeExpr<'src>, Option<Lexeme<'src>>)>,
    /// `)`
    pub close: Lexeme<'src>,
    // TBD: should return be optional when none?
    /// `->`
    pub arrow: Lexeme<'src>,
    pub ret: TypeExpr<'src>,
}

impl<'src> AstNode<'src> for ShapeBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ret.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.ret).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.ret).expansion(source)
    }
}

/// UNDER CONSTRUCTION
#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct OverloadBound<'src> {
    _marker: std::marker::PhantomData<&'src str>,
    // TODO
}

impl<'src> AstNode<'src> for OverloadBound<'src> {
    fn get_range_start(&self, _source: &'src str) -> Option<usize> {
        todo!()
    }

    fn get_range_end(&self, _source: &'src str) -> Option<usize> {
        todo!()
    }

    fn get_range(&self, _source: &'src str) -> Option<Range<usize>> {
        todo!()
    }

    fn expansion(&self, _source: &'src str) -> Option<ExpansionData<'src>> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ExprBound<'src> {
    /// `for`
    pub kw: Lexeme<'src>,
    /// Identifier
    pub name: Lexeme<'src>,
    /// `:`
    pub colon: Lexeme<'src>,
    pub expr: Expr<'src>,
}

impl<'src> AstNode<'src> for ExprBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.expr).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.expr).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct GroupedBound<'src> {
    /// `(`
    pub open: Lexeme<'src>,
    pub inner: TypeBound<'src>,
    /// `)`
    pub close: Lexeme<'src>,
}

impl<'src> AstNode<'src> for GroupedBound<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum TypeBound<'src> {
    Combo(Box<ComboBound<'src>>),
    Not(Box<NegativeBound<'src>>),
    Exact(Box<ExactTypeBound<'src>>),
    Has(Box<MemberBound<'src>>),
    Shape(Box<ShapeBound<'src>>),
    Supports(Box<OverloadBound<'src>>),
    Satisfies(Box<ExprBound<'src>>),
    Group(Box<GroupedBound<'src>>),
}

impl<'src> TypeBound<'src> {
    pub fn combo(value: ComboBound<'src>) -> Self {
        Self::Combo(Box::new(value))
    }

    pub fn not(value: NegativeBound<'src>) -> Self {
        Self::Not(Box::new(value))
    }

    pub fn exact(value: ExactTypeBound<'src>) -> Self {
        Self::Exact(Box::new(value))
    }

    pub fn has(value: MemberBound<'src>) -> Self {
        Self::Has(Box::new(value))
    }

    pub fn shape(value: ShapeBound<'src>) -> Self {
        Self::Shape(Box::new(value))
    }

    pub fn supports(value: OverloadBound<'src>) -> Self {
        Self::Supports(Box::new(value))
    }

    pub fn satisfies(value: ExprBound<'src>) -> Self {
        Self::Satisfies(Box::new(value))
    }

    pub fn group(value: GroupedBound<'src>) -> Self {
        Self::Group(Box::new(value))
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Literal<'src> {
    pub value: Token<'src>,
}

impl<'src> AstNode<'src> for Literal<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.value.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.value.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.value.get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.value.expansion(source)
    }
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'src> TryFrom<SpecToken<'src, Keyword>> for SpecToken<'src, DirectiveKw> {
    type Error = ();

    fn try_from(
        SpecToken { lex, mac, val }: SpecToken<'src, Keyword>,
    ) -> Result<Self, Self::Error> {
        DirectiveKw::new(val)
            .map(|val| Self { lex, mac, val })
            .ok_or(())
    }
}
impl<'src> TryFrom<Token<'src>> for SpecToken<'src, DirectiveKw> {
    type Error = ();

    fn try_from(value: Token<'src>) -> Result<Self, Self::Error> {
        <SpecToken<'src, Keyword>>::try_from(value)?.try_into()
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Directive<'src> {
    pub kw: SpecToken<'src, DirectiveKw>,
}

impl<'src> AstNode<'src> for Directive<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.kw.get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.kw.expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Variable<'src> {
    /// Identifier
    pub ident: Lexeme<'src>,
}

impl<'src> AstNode<'src> for Variable<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.ident.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ident.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.ident.get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.ident.expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum BlockExpr<'src> {
    If(Box<IfStmt<'src>>),
    /// Only `loop` or `cord`
    Loop(Box<LoopBlock<'src>>),
    Block(BlockStmt<'src>),
}

impl<'src> BlockExpr<'src> {
    pub const fn precedence(&self) -> ExprPrecedence {
        ExprPrecedence::Or
    }

    pub fn if_expr(inner: IfStmt<'src>) -> Self {
        Self::If(Box::new(inner))
    }

    pub fn loop_expr(inner: LoopBlock<'src>) -> Self {
        Self::Loop(Box::new(inner))
    }

    pub const fn block(inner: BlockStmt<'src>) -> Self {
        Self::Block(inner)
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum Expr<'src> {
    Block(BlockExpr<'src>),
    Binary(Box<Binary<'src>>),
    Unary(Box<Unary<'src>>),
    Literal(Literal<'src>),
    Directive(Directive<'src>),
    Variable(Variable<'src>),
    Grouping(Box<Grouping<'src>>),
    Type(Box<TypeExpr<'src>>),
}

impl<'src> Expr<'src> {
    pub const fn precedence(&self) -> ExprPrecedence {
        match self {
            Self::Block(inner) => inner.precedence(),
            Self::Binary(inner) => inner.precedence(),
            Self::Unary(inner) => inner.precedence(),
            Self::Directive(_inner) => todo!(),
            Self::Literal(_) | Self::Variable(_) | Self::Grouping(_) => ExprPrecedence::Primary,
            Self::Type(_inner) => todo!(),
        }
    }

    pub fn binary(inner: Binary<'src>) -> Self {
        Self::Binary(Box::new(inner))
    }

    pub fn unary(inner: Unary<'src>) -> Self {
        Self::Unary(Box::new(inner))
    }

    pub const fn literal(inner: Literal<'src>) -> Self {
        Self::Literal(inner)
    }

    pub const fn directive(inner: Directive<'src>) -> Self {
        Self::Directive(inner)
    }

    pub const fn variable(inner: Variable<'src>) -> Self {
        Self::Variable(inner)
    }

    pub fn grouping(inner: Grouping<'src>) -> Self {
        Self::Grouping(Box::new(inner))
    }

    pub fn type_expr(inner: TypeExpr<'src>) -> Self {
        Self::Type(Box::new(inner))
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum OrBody<'src> {
    More(IfStmt<'src>),
    Last(BlockStmt<'src>),
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct OrStmt<'src> {
    /// `or`
    pub or_kw: Lexeme<'src>,
    pub body: OrBody<'src>,
}

impl<'src> AstNode<'src> for OrStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.or_kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.body.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.or_kw, &self.body).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.or_kw, &self.body).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct IfStmt<'src> {
    /// `if`
    pub if_kw: Lexeme<'src>,
    pub pred: Expr<'src>,
    pub then: BlockStmt<'src>,
    pub or: Option<Box<OrStmt<'src>>>,
}

impl<'src> AstNode<'src> for IfStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.if_kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        (&self.then, self.or.as_deref()).get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.if_kw, (&self.then, self.or.as_deref())).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.if_kw, (&self.then, self.or.as_deref())).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct DeferParam<'src> {
    /// `(`
    pub open: Lexeme<'src>,
    pub param: BindingPattern<'src>,
    /// `)`
    pub close: Lexeme<'src>,
}

impl<'src> AstNode<'src> for DeferParam<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct DeclDefer<'src> {
    /// `defer`
    pub kw: Lexeme<'src>,
    pub param: Option<DeferParam<'src>>,
    pub on_pop: BlockStmt<'src>,
}

impl<'src> AstNode<'src> for DeclDefer<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.on_pop.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.on_pop).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.on_pop).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoopKind<'src> {
    /// `kw` is `loop`
    Loop,

    /// `kw` is `cord`
    Cord,

    /// `kw` is `rep`
    Rep {
        /// The condition on which the loop will continue
        pred: Expr<'src>,
    },

    /// `kw` is `for`
    For {
        /// The variable `iter` items are stored in
        bind: BindingPattern<'src>,
        /// `in`
        in_kw: Lexeme<'src>,
        /// An expression which produces an iterable value
        /// (i.e. a value which overloads `++`)
        iter: Expr<'src>,
    },
}

impl<'src> AstNode<'src> for LoopKind<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        match self {
            Self::Loop | Self::Cord => None,
            Self::Rep { pred } => pred.get_range_start(source),
            Self::For { bind, .. } => bind.get_range_start(source),
        }
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        match self {
            Self::Loop | Self::Cord => None,
            Self::Rep { pred } => pred.get_range_end(source),
            Self::For { iter, .. } => iter.get_range_end(source),
        }
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        match self {
            Self::Loop | Self::Cord => None,
            Self::Rep { pred } => pred.get_range(source),
            Self::For { bind, iter, .. } => (bind, iter).get_range(source),
        }
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        match self {
            Self::Loop | Self::Cord => None,
            Self::Rep { pred } => pred.expansion(source),
            Self::For { bind, iter, .. } => (bind, iter).expansion(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct LoopBlock<'src> {
    pub label: Option<ScopeLabel<'src>>,
    /// variant-dependent
    pub kw: Lexeme<'src>,
    pub kind: LoopKind<'src>,
    /// the code that gets repeated
    pub block: BlockStmt<'src>,
}

impl<'src> AstNode<'src> for LoopBlock<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        (&self.label, &self.kw).get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.block.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        ((&self.label, &self.kw), &self.block).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        ((&self.label, &self.kw), &self.block).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct DoneStmt<'src> {
    /// `done`
    pub kw: Lexeme<'src>,
    pub label: Option<Label<'src>>,
    pub value: Option<Expr<'src>>,
    /// `;`
    pub semi: Lexeme<'src>,
}

impl<'src> AstNode<'src> for DoneStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.semi.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.semi).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.semi).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct SkipStmt<'src> {
    /// `skip`
    pub kw: Lexeme<'src>,
    pub label: Option<Label<'src>>,
    /// `;`
    pub semi: Lexeme<'src>,
}

impl<'src> AstNode<'src> for SkipStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.semi.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.semi).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.semi).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct BlockStmt<'src> {
    /// `{`
    pub open: Lexeme<'src>,
    pub body: Vec<Declaration<'src>>,
    /// `}`
    pub close: Lexeme<'src>,
}

impl<'src> AstNode<'src> for BlockStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.open.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.close.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.open, &self.close).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.open, &self.close).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ExprStmt<'src> {
    pub expr: Expr<'src>,
    /// `;`
    pub semi: Lexeme<'src>,
}

impl<'src> AstNode<'src> for ExprStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.semi.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.expr, &self.semi).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.expr, &self.semi).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct PrintStmt<'src> {
    /// `prnt`
    pub kw: Lexeme<'src>,
    pub group: Grouping<'src>,
    /// `;`
    pub semi: Lexeme<'src>,
}

impl<'src> AstNode<'src> for PrintStmt<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.semi.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.kw, &self.semi).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.kw, &self.semi).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum Statement<'src> {
    Expr(ExprStmt<'src>),
    // TODO: im not too fond of the print keyword
    Print(PrintStmt<'src>),
    If(IfStmt<'src>),
    Loop(LoopBlock<'src>),
    Done(DoneStmt<'src>),
    Skip(SkipStmt<'src>),
    Block(BlockStmt<'src>),
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Program<'src> {
    pub decls: Vec<Declaration<'src>>,
}

impl<'src> std::ops::Deref for Program<'src> {
    type Target = Vec<Declaration<'src>>;

    fn deref(&self) -> &Self::Target {
        &self.decls
    }
}

impl std::ops::DerefMut for Program<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.decls
    }
}

impl<'src> IntoIterator for Program<'src> {
    type Item = Declaration<'src>;
    type IntoIter = std::vec::IntoIter<Declaration<'src>>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.into_iter()
    }
}

impl<'prgm, 'src> IntoIterator for &'prgm Program<'src> {
    type Item = &'prgm Declaration<'src>;
    type IntoIter = std::slice::Iter<'prgm, Declaration<'src>>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.iter()
    }
}

impl<'prgm, 'src> IntoIterator for &'prgm mut Program<'src> {
    type Item = &'prgm mut Declaration<'src>;
    type IntoIter = std::slice::IterMut<'prgm, Declaration<'src>>;

    fn into_iter(self) -> Self::IntoIter {
        self.decls.iter_mut()
    }
}

impl<'src> AstNode<'src> for Program<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.decls.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.decls.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        self.decls.get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        self.decls.expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Assignment<'src> {
    /// `=`
    pub op: Lexeme<'src>,
    pub expr: Expr<'src>,
}

impl<'src> AstNode<'src> for Assignment<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.op.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.expr.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.op, &self.expr).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.op, &self.expr).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct Label<'src> {
    /// `.`
    pub dot: Lexeme<'src>,
    /// identifier
    pub name: Lexeme<'src>,
}

impl<'src> AstNode<'src> for Label<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.dot.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.name.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.dot, &self.name).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.dot, &self.name).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct ScopeLabel<'src> {
    pub label: Label<'src>,
    /// `:`
    pub colon: Lexeme<'src>,
}

impl<'src> AstNode<'src> for ScopeLabel<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.label.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.colon.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.label, &self.colon).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.label, &self.colon).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct TypeAnnotation<'src> {
    /// `:`
    pub colon: Lexeme<'src>,
    // TODO: should type annotations be different from type expressions?
    pub ty: TypeExpr<'src>,
}

impl<'src> AstNode<'src> for TypeAnnotation<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.colon.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.ty.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.colon, &self.ty).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.colon, &self.ty).expansion(source)
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

impl<'src> TryFrom<SpecToken<'src, Keyword>> for SpecToken<'src, VarKeyword> {
    type Error = ();

    fn try_from(
        SpecToken { lex, mac, val }: SpecToken<'src, Keyword>,
    ) -> Result<Self, Self::Error> {
        VarKeyword::new(val)
            .map(|val| Self { lex, mac, val })
            .ok_or(())
    }
}
impl<'src> TryFrom<Token<'src>> for SpecToken<'src, VarKeyword> {
    type Error = ();

    fn try_from(value: Token<'src>) -> Result<Self, Self::Error> {
        <SpecToken<'src, Keyword>>::try_from(value)?.try_into()
    }
}

#[derive(Debug, Clone, PartialEq, AstNodeF)]
pub struct DeclVar<'src> {
    /// `uni`, `let`, or `pvt`
    pub var_kw: SpecToken<'src, VarKeyword>,
    /// Identifier
    pub binding: BindingPattern<'src>,
    pub ty: Option<TypeAnnotation<'src>>,
    pub assignment: Option<Assignment<'src>>,
    /// `;`
    pub semi: Lexeme<'src>,
}

impl<'src> AstNode<'src> for DeclVar<'src> {
    fn get_range_start(&self, source: &'src str) -> Option<usize> {
        self.var_kw.get_range_start(source)
    }

    fn get_range_end(&self, source: &'src str) -> Option<usize> {
        self.semi.get_range_end(source)
    }

    fn get_range(&self, source: &'src str) -> Option<Range<usize>> {
        (&self.var_kw, &self.semi).get_range(source)
    }

    fn expansion(&self, source: &'src str) -> Option<ExpansionData<'src>> {
        (&self.var_kw, &self.semi).expansion(source)
    }
}

#[derive(Debug, Clone, PartialEq, AstEnum, AstNodeF)]
pub enum Declaration<'src> {
    Var(DeclVar<'src>),
    Defer(DeclDefer<'src>),
    Stmt(Statement<'src>),
}

impl<'src> Declaration<'src> {
    pub const fn var(inner: DeclVar<'src>) -> Self {
        Self::Var(inner)
    }

    pub const fn defer(inner: DeclDefer<'src>) -> Self {
        Self::Defer(inner)
    }

    pub const fn stmt(inner: Statement<'src>) -> Self {
        Self::Stmt(inner)
    }
}
