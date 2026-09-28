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

        impl<'a> Rule<'a> for $Struct<'a> {
            fn try_pull<'b>(
                source: &'a str,
                tokens: &'b [Token<'a>],
            ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
                $(let ($field, tokens) = Rule::try_pull(source, tokens)?;)*
                Ok((Self { $($field),* }, tokens))
            }
        }
    };

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

        impl<'a> Rule<'a> for $Enum<'a> {
            fn try_pull<'b>(
                source: &'a str,
                tokens: &'b [Token<'a>],
            ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
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

        impl<'a> $crate::grammar::Rule<'a> for $Struct<'a> {
            fn try_pull<'b>(
                source: &'a str,
                tokens: &'b [Token<'a>],
            ) -> Result<(Self, &'b [Token<'a>]), $crate::error::ContextError<'a>> {
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

fn map_pull<'a, 'b, T, U, F>(f: F) -> impl FnOnce((T, &'b [Token<'a>])) -> (U, &'b [Token<'a>])
where
    F: FnOnce(T) -> U,
{
    move |(x, tokens)| (f(x), tokens)
}

pub fn try_pull_matching<'a: 'b, 'b, T, F>(
    f: F,
    mut tokens: &'b [Token<'a>],
) -> Result<(T, &'b [Token<'a>]), Option<Token<'a>>>
where
    F: FnOnce(Token<'a>) -> Option<T>,
{
    tokens
        .split_off_first()
        .ok_or(None)
        .and_then(|&token| f(token).map(|x| (x, tokens)).ok_or(Some(token)))
}

pub trait Rule<'a>: Sized {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>>;
}

impl<'a, T: Rule<'a>> Rule<'a> for Box<T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        T::try_pull(source, tokens).map(map_pull(Box::new))
    }
}

impl<'a, T: Rule<'a>> Rule<'a> for Option<T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        Ok(T::try_pull(source, tokens)
            .map(map_pull(Some))
            .unwrap_or((None, tokens)))
    }
}

simple_rule! {
    /// [`LetStatement`] ::= "let" [`Binding`] "=" [`Expression`] ";"
    #[derive(Debug, Clone, PartialEq)]
    pub struct LetStatement<'a> {
        pub let_kw: LetKeyword<'a>,
        pub binding: Binding<'a>,
        pub assign_kw: AssignOp<'a>,
        pub expression: Expression<'a>,
        pub semi: SemiOp<'a>,
    }
}

simple_rule! {
    /// [`Binding`] ::= [`Identifier`]
    // TODO: this can be way cooler
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Binding<'a> {
        name: Identifier<'a>,
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
pub struct Parenthesized<'a, T> {
    pub open: LParenOp<'a>,
    pub inner: T,
    pub close: RParenOp<'a>,
}

/// Not a simple rule, because brackets have special errors
impl<'a, T: Rule<'a>> Rule<'a> for Parenthesized<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
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
pub struct Bracketed<'a, T> {
    pub open: LBrackOp<'a>,
    pub inner: T,
    pub close: RBrackOp<'a>,
}

/// Not a simple rule, because brackets have special errors
impl<'a, T: Rule<'a>> Rule<'a> for Bracketed<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
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
pub struct Braced<'a, T> {
    /// `{`
    pub open: LBraceOp<'a>,
    pub inner: T,
    /// `}`
    pub close: RBraceOp<'a>,
}

