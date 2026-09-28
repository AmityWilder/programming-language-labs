//! Context-free grammar

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::{ContextError, ErrorType, Expecting},
    scanner::{
        Bracket,
        token::{CharLiteral, Punctuation, StrLiteral, Token, TokenValue, keyword::*, operator::*},
    },
};
use std::range::Range;

macro_rules! matching {
    ($value:expr, $pattern:pat $(if $guard:expr)? => $result:expr) => {
        match $value {
            $pattern $(if $guard)? => Some($result),
            _ => None,
        }
    };
}

macro_rules! pattern {
    ($pattern:pat $(if $guard:expr)? => $result:expr) => {
        |value| matching!(value, $pattern $(if $guard)? => $result)
    };
}

macro_rules! token_pattern {
    ($($field:ident$(: $pattern:pat)?),* $(if $guard:expr)? => $result:expr) => {
        pattern!(Token { $($field$(: $pattern)?,)* .. } $(if $guard)? => $result)
    };
}

/// A rule that is just a sequence of one rule after another
macro_rules! simple_rule {
    // sequence of rules
    (
        $(#[$meta:meta])*
        $vis:vis struct $Struct:ident<$lt:lifetime> {$(
            $(#[$fmeta:meta])*
            $fvis:vis $field:ident: $Type:ty
        ),* $(,)?}
    ) => {
        $(#[$meta])*
        $vis struct $Struct<$lt> {$(
            $(#[$fmeta])*
            $fvis $field: $Type
        ),*}

        impl<'src> Rule<'src> for $Struct<'src> {
            fn try_pull<'arr>(
                source: &'src str,
                tokens: &'arr [Token<'src>],
            ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
                $(let ($field, tokens) = Rule::try_pull(source, tokens)?;)*
                Ok((Self { $($field),* }, tokens))
            }
        }
    };

    // switch case of rules
    (
        $(#[$meta:meta])*
        $vis:vis enum $Enum:ident<$lt:lifetime> {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident($Type:ty),
            )*
            _ => $x:ident $expecting:literal $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $Enum<$lt> {$(
            $(#[$vmeta])*
            $Variant($Type)
        ),*}

        impl<'src> Rule<'src> for $Enum<'src> {
            fn try_pull<'arr>(
                source: &'src str,
                tokens: &'arr [Token<'src>],
            ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
                $(if let Ok(item) = Rule::try_pull(source, tokens).map(map_pull(Self::$Variant)) {
                    Ok(item)
                } else)* {
                    Err(ContextError::missing_or_unexpected(
                        tokens.first().copied(),
                        source,
                        Expecting::$x($expecting),
                    ))
                }
            }
        }
    };
}

/// A rule that is a single token matching a specific pattern
#[macro_export]
macro_rules! terminal_rule {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Struct:ident<$lt:lifetime>($($fvis:vis $Type:ty),* $(,)?)
            := ($($field:ident$(: $pattern:pat)?),+ $(,)?)
                $([ if $($guard:tt)+ ])?
            => ($result:expr)
            as $article:ident $desc:expr;
    ) => {
        $(#[$meta])*
        $vis struct $Struct<$lt>($($fvis $Type),*);

        impl<'src> $crate::grammar::Rule<'src> for $Struct<'src> {
            fn try_pull<'arr>(
                source: &'src str,
                tokens: &'arr [Token<'src>],
            ) -> Result<(Self, &'arr [Token<'src>]), $crate::error::ContextError<'src>> {
                $crate::grammar::try_pull_matching(
                    |token| match token {
                        Token { $($field$(: $pattern)?,)+ .. } $(if $($guard)+)? => Some($result),
                        _ => None,
                    },
                    tokens,
                )
                .map_err(|token| {
                    $crate::error::ContextError::missing_or_unexpected(token, source, $crate::error::Expecting::$article($desc))
                })
            }
        }
    };
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

fn map_pull<'src, 'arr, T, U, F>(
    f: F,
) -> impl FnOnce((T, &'arr [Token<'src>])) -> (U, &'arr [Token<'src>])
where
    F: FnOnce(T) -> U,
{
    move |(x, tokens)| (f(x), tokens)
}

