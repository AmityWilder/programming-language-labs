//! Context-free grammar

#![allow(clippy::missing_docs_in_private_items, reason = "under construction")]

use crate::{
    error::{ContextError, Expecting},
    scanner::token::{Keyword, Punctuation, Token, TokenType, TokenValue},
};

pub trait MatchRule<'a>: Sized {
    type Output;

    fn try_pull<'b>(
        self,
        source: &'a str,
        tokens: &'b [Token<'a>],
        expecting: Expecting,
    ) -> Result<(Self::Output, &'b [Token<'a>]), ContextError<'a>>;
}

impl<'a, U, F> MatchRule<'a> for F
where
    F: FnOnce(Token<'a>) -> Option<U>,
{
    type Output = U;

    fn try_pull<'b>(
        self,
        source: &'a str,
        mut tokens: &'b [Token<'a>],
        expecting: Expecting,
    ) -> Result<(Self::Output, &'b [Token<'a>]), ContextError<'a>> {
        tokens
            .split_off_first()
            .ok_or_else(|| ContextError::missing(source, expecting))
            .and_then(|&token| {
                self(token)
                    .map(|x| (x, tokens))
                    .ok_or_else(|| ContextError::unexpected(token, source, expecting))
            })
    }
}

pub trait Rule<'a>: Sized {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>>;
}

/// `<let-statement> ::= "let" <binding> "=" <expression>`
#[derive(Debug, Clone, PartialEq)]
pub struct LetStatement<'a> {
    pub let_kw: &'a str,
    pub binding: Binding<'a>,
    pub assign_kw: &'a str,
    pub expression: Expression<'a>,
}

impl<'a> Rule<'a> for LetStatement<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (let_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Let),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("`let` keyword"),
        )?;

        let (binding, tokens) = Binding::try_pull(source, tokens)?;

        let (assign_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::Assign),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("`=` operator"),
        )?;

        let (expression, tokens) = Expression::try_pull(source, tokens)?;

        Ok((
            Self {
                let_kw,
                binding,
                assign_kw,
                expression,
            },
            tokens,
        ))
    }
}

/// `<binding> ::= <identifier>`
// TODO: this can be way cooler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Binding<'a> {
    name: &'a str,
}

impl<'a> Rule<'a> for Binding<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (name, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    ty: TokenType::Identifier,
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("identifier"),
        )?;

        Ok((Self { name }, tokens))
    }
}

/// `<parenthesized> ::= "(" INNER ")"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Parenthesized<'a, T> {
    pub open: &'a str,
    pub inner: T,
    pub close: &'a str,
}

impl<'a, T: Rule<'a>> Rule<'a> for Parenthesized<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::LParen),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("open parenthesis `(`"),
        )?;

        let (inner, tokens) = T::try_pull(source, tokens)?;

        let (close, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::RParen),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("close parenthesis `)`"),
        )?;

        Ok((Self { open, inner, close }, tokens))
    }
}

/// `<bracketed> ::= "[" INNER "]"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bracketed<'a, T> {
    pub open: &'a str,
    pub inner: T,
    pub close: &'a str,
}

impl<'a, T: Rule<'a>> Rule<'a> for Bracketed<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::LBrack),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("open bracket `[`"),
        )?;

        let (inner, tokens) = T::try_pull(source, tokens)?;

        let (close, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::RBrack),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("close bracket `]`"),
        )?;

        Ok((Self { open, inner, close }, tokens))
    }
}

/// `<braced> ::= "{" INNER "}"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Braced<'a, T> {
    pub open: &'a str,
    pub inner: T,
    pub close: &'a str,
}

impl<'a, T: Rule<'a>> Rule<'a> for Braced<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::LBrace),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("open brace `{`"),
        )?;

        let (inner, tokens) = T::try_pull(source, tokens)?;

        let (close, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::RBrace),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("close brace `}`"),
        )?;

        Ok((Self { open, inner, close }, tokens))
    }
}

/// `<group> ::= "(" <expression> ")"`
#[derive(Debug, Clone, PartialEq)]
pub struct Group<'a> {
    pub open: &'a str,
    pub inner: Box<Expression<'a>>,
    pub close: &'a str,
}

impl<'a> Rule<'a> for Group<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::LParen),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("open parenthesis `(`"),
        )?;

        let (inner, tokens) = Expression::try_pull(source, tokens)?;

        let (close, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(Punctuation::RParen),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("close parenthesis`)`"),
        )?;

        Ok((
            Self {
                open,
                inner: Box::new(inner),
                close,
            },
            tokens,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Literal<'a> {
    pub token: Token<'a>,
}

impl<'a> Rule<'a> for Literal<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (token, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    ty:
                        TokenType::BoolLiteral
                        | TokenType::NumberLiteral
                        | TokenType::CharLiteral
                        | TokenType::StringLiteral,
                    ..
                } => Some(token),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("literal"),
        )?;

        Ok((Self { token }, tokens))
    }
}

