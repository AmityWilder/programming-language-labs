//! Context-free grammar

#![allow(clippy::missing_docs_in_private_items, reason = "under construction")]

use crate::{
    error::{ContextError, ErrorType, Expecting},
    scanner::{
        Bracket,
        token::{Keyword, Punctuation, Token, TokenType, TokenValue},
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
        $vis struct $Struct<$lt> {
            $(
                $(#[$fmeta])*
                $fvis $field: $Type
            ),*
        }

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
}

/// A rule that is a single token matching a specific pattern
macro_rules! terminal_rule {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Struct:ident<$lt:lifetime>($fvis:vis $Type:ty)
            := ($($field:ident$(: $pattern:pat)?),+ $(,)? $(if $($guard:tt)+)?) => ($result:expr)
            as $article:ident $desc:literal;
    ) => {
        $(#[$meta])*
        $vis struct $Struct<$lt>($fvis $Type);

        impl<'a> Rule<'a> for $Struct<'a> {
            fn try_pull<'b>(
                source: &'a str,
                tokens: &'b [Token<'a>],
            ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
                MatchRule::try_pull(
                    |token| match token {
                        Token { $($field$(: $pattern)?,)+ .. } $(if $($guard)+)? => Some($result),
                        _ => None,
                    },
                    tokens,
                )
                .map_err(|token| {
                    ContextError::missing_or_unexpected(token, source, Expecting::$article($desc))
                })
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

pub trait MatchRule<'a>: Sized {
    type Output;

    fn try_pull<'b>(
        self,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self::Output, &'b [Token<'a>]), Option<Token<'a>>>;
}

impl<'a, U, F> MatchRule<'a> for F
where
    F: FnOnce(Token<'a>) -> Option<U>,
{
    type Output = U;

    fn try_pull<'b>(
        self,
        mut tokens: &'b [Token<'a>],
    ) -> Result<(Self::Output, &'b [Token<'a>]), Option<Token<'a>>> {
        tokens
            .split_off_first()
            .ok_or(None)
            .and_then(|&token| self(token).map(|x| (x, tokens)).ok_or(Some(token)))
    }
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

terminal_rule! {
    /// `"let"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LetKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Let)) => (Self(src)) as a "`let` keyword";
}
terminal_rule! {
    /// `"="`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct AssignOp<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::Assign)) => (Self(src)) as an "`=` operator";
}
terminal_rule! {
    /// `";"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SemiColon<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::Semi)) => (Self(src)) as an "`;`";
}

simple_rule! {
    /// `<let-statement> ::= "let" <binding> "=" <expression>`
    #[derive(Debug, Clone, PartialEq)]
    pub struct LetStatement<'a> {
        pub let_kw: LetKeyword<'a>,
        pub binding: Binding<'a>,
        pub assign_kw: AssignOp<'a>,
        pub expression: Expression<'a>,
        pub semi: SemiColon<'a>,
    }
}

simple_rule! {
    /// `<binding> ::= <identifier>`
    // TODO: this can be way cooler
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Binding<'a> {
        name: Ident<'a>,
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

terminal_rule! {
    /// `"("`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct OpenParen<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::LParen)) => (Self(src)) as an "open parenthesis `(`";
}
terminal_rule! {
    /// `")"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CloseParen<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::RParen)) => (Self(src)) as a "close parenthesis `)`";
}

/// `<parenthesized> ::= "(" INNER ")"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Parenthesized<'a, T> {
    pub open: OpenParen<'a>,
    pub inner: T,
    pub close: CloseParen<'a>,
}

/// Not a simple rule, because brackets have special errors
impl<'a, T: Rule<'a>> Rule<'a> for Parenthesized<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = OpenParen::try_pull(source, tokens)?;

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

terminal_rule! {
    /// `"["`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct OpenBrack<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::LBrack)) => (Self(src)) as an "open bracket `[`";
}
terminal_rule! {
    /// `"]"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CloseBrack<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::RBrack)) => (Self(src)) as a "close bracket `]`";
}