pub fn try_pull_matching<'src: 'arr, 'arr, T, F>(
    f: F,
    mut tokens: &'arr [Token<'src>],
) -> Result<(T, &'arr [Token<'src>]), Option<Token<'src>>>
where
    F: FnOnce(Token<'src>) -> Option<T>,
{
    tokens
        .split_off_first()
        .ok_or(None)
        .and_then(|&token| f(token).map(|x| (x, tokens)).ok_or(Some(token)))
}

pub trait Rule<'src>: Sized {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>>;
}

impl<'src, T: Rule<'src>> Rule<'src> for Box<T> {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        T::try_pull(source, tokens).map(map_pull(Box::new))
    }
}

impl<'src, T: Rule<'src>> Rule<'src> for Option<T> {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        Ok(T::try_pull(source, tokens)
            .map(map_pull(Some))
            .unwrap_or((None, tokens)))
    }
}

impl<'src, T: Rule<'src>> Rule<'src> for Vec<T> {
    fn try_pull<'arr>(
        source: &'src str,
        mut tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        Ok((
            std::iter::from_fn(|| T::try_pull(source, tokens).ok())
                .map(|(item, tkns)| {
                    tokens = tkns;
                    item
                })
                .collect(),
            tokens,
        ))
    }
}

macro_rules! tuple_rule {
    ($($T:ident),+ $(,)?) => {
        impl<'src, $($T: Rule<'src>),+> Rule<'src> for ($($T),+) {
            fn try_pull<'arr>(
                source: &'src str,
                tokens: &'arr [Token<'src>],
            ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
                $(#[expect(non_snake_case)] let ($T, tokens) = Rule::try_pull(source, tokens)?;)+
                Ok((($($T),+), tokens))
            }
        }
    };
}

tuple_rule!(T1, T2);
tuple_rule!(T1, T2, T3);
tuple_rule!(T1, T2, T3, T4);
tuple_rule!(T1, T2, T3, T4, T5);
tuple_rule!(T1, T2, T3, T4, T5, T6);

simple_rule! {
    /// [`LetStatement`] ::= "let" [`Binding`] "=" [`Expression`] ";"
    #[derive(Debug, Clone, PartialEq)]
    pub struct LetStatement<'src> {
        pub let_kw: LetKeyword<'src>,
        pub binding: Binding<'src>,
        pub assign_kw: AssignOp<'src>,
        pub expression: Expression<'src>,
        pub semi: SemiOp<'src>,
    }
}

simple_rule! {
    /// [`Binding`] ::= [`Identifier`]
    // TODO: this can be way cooler
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Binding<'src> {
        name: Identifier<'src>,
    }
}

fn specify_bracket_err(
    expect: (Bracket, Range<usize>),
    ContextError { source, range, err }: ContextError,
) -> ContextError {
    ContextError {
        source,
        range,
        err: match err {
            ErrorType::MissingToken { expect: _ } => ErrorType::MissingCloseBracket { expect },

            ErrorType::UnexpectedToken {
                expect: _,
                actual:
                    Token {
                        val:
                            TokenValue::Punctuation(
                                actual @ (Punctuation::RParen
                                | Punctuation::RBrack
                                | Punctuation::RBrace),
                            ),
                        ..
                    },
            } => ErrorType::IncorrectCloseBracket {
                expect,
                actual: match actual {
                    Punctuation::RParen => Bracket::Paren,
                    Punctuation::RBrack => Bracket::Brack,
                    Punctuation::RBrace => Bracket::Brace,
                    _ => unreachable!("guarded by match arm"),
                },
            },

            _ => err,
        },
    }
}

/// [`Parenthesized`] ::= "(" `T` ")"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Parenthesized<'src, T> {
    pub open: LParenOp<'src>,
    pub inner: T,
    pub close: RParenOp<'src>,
}

/// Not a simple rule, because brackets have special errors
impl<'src, T: Rule<'src>> Rule<'src> for Parenthesized<'src, T> {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        let (open, tokens) = LParenOp::try_pull(source, tokens)?;

        let (inner, tokens) = Rule::try_pull(source, tokens)?;

        let (close, tokens) = Rule::try_pull(source, tokens).map_err(|e| {
            specify_bracket_err(
                (
                    Bracket::Paren,
                    source
                        .substr_range(open.0)
                        .expect("open should be a substr of source"),
                ),
                e,
            )
        })?;

        Ok((Self { open, inner, close }, tokens))
    }
}

#[expect(clippy::doc_link_with_quotes, reason = "not a doc link")]
/// [`Bracketed`] ::= "[" `T` "]"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bracketed<'src, T> {
    pub open: LBrackOp<'src>,
    pub inner: T,
    pub close: RBrackOp<'src>,
}