/// Not a simple rule, because brackets have special errors
impl<'a, T: Rule<'a>> Rule<'a> for Braced<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
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
    pub struct LiteralBool<'a>(pub &'a str, pub bool) := (lex, val: TokenValue::BoolLiteral(val)) => (Self(lex, val)) as a "boolean literal";
}
terminal_rule! {
    /// [`LiteralUInt`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralUInt<'a>(pub &'a str, pub usize) := (lex, val: TokenValue::UIntLiteral(val)) => (Self(lex, val)) as a "uint literal";
}
terminal_rule! {
    /// [`LiteralSInt`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralSInt<'a>(pub &'a str, pub isize) := (lex, val: TokenValue::SIntLiteral(val)) => (Self(lex, val)) as an "sint literal";
}
terminal_rule! {
    /// [`LiteralFrac`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub struct LiteralFrac<'a>(pub &'a str, pub f64) := (lex, val: TokenValue::FltLiteral(val)) => (Self(lex, val)) as a "frac literal";
}
terminal_rule! {
    /// [`LiteralChar`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralChar<'a>(pub &'a str, pub CharLiteral) := (lex, val: TokenValue::CharLiteral(val)) => (Self(lex, val)) as a "char literal";
}
terminal_rule! {
    /// [`LiteralStr`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LiteralStr<'a>(pub &'a str, pub StrLiteral<'a>) := (lex, val: TokenValue::StringLiteral(val)) => (Self(lex, val)) as a "string literal";
}

simple_rule! {
    /// [`IntLiteral`] ::= [`LiteralUInt`] | [`LiteralSInt`]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum IntLiteral<'a> {
        UInt(LiteralUInt<'a>),
        SInt(LiteralSInt<'a>),
        _ => an "integer literal"
    }
}

simple_rule! {
    /// [`NumLiteral`] ::= [`IntLiteral`] | [`LiteralFrac`]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum NumLiteral<'a> {
        Int(IntLiteral<'a>),
        Flt(LiteralFrac<'a>),
        _ => a "number literal"
    }
}

simple_rule! {
    /// [`Literal`] ::= [`LiteralBool`] | [`NumLiteral`] | [`LiteralChar`] | [`LiteralStr`]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Literal<'a> {
        Bool(LiteralBool<'a>),
        Num(NumLiteral<'a>),
        Char(LiteralChar<'a>),
        Str(LiteralStr<'a>),
        _ => a "literal"
    }
}

define_enum_subset! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum BinaryOpPunc : Punctuation {
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

terminal_rule! {
    /// [`BinaryOp`] ::= [`BinaryOpPunc`]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BinaryOp<'a>(pub &'a str, pub BinaryOpPunc)
        := (lex, val: TokenValue::Punctuation(val))
            [ if let Ok(val) = BinaryOpPunc::try_from(val) ]
        => (Self(lex, val)) as a "binary operator";
}

simple_rule! {
    /// [`BinaryOperation`] ::= [`Expression`] [`BinaryOp`] [`Expression`]
    #[derive(Debug, Clone, PartialEq)]
    pub struct BinaryOperation<'a> {
        pub lhs: Box<Expression<'a>>,
        pub op: BinaryOp<'a>,
        pub rhs: Box<Expression<'a>>,
    }
}

simple_rule! {
    /// [`Expression`] ::= [`Literal`] | "(" [`Expression`] ")" | [`BinaryOperation`] | ; TODO
    #[derive(Debug, Clone, PartialEq)]
    pub enum Expression<'a> {
        Literal(Literal<'a>),
        Group(Parenthesized<'a, Box<Self>>),
        BinOp(BinaryOperation<'a>),
        // TODO: are there other expression structures?
        _ => an "expression"
    }
}

simple_rule! {
    /// [`RecBody`] ::= [`Identifier`] ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct RecBody<'a> {
        pub field: Identifier<'a>,
        // TODO: list
    }
}

simple_rule! {
    /// [`RecDef`] ::= "rec" [`Identifier`] "{" [`RecBody`] "}"
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct RecDef<'a> {
        pub rec_kw: RecKeyword<'a>,
        pub name: Identifier<'a>,
        pub body: Braced<'a, RecBody<'a>>,
    }
}

simple_rule! {
    /// [`SupDef`] ::= "sup" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SupDef<'a> {
        pub sup_kw: SupKeyword<'a>,
        // TODO
    }
}

simple_rule! {
    /// [`SubDef`] ::= "sub" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SubDef<'a> {
        pub sup_kw: SubKeyword<'a>,
        // TODO
    }
}

simple_rule! {
    /// [`CatDef`] ::= "cat" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CatDef<'a> {
        pub cat_kw: CatKeyword<'a>,
        // TODO
    }
}

simple_rule! {
    /// [`AltDef`] ::= "alt" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct AltDef<'a> {
        pub alt_kw: AltKeyword<'a>,
        // TODO
    }
}

simple_rule! {
    /// [`MacroDef`] ::= "def" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MacroDef<'a> {
        pub def_kw: DefKeyword<'a>,
        // TODO
    }
}