/// `<bracketed> ::= "[" INNER "]"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bracketed<'a, T> {
    pub open: OpenBrack<'a>,
    pub inner: T,
    pub close: CloseBrack<'a>,
}

/// Not a simple rule, because brackets have special errors
impl<'a, T: Rule<'a>> Rule<'a> for Bracketed<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = OpenBrack::try_pull(source, tokens)?;

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

terminal_rule! {
    /// `"{"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct OpenBrace<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::LBrace)) => (Self(src)) as an "open brace `{`";
}
terminal_rule! {
    /// `"}"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CloseBrace<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::RBrace)) => (Self(src)) as a "close brace `}`";
}

/// `<braced> ::= "{" INNER "}"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Braced<'a, T> {
    /// `{`
    pub open: OpenBrace<'a>,
    pub inner: T,
    /// `}`
    pub close: CloseBrace<'a>,
}

/// Not a simple rule, because brackets have special errors
impl<'a, T: Rule<'a>> Rule<'a> for Braced<'a, T> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (open, tokens) = OpenBrace::try_pull(source, tokens)?;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Literal<'a> {
    pub token: (&'a str, TokenValue<'a>),
}

impl<'a> Rule<'a> for Literal<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let (token, tokens) = MatchRule::try_pull(
            token_pattern!(src, val, ty: TokenType::BoolLiteral | TokenType::NumberLiteral | TokenType::CharLiteral | TokenType::StringLiteral => (src, val)),
            tokens,
        )
        .map_err(|token| {
            ContextError::missing_or_unexpected(token, source, Expecting::a("literal"))
        })?;

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
            tokens,
        )
        .map_err(|token| {
            ContextError::missing_or_unexpected(token, source, Expecting::a("binary operator"))
        })?;
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
    Group(Parenthesized<'a, Box<Self>>),
    BinOp(BinaryOperation<'a>),
}

impl<'a> Rule<'a> for Expression<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        if let Ok((value, tokens)) = Parenthesized::try_pull(source, tokens) {
            Ok((Self::Group(value), tokens))
        } else if let Ok((value, tokens)) = Literal::try_pull(source, tokens) {
            Ok((Self::Literal(value), tokens))
        } else if let Ok((value, tokens)) = BinaryOperation::try_pull(source, tokens) {
            Ok((Self::BinOp(value), tokens))
        }
        // TODO: are there other expression structures?
        else {
            Err(ContextError::missing_or_unexpected(
                tokens.first().copied(),
                source,
                Expecting::an("expression"),
            ))
        }
    }
}

terminal_rule! {
    /// `"struct"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StructKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Struct)) => (Self(src)) as a "`struct` keyword";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StructBody {}

impl<'a> Rule<'a> for StructBody {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        todo!("struct body")
    }
}

simple_rule! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StructDef<'a> {
        pub struct_kw: StructKeyword<'a>,
        pub name: Ident<'a>,
        pub body: Braced<'a, StructBody>,
    }
}

terminal_rule! {
    /// `"union"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct UnionKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Union)) => (Self(src)) as a "`union` keyword";
}

simple_rule! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct UnionDef<'a> {
        pub union_kw: UnionKeyword<'a>,
        // TODO
    }
}

terminal_rule! {
    /// `"enum"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct EnumKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Enum)) => (Self(src)) as an "`enum` keyword";
}

simple_rule! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct EnumDef<'a> {
        pub enum_kw: EnumKeyword<'a>,
        // TODO
    }
}

terminal_rule! {
    /// `"type"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct TypeKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Type)) => (Self(src)) as a "`type` keyword";
}

simple_rule! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct TypeDef<'a> {
        pub type_kw: TypeKeyword<'a>,
        // TODO
    }
}

terminal_rule! {
    /// `"def"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct DefKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Def)) => (Self(src)) as a "`def` keyword";
}

simple_rule! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MacroDef<'a> {
        pub def_kw: DefKeyword<'a>,
        // TODO
    }
}