/// Not a simple rule, because brackets have special errors
impl<'src, T: Rule<'src>> Rule<'src> for Bracketed<'src, T> {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        let (open, tokens) = LBrackOp::try_pull(source, tokens)?;

        let expect = (
            Bracket::Brack,
            source
                .substr_range(open.0)
                .expect("open should be a substr of source"),
        );

        let (inner, tokens) = Rule::try_pull(source, tokens)?;

        let (close, tokens) =
            Rule::try_pull(source, tokens).map_err(|e| specify_bracket_err(expect, e))?;

        Ok((Self { open, inner, close }, tokens))
    }
}

/// [`Braced`] ::= "{" `T` "}"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Braced<'src, T> {
    /// `{`
    pub open: LBraceOp<'src>,
    pub inner: T,
    /// `}`
    pub close: RBraceOp<'src>,
}

/// Not a simple rule, because brackets have special errors
impl<'src, T: Rule<'src>> Rule<'src> for Braced<'src, T> {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        let (open, tokens) = LBraceOp::try_pull(source, tokens)?;

        let expect = (
            Bracket::Brace,
            source
                .substr_range(open.0)
                .expect("open should be a substr of source"),
        );

        let (inner, tokens) = Rule::try_pull(source, tokens)?;

        let (close, tokens) =
            Rule::try_pull(source, tokens).map_err(|e| specify_bracket_err(expect, e))?;

        Ok((Self { open, inner, close }, tokens))
    }
}

terminal_rule! {
    /// [`LiteralBool`] ::= "true" | "fals"
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralBool<'src>(pub &'src str, pub bool) := (lex, val: TokenValue::BoolLiteral(val)) => (Self(lex, val)) as a "boolean literal";
}
terminal_rule! {
    /// [`LiteralUInt`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralUInt<'src>(pub &'src str, pub usize) := (lex, val: TokenValue::UIntLiteral(val)) => (Self(lex, val)) as a "uint literal";
}
terminal_rule! {
    /// [`LiteralSInt`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralSInt<'src>(pub &'src str, pub isize) := (lex, val: TokenValue::SIntLiteral(val)) => (Self(lex, val)) as an "sint literal";
}
terminal_rule! {
    /// [`LiteralFrac`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct LiteralFrac<'src>(pub &'src str, pub f64) := (lex, val: TokenValue::FltLiteral(val)) => (Self(lex, val)) as a "frac literal";
}
terminal_rule! {
    /// [`LiteralChar`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralChar<'src>(pub &'src str, pub CharLiteral) := (lex, val: TokenValue::CharLiteral(val)) => (Self(lex, val)) as a "char literal";
}
terminal_rule! {
    /// [`LiteralStr`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralStr<'src>(pub &'src str, pub StrLiteral<'src>) := (lex, val: TokenValue::StringLiteral(val)) => (Self(lex, val)) as a "string literal";
}