macro_rules! define_enum_subset {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $Enum:ident : $Super:ident {$(
            $(#[$variant_meta:meta])*
            $Variant:ident
        ),* $(,)?}
    ) => {
        $(#[$enum_meta])*
        #[doc = concat!("Subset of [`", stringify!($Super), "`]")]
        $vis enum $Enum {$(
            $(#[$variant_meta])*
            #[doc = concat!("[`", stringify!($Super), "::", stringify!($Variant), "`]")]
            $Variant = $Super::$Variant as isize,
        )*}

        impl From<$Enum> for $Super {
            fn from(value: $Enum) -> Self {
                match value {
                    $($Enum::$Variant => Self::$Variant,)*
                }
            }
        }

        impl TryFrom<$Super> for $Enum {
            type Error = ();

            fn try_from(value: $Super) -> Result<Self, Self::Error> {
                match value {
                    $($Super::$Variant => Ok(Self::$Variant),)*
                    _ => Err(())
                }
            }
        }
    };
}

define_enum_subset! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum BinaryOp : Punctuation {
        Neq,
        Nand,
        Nor,
        Xnor,
        Exponent,
        Le,
        Shl,
        Eq,
        Ge,
        Shr,
        Remainder,
        And,
        Mul,
        Add,
        Sub,
        Div,
        Lt,
        Gt,
        Xor,
        Or,
    }
}

/// ```not_code
/// <binary-operation> ::= <expression> <binary-operator> <expression>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOperation<'a> {
    pub lhs: Box<Expression<'a>>,
    pub op: (&'a str, BinaryOp),
    pub rhs: Box<Expression<'a>>,
}

impl<'a> Rule<'a> for BinaryOperation<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (lhs, tokens) = Expression::try_pull(source, tokens)?;
        let (op, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Punctuation(punc),
                    ..
                } if let Ok(op) = punc.try_into() => Some((src, op)),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("binary operator"),
        )?;
        let (rhs, tokens) = Expression::try_pull(source, tokens)?;

        Ok((
            Self {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            },
            tokens,
        ))
    }
}

/// ```not_code
/// <expression> ::= <literal>
///     | <group>
///     | <array>
///     | <binary-operation>
///     | <prefix-operator> <expression> ; TODO
///     | <expression> <postfix-operator> ; TODO
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'a> {
    Literal(Literal<'a>),
    Group(Group<'a>),
    BinOp(BinaryOperation<'a>),
}

impl<'a> Rule<'a> for Expression<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        if let Ok((value, tokens)) = Group::try_pull(source, tokens) {
            Ok((Self::Group(value), tokens))
        } else if let Ok((value, tokens)) = Literal::try_pull(source, tokens) {
            Ok((Self::Literal(value), tokens))
        } else if let Ok((value, tokens)) = BinaryOperation::try_pull(source, tokens) {
            Ok((Self::BinOp(value), tokens))
        }
        // TODO: are there other expression structures?
        else {
            Err(ContextError::missing_or_unexpected(
                tokens.first(),
                source,
                Expecting::an("expression"),
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StructBody {}

impl<'a> Rule<'a> for StructBody {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StructDef<'a> {
    pub struct_kw: &'a str,
    pub name: &'a str,
    pub body: Braced<'a, StructBody>,
}

impl<'a> Rule<'a> for StructDef<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (struct_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Struct),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("`struct` keyword"),
        )?;

        let (name, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    ty: TokenType::Identifier,
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("identifier"),
        )?;

        let (body, tokens) = <Braced<StructBody>>::try_pull(source, tokens)?;

        Ok((
            Self {
                struct_kw,
                name,
                body,
            },
            tokens,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct UnionDef<'a> {
    pub union_kw: &'a str,
}

impl<'a> Rule<'a> for UnionDef<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (union_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Union),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("`union` keyword"),
        )?;

        // TODO

        Ok((Self { union_kw }, tokens))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EnumDef<'a> {
    pub enum_kw: &'a str,
}

impl<'a> Rule<'a> for EnumDef<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (enum_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Enum),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::an("`enum` keyword"),
        )?;

        // TODO

        Ok((Self { enum_kw }, tokens))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TypeDef<'a> {
    pub type_kw: &'a str,
}

impl<'a> Rule<'a> for TypeDef<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (type_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Type),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("`type` keyword"),
        )?;

        // TODO

        Ok((Self { type_kw }, tokens))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MacroDef<'a> {
    pub def_kw: &'a str,
}

impl<'a> Rule<'a> for MacroDef<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (def_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Def),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("`def` keyword"),
        )?;

        // TODO

        Ok((Self { def_kw }, tokens))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FnDef<'a> {
    pub fn_kw: &'a str,
}

impl<'a> Rule<'a> for FnDef<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (fn_kw, tokens) = MatchRule::try_pull(
            |token| match token {
                Token {
                    src,
                    val: TokenValue::Keyword(Keyword::Fn),
                    ..
                } => Some(src),
                _ => None,
            },
            source,
            tokens,
            Expecting::a("`fn` keyword"),
        )?;

        // TODO

        Ok((Self { fn_kw }, tokens))
    }
}

/// ```not_code
/// <syntax> ::=
///     <syntax> <syntax>
///     | <struct-def>
///     | <union-def>
///     | <enum-def>
///     | <type-def>
///     | <macro-def>
///     | <fn-def>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Syntax<'a> {
    Pair(Box<(Syntax<'a>, Syntax<'a>)>),
    StructDef(StructDef<'a>),
    UnionDef(UnionDef<'a>),
    EnumDef(EnumDef<'a>),
    TypeDef(TypeDef<'a>),
    MacroDef(MacroDef<'a>),
    FnDef(FnDef<'a>),
}

impl<'a> Rule<'a> for Syntax<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        todo!()
    }
}

pub fn grammarize<'a>(
    source: &'a str,
    tokens: &[Token<'a>],
) -> Result<Syntax<'a>, ContextError<'a>> {
    Syntax::try_pull(source, tokens).map(|(syn, _)| syn)
}