terminal_rule! {
    /// `","`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CommaPunc<'a>(pub &'a str) := (src, val: TokenValue::Punctuation(Punctuation::Comma)) => (Self(src)) as a "comma (`,`)";
}

simple_rule! {
    /// `<param-list1> ::= "," | "," <param-list>`
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
    pub struct ParamList1<'a> {
        pub comma: CommaPunc<'a>,
        pub param: Option<Box<ParamList<'a>>>,
    }
}

simple_rule! {
    /// `<param-list> ::= <ident> | <ident> <param-list1>`
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
    pub struct ParamList<'a> {
        /// ident
        pub param: Ident<'a>,
        pub next: Option<ParamList1<'a>>,
    }
}

simple_rule! {
    #[derive(Debug, Clone, PartialEq)]
    pub struct FnBody<'a> {
        statement: LetStatement<'a>, // TODO
    }
}

terminal_rule! {
    /// `"fn"`
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct FnKeyword<'a>(pub &'a str) := (src, val: TokenValue::Keyword(Keyword::Fn)) => (Self(src)) as a "`fn` keyword";
}

terminal_rule! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Ident<'a>(pub &'a str) := (val: TokenValue::Direct(src) /* TODO: are there other uses for Direct? */) => (Self(src)) as an "identifier";
}

simple_rule! {
    /// `<fn-def> ::= "fn" <ident> "(" <param-list> ")" "{" <fn-body> "}"`
    #[derive(Debug, Clone, PartialEq)]
    pub struct FnDef<'a> {
        /// `fn`
        pub fn_kw: FnKeyword<'a>,
        /// ident
        pub name: Ident<'a>,
        pub params: Parenthesized<'a, Option<ParamList<'a>>>,
        pub body: Braced<'a, FnBody<'a>>,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item<'a> {
    StructDef(StructDef<'a>),
    UnionDef(UnionDef<'a>),
    EnumDef(EnumDef<'a>),
    TypeDef(TypeDef<'a>),
    MacroDef(MacroDef<'a>),
    FnDef(FnDef<'a>),
}

impl<'a> Rule<'a> for Item<'a> {
    fn try_pull<'b>(
        source: &'a str,
        tokens: &'b [Token<'a>],
    ) -> Result<(Self, &'b [Token<'a>]), ContextError<'a>> {
        let unexpected = match tokens.first().copied() {
            Some(
                tkn @ Token {
                    val: TokenValue::Keyword(kw),
                    ..
                },
            ) => match kw {
                Keyword::Struct => {
                    return StructDef::try_pull(source, tokens).map(map_pull(Self::StructDef));
                }
                Keyword::Union => {
                    return UnionDef::try_pull(source, tokens).map(map_pull(Self::UnionDef));
                }
                Keyword::Enum => {
                    return EnumDef::try_pull(source, tokens).map(map_pull(Self::EnumDef));
                }
                Keyword::Type => {
                    return TypeDef::try_pull(source, tokens).map(map_pull(Self::TypeDef));
                }
                Keyword::Def => {
                    return MacroDef::try_pull(source, tokens).map(map_pull(Self::MacroDef));
                }
                Keyword::Fn => {
                    return FnDef::try_pull(source, tokens).map(map_pull(Self::FnDef));
                }

                _ => Some(tkn),
            },

            _ => None,
        };

        Err(ContextError::missing_or_unexpected(
            unexpected,
            source,
            Expecting::a("`struct`, `union`, `enum`, `type`, `def`, or `fn` keyword"),
        ))
    }
}

/// ```not_code
/// <syntax> ::= <item> | <item> <syntax>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Syntax<'a> {
    Item(Item<'a>),
    Pair(Item<'a>, Box<Syntax<'a>>),
}

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
            !res.as_ref()
                .is_ok_and(|token| token.val == TokenValue::Ignore)
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("lex error(s)");

    match grammarize(SOURCE, &tokens) {
        Ok(value) => println!("{value:#?}"),
        Err(e) => panic!("{e}"),
    }
}