simple_rule! {
    /// [`IntLiteral`] ::= [`LiteralUInt`] | [`LiteralSInt`]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum IntLiteral<'src> {
        UInt(LiteralUInt<'src>),
        SInt(LiteralSInt<'src>),
        _ => an "integer literal"
    }
}

simple_rule! {
    /// [`NumLiteral`] ::= [`IntLiteral`] | [`LiteralFrac`]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum NumLiteral<'src> {
        Int(IntLiteral<'src>),
        Flt(LiteralFrac<'src>),
        _ => a "number literal"
    }
}

simple_rule! {
    /// [`Literal`] ::= [`LiteralBool`] | [`NumLiteral`] | [`LiteralChar`] | [`LiteralStr`]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Literal<'src> {
        Bool(LiteralBool<'src>),
        Num(NumLiteral<'src>),
        Char(LiteralChar<'src>),
        Str(LiteralStr<'src>),
        _ => a "literal"
    }
}

simple_rule! {
    /// [`Primary`] ::= [`Literal`] | "(" [`Expression`] ")"
    #[derive(Debug, Clone, PartialEq)]
    pub enum Primary<'src> {
        Literal(Literal<'src>),
        Expr(Parenthesized<'src, Expression<'src>>),
        _ => a "literal or parenthesized expression"
    }
}

define_enum_subset! {
    /// [`UnaryOpPunc`] ::= "!" | "#" | "@"
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum UnaryOpPunc : Punctuation {
        Not,
        MacroStringify,
        Ref,
    }
}

terminal_rule! {
    /// [`UnaryOp`] ::= [`UnaryOpPunc`]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UnaryOp<'src>(pub &'src str, pub UnaryOpPunc)
        := (lex, val: TokenValue::Punctuation(val))
            [ if let Ok(val) = UnaryOpPunc::try_from(val) ]
        => (Self(lex, val)) as a "unary operator";
}

simple_rule! {
    /// [`Unary`] ::= [`UnaryOp`] [`Unary`] | [`Primary`]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Unary<'src> {
        Unary(Box<(UnaryOp<'src>, Unary<'src>)>),
        Primary(Primary<'src>),
        _ => a "unary or primary"
    }
}

define_enum_subset! {
    /// [`FactorPunc`] ::= "*" | "/"
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum FactorPunc : Punctuation {
        Mul,
        Div,
    }
}

terminal_rule! {
    /// [`FactorOp`] ::= [`FactorPunc`]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FactorOp<'src>(pub &'src str, pub FactorPunc)
        := (lex, val: TokenValue::Punctuation(val))
            [ if let Ok(val) = FactorPunc::try_from(val) ]
        => (Self(lex, val)) as a "`*` or `/` operator";
}

simple_rule! {
    /// [`Factor`] ::= [`Factor`] [`FactorOp`] [`Unary`] | [`Unary`]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Factor<'src> {
        Factor(Box<(Factor<'src>, FactorOp<'src>)>),
        Unary(Unary<'src>),
        _ => a "`*` or `/` operator or unary expression"
    }
}

define_enum_subset! {
    /// [`BinaryOpPunc`] ::= "%" | "&" | "*" | "+" | "/" | "<" | ">" | "^" | "|" | "!=" | "!&" | "!|" | "!^" |"**" | "<=" | "<<" | "==" | ">=" | ">>"
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum BinaryOpPunc : Punctuation {
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
    }
}

terminal_rule! {
    /// [`BinaryOp`] ::= [`BinaryOpPunc`]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BinaryOp<'src>(pub &'src str, pub BinaryOpPunc)
        := (lex, val: TokenValue::Punctuation(val))
            [ if let Ok(val) = BinaryOpPunc::try_from(val) ]
        => (Self(lex, val)) as a "binary operator";
}

simple_rule! {
    /// [`BinaryOperation`] ::= [`Expression`] [`BinaryOp`] [`Expression`]
    #[derive(Debug, Clone, PartialEq)]
    pub struct BinaryOperation<'src> {
        pub lhs: Box<Expression<'src>>,
        pub op: BinaryOp<'src>,
        pub rhs: Box<Expression<'src>>,
    }
}

simple_rule! {
    /// [`Expression`] ::= [`Literal`] | "(" [`Expression`] ")" | [`BinaryOperation`] | ; TODO
    #[derive(Debug, Clone, PartialEq)]
    pub enum Expression<'src> {
        Literal(Literal<'src>),
        Group(Parenthesized<'src, Box<Self>>),
        BinOp(BinaryOperation<'src>),
        // TODO: are there other expression structures?
        _ => an "expression"
    }
}

simple_rule! {
    /// [`RecBody`] ::= [`Identifier`] ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct RecBody<'src> {
        pub field: Identifier<'src>,
        // TODO: list
    }
}

simple_rule! {
    /// [`RecDef`] ::= "rec" [`Identifier`] "{" [`RecBody`] "}"
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct RecDef<'src> {
        pub rec_kw: RecKeyword<'src>,
        pub name: Identifier<'src>,
        pub body: Braced<'src, RecBody<'src>>,
    }
}

simple_rule! {
    /// [`SupDef`] ::= "sup" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SupDef<'src> {
        pub sup_kw: SupKeyword<'src>,
        // TODO
    }
}

simple_rule! {
    /// [`SubDef`] ::= "sub" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SubDef<'src> {
        pub sup_kw: SubKeyword<'src>,
        // TODO
    }
}

simple_rule! {
    /// [`CatDef`] ::= "cat" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CatDef<'src> {
        pub cat_kw: CatKeyword<'src>,
        // TODO
    }
}

simple_rule! {
    /// [`AltDef`] ::= "alt" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct AltDef<'src> {
        pub alt_kw: AltKeyword<'src>,
        // TODO
    }
}