simple_rule! {
    /// [`ParamList1`] ::= "," | "," [`ParamList`]
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
    pub struct ParamList1<'a> {
        pub comma: CommaOp<'a>,
        pub param: Option<Box<ParamList<'a>>>,
    }
}

simple_rule! {
    /// [`ParamList`] ::= [`Identifier`] | [`Identifier`] [`ParamList1`]
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
    pub struct ParamList<'a> {
        /// ident
        pub param: Identifier<'a>,
        pub next: Option<ParamList1<'a>>,
    }
}

simple_rule! {
    /// [`FnBody`] ::= [`LetStatement`] ; TODO
    #[derive(Debug, Clone, PartialEq)]
    pub struct FnBody<'a> {
        statement: LetStatement<'a>,
        // TODO
    }
}

terminal_rule! {
    /// [`Identifier`] ::= ; tokenizer-defined
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Identifier<'a>(pub &'a str) := (lex, val: TokenValue::Identifier | TokenValue::Callable) => (Self(lex)) as an "identifier";
}

simple_rule! {
    /// [`FnDef`] ::= "fn" [`Identifier`] "(" [`ParamList`] ")" "{" [`FnBody`] "}"
    #[derive(Debug, Clone, PartialEq)]
    pub struct FnDef<'a> {
        pub fn_kw: FnKeyword<'a>,
        pub name: Identifier<'a>,
        pub params: Parenthesized<'a, Option<ParamList<'a>>>,
        pub body: Braced<'a, FnBody<'a>>,
    }
}

simple_rule! {
    /// [`MemDef`] ::= "mem" ; TODO
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MemDef<'a> {
        pub mem_kw: MemKeyword<'a>,
        // TODO
    }
}

simple_rule! {
    /// [`Item`] ::= [`RecDef`] | [`SupDef`] | [`SubDef`] | [`CatDef`] | [`AltDef`] | [`MacroDef`] | [`FnDef`] | [`MemDef`]
    #[derive(Debug, Clone, PartialEq)]
    pub enum Item<'a> {
        Rec(RecDef<'a>),
        Sup(SupDef<'a>),
        Sub(SubDef<'a>),
        Cat(CatDef<'a>),
        Alt(AltDef<'a>),
        Def(MacroDef<'a>),
        Fn(FnDef<'a>),
        Mem(MemDef<'a>),
        _ => a "`rec`, `sup`, `sub`, `cat`, `alt`, `def`, `fn`, or `mem` keyword"
    }
}

/// [`Syntax`] ::= [`Item`] | [`Item`] [`Syntax`]
#[derive(Debug, Clone, PartialEq)]
pub enum Syntax<'a> {
    Item(Item<'a>),
    Pair(Item<'a>, Box<Syntax<'a>>),
}

/// There is a distinction between "optional rest" vs "there should be nothing else".
/// Here we want there to be nothing remaining after.
impl<'a> Rule<'a> for Syntax<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (item, tokens) = Item::try_pull(source, tokens)?;
        if tokens.is_empty() {
            Ok((Self::Item(item), tokens))
        } else {
            let (syntax, tokens) = <Box<Syntax>>::try_pull(source, tokens)?;
            Ok((Self::Pair(item, syntax), tokens))
        }
    }
}

pub fn grammarize<'a>(
    source: &'a str,
    tokens: &[Token<'a>],
) -> Result<Syntax<'a>, ContextError<'a>> {
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