simple_rule! {
    /// [`MacroDef`] ::= "def" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MacroDef<'src> {
        pub def_kw: DefKeyword<'src>,
        // TODO
    }
}

simple_rule! {
    /// [`ParamList1`] ::= "," | "," [`ParamList`]
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
    pub struct ParamList1<'src> {
        pub comma: CommaOp<'src>,
        pub param: Option<Box<ParamList<'src>>>,
    }
}

simple_rule! {
    /// [`ParamList`] ::= [`Identifier`] | [`Identifier`] [`ParamList1`]
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
    pub struct ParamList<'src> {
        /// ident
        pub param: Identifier<'src>,
        pub next: Option<ParamList1<'src>>,
    }
}

simple_rule! {
    /// [`FnBody`] ::= [`LetStatement`] ; TODO
    #[derive(Debug, Clone, PartialEq)]
    pub struct FnBody<'src> {
        statement: LetStatement<'src>,
        // TODO
    }
}

terminal_rule! {
    /// [`Identifier`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Identifier<'src>(pub &'src str) := (lex, val: TokenValue::Identifier | TokenValue::Callable) => (Self(lex)) as an "identifier";
}

simple_rule! {
    /// [`FnDef`] ::= "fn" [`Identifier`] "(" [`ParamList`] ")" "{" [`FnBody`] "}"
    #[derive(Debug, Clone, PartialEq)]
    pub struct FnDef<'src> {
        pub fn_kw: FnKeyword<'src>,
        pub name: Identifier<'src>,
        pub params: Parenthesized<'src, Option<ParamList<'src>>>,
        pub body: Braced<'src, FnBody<'src>>,
    }
}

simple_rule! {
    /// [`MemDef`] ::= "mem" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MemDef<'src> {
        pub mem_kw: MemKeyword<'src>,
        // TODO
    }
}

simple_rule! {
    /// [`Item`] ::= [`RecDef`] | [`SupDef`] | [`SubDef`] | [`CatDef`] | [`AltDef`] | [`MacroDef`] | [`FnDef`] | [`MemDef`]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Item<'src> {
        Rec(RecDef<'src>),
        Sup(SupDef<'src>),
        Sub(SubDef<'src>),
        Cat(CatDef<'src>),
        Alt(AltDef<'src>),
        Def(MacroDef<'src>),
        Fn(FnDef<'src>),
        Mem(MemDef<'src>),
        _ => a "`rec`, `sup`, `sub`, `cat`, `alt`, `def`, `fn`, or `mem` keyword"
    }
}

/// [`Syntax`] ::= [`Item`] | [`Item`] [`Syntax`]
#[derive(Debug, Clone, PartialEq)]
pub enum Syntax<'src> {
    Item(Item<'src>),
    Pair(Item<'src>, Box<Syntax<'src>>),
}

/// There is a distinction between "optional rest" vs "there should be nothing else".
/// Here we want there to be nothing remaining after.
impl<'src> Rule<'src> for Syntax<'src> {
    fn try_pull<'arr>(
        source: &'src str,
        tokens: &'arr [Token<'src>],
    ) -> Result<(Self, &'arr [Token<'src>]), ContextError<'src>> {
        let (item, tokens) = Item::try_pull(source, tokens)?;
        if tokens.is_empty() {
            Ok((Self::Item(item), tokens))
        } else {
            let (syntax, tokens) = <Box<Syntax>>::try_pull(source, tokens)?;
            Ok((Self::Pair(item, syntax), tokens))
        }
    }
}

pub fn grammarize<'src>(
    source: &'src str,
    tokens: &[Token<'src>],
) -> Result<Syntax<'src>, ContextError<'src>> {
    Syntax::try_pull(source, tokens).map(|(syn, _)| syn)
}

#[test]
fn test() {
    const SOURCE: &str = "fn main() { let x = 5; }";
    let tokens = crate::scanner::tokenize(SOURCE)
        // TODO: make these optional instead(?)
        .filter(|res| {
            !res.as_ref().is_ok_and(|token| {
                matches!(token.val, TokenValue::Comment | TokenValue::Whitespace)
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("lex error(s)");

    match grammarize(SOURCE, &tokens) {
        Ok(value) => println!("{value:#?}"),
        Err(e) => panic!("{e}"),
    }
}
