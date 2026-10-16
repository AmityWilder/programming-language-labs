use crate::{
    analyzer::static_env::StaticEnvironment,
    error::{ContextError, OpSide},
    exec::{
        env::{Environment, Pointer, Primitive, UntypedPointer},
        error::Failure,
        eval::ValueType,
    },
    grammar::ast,
    scanner::token::{
        ExpansionData, Token,
        punc::Punctuation,
        value::{CharLiteral, Keyword, LexValue, TextLiteral, TokenEq},
    },
    spec_ops::{BitRotate, BitShift},
    types::{fail, frac, nevr, none, sint, text, uint},
};
use std::marker::PhantomData;

pub mod static_env;

// TODO: all of this is very yucky. it desperately needs to be completely refactored.

/// Valueless token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lexeme<'src> {
    pub lex: &'src str,
    pub mac: Option<ExpansionData<'src>>,
}

impl<'src> From<Token<'src>> for Lexeme<'src> {
    fn from(Token { lex, val: _, mac }: Token<'src>) -> Self {
        Self { lex, mac }
    }
}

/// Post-analysis evaluation
pub trait Evaluate<'src, T> {
    fn eval(&self, env: &Environment<'src>) -> Result<T, Failure>;
}

pub trait LangType {
    fn get_type(&self) -> ValueType;

    fn szof(&self) -> usize {
        std::mem::size_of_val(self)
    }

    fn align(&self) -> usize {
        std::mem::align_of_val(self)
    }
}

impl LangType for none {
    fn get_type(&self) -> ValueType {
        ValueType::None
    }
}
impl LangType for nevr {
    fn get_type(&self) -> ValueType {
        ValueType::Nevr
    }
}
impl LangType for fail {
    fn get_type(&self) -> ValueType {
        ValueType::Fail
    }
}
impl LangType for bool {
    fn get_type(&self) -> ValueType {
        ValueType::Bool
    }
}
impl LangType for uint {
    fn get_type(&self) -> ValueType {
        ValueType::UInt
    }
}
impl LangType for sint {
    fn get_type(&self) -> ValueType {
        ValueType::SInt
    }
}
impl LangType for frac {
    fn get_type(&self) -> ValueType {
        ValueType::Frac
    }
}
impl LangType for char {
    fn get_type(&self) -> ValueType {
        ValueType::Char
    }
}
impl LangType for text {
    fn get_type(&self) -> ValueType {
        ValueType::Text
    }
}
impl LangType for str {
    fn get_type(&self) -> ValueType {
        ValueType::Text
    }
}
impl LangType for std::borrow::Cow<'_, str> {
    fn get_type(&self) -> ValueType {
        ValueType::Text
    }
}
impl LangType for UntypedPointer {
    fn get_type(&self) -> ValueType {
        ValueType::Ptr
    }
}
impl<T: ?Sized> LangType for Pointer<T> {
    fn get_type(&self) -> ValueType {
        ValueType::Ptr
    }
}

pub trait Output: Sized + LangType {
    /// Structure for operations that **result** in `Self`
    type Ops<'src>: 'src
        + Operators<'src, Output = Self>
        + Evaluate<'src, Self>
        + std::fmt::Debug
        + Clone;
}

pub trait Operators<'src>: Sized {
    type Output: Output;

    fn new(
        op: TokenEq,
        lhs: Option<AnyExpr<'src>>,
        rhs: Option<AnyExpr<'src>>,
    ) -> Result<Self, (TokenEq, Option<AnyExpr<'src>>, Option<AnyExpr<'src>>)>;

    fn new_expr(
        args: (
            TokenEq,
            Option<AnyExpr<'src>>,
            Option<AnyExpr<'src>>,
            Lexeme<'src>,
        ),
    ) -> Result<
        Expr<'src, Self::Output>,
        (
            TokenEq,
            Option<AnyExpr<'src>>,
            Option<AnyExpr<'src>>,
            Lexeme<'src>,
        ),
    >;

    fn new_any_expr(
        args: (
            TokenEq,
            Option<AnyExpr<'src>>,
            Option<AnyExpr<'src>>,
            Lexeme<'src>,
        ),
    ) -> Result<
        AnyExpr<'src>,
        (
            TokenEq,
            Option<AnyExpr<'src>>,
            Option<AnyExpr<'src>>,
            Lexeme<'src>,
        ),
    >;
}

/// Outputs the provided tokens wrapped in a [`Some`], or a [`None`] if no tokens are present.
///
/// Intended for use with optional (`$()?` or `$()*`) patterns.
macro_rules! optional {
    () => {
        None
    };

    ($($tokens:tt)+) => {
        Some($($tokens)+)
    };
}

macro_rules! impl_output {
    (
        $(#[$ops_meta:meta])*
        for $Type:ident as $vis:vis $Ops:ident<$lt:lifetime> {$(
            $(#[doc = $op_doc:expr])*
            #[op($OpKind:ident::$OpKey:ident)]
            #[def($(@$env:ident,)? $def:expr)]
            $Operation:ident {
                $($lhs:ident: $Lhs:ident)?,
                $($rhs:ident: $Rhs:ident)?
            }
        ),* $(,)?}
    ) => {
        impl Output for $Type {
            type Ops<$lt> = $Ops<$lt>;
        }

        $(#[$ops_meta])*
        $vis enum $Ops<$lt> {$(
            $(#[doc = $op_doc])*
            $Operation {
                $($lhs: Expr<$lt, $Lhs>,)?
                $($rhs: Expr<$lt, $Rhs>,)?
            }
        ),*}

        impl<$lt> Evaluate<$lt, $Type> for $Ops<$lt> {
            fn eval(&self, env: &Environment<$lt>) -> Result<$Type, Failure> {
                match self {
                    $(Self::$Operation { $($lhs,)? $($rhs)? } => {
                        $(let $lhs = $lhs.eval(env)?;)?
                        $(let $rhs = $rhs.eval(env)?;)?
                        $(let $env = env;)?
                        Ok($def)
                    }),*
                }
            }
        }

        impl<$lt> Operators<$lt> for $Ops<$lt> {
            type Output = $Type;

            fn new(
                op: TokenEq,
                lhs: Option<AnyExpr<$lt>>,
                rhs: Option<AnyExpr<$lt>>,
            ) -> Result<Self, (TokenEq, Option<AnyExpr<$lt>>, Option<AnyExpr<$lt>>)> {
                match (op, lhs, rhs) {
                    $((
                        TokenEq::$OpKind($OpKind::$OpKey),
                        optional!($(AnyExpr::$Lhs($lhs))?),
                        optional!($(AnyExpr::$Rhs($rhs))?),
                    ) => Ok(Self::$Operation { $($lhs,)? $($rhs)? }),)*
                    args => Err(args),
                }
            }

            fn new_expr((op, lhs, rhs, op_lex): (
                TokenEq,
                Option<AnyExpr<$lt>>,
                Option<AnyExpr<$lt>>,
                Lexeme<$lt>,
            )) -> Result<Expr<$lt, Self::Output>, (TokenEq, Option<AnyExpr<$lt>>, Option<AnyExpr<$lt>>, Lexeme<$lt>)> {
                Self::new(op, lhs, rhs).map(|op| {
                    Expr::Op {
                        op: Box::new(op),
                        lex: op_lex,
                    }
                })
                .map_err(|(op, lhs, rhs)| (op, lhs, rhs, op_lex))
            }

            fn new_any_expr(args: (
                TokenEq,
                Option<AnyExpr<$lt>>,
                Option<AnyExpr<$lt>>,
                Lexeme<$lt>,
            )) -> Result<AnyExpr<$lt>, (TokenEq, Option<AnyExpr<$lt>>, Option<AnyExpr<$lt>>, Lexeme<$lt>)> {
                Self::new_expr(args).map(AnyExpr::$Type)
            }
        }
    };
}

impl_output! {
    #[derive(Debug, Clone)]
    for bool as pub BoolOps<'src> {
        // binary

        #[op(Punctuation::And)] #[def(lhs & rhs)] And { lhs: bool, rhs: bool },
        #[op(Punctuation::Or )] #[def(lhs | rhs)] Or  { lhs: bool, rhs: bool },
        #[op(Punctuation::Xor)] #[def(lhs ^ rhs)] Xor { lhs: bool, rhs: bool },

        #[op(Punctuation::Nand)] #[def(!(lhs & rhs))] Nand { lhs: bool, rhs: bool },
        #[op(Punctuation::Nor)]  #[def(!(lhs | rhs))] Nor  { lhs: bool, rhs: bool },
        #[op(Punctuation::Xnor)] #[def(!(lhs ^ rhs))] Xnor { lhs: bool, rhs: bool },

        // logical

        #[op(Punctuation::Eq)] #[def(lhs == rhs)] LEq { lhs: bool, rhs: bool },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] LNe { lhs: bool, rhs: bool },
        #[op(Punctuation::Eq)] #[def(lhs == rhs)] UEq { lhs: uint, rhs: uint },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] UNe { lhs: uint, rhs: uint },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] ULt { lhs: uint, rhs: uint },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] UGt { lhs: uint, rhs: uint },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] ULe { lhs: uint, rhs: uint },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] UGe { lhs: uint, rhs: uint },

        #[op(Punctuation::Eq)] #[def(lhs == rhs)] SEq { lhs: sint, rhs: sint },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] SNe { lhs: sint, rhs: sint },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] SLt { lhs: sint, rhs: sint },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] SGt { lhs: sint, rhs: sint },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] SLe { lhs: sint, rhs: sint },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] SGe { lhs: sint, rhs: sint },

        #[op(Punctuation::Eq)] #[def(lhs == rhs)] FEq { lhs: frac, rhs: frac },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] FNe { lhs: frac, rhs: frac },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] FLt { lhs: frac, rhs: frac },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] FGt { lhs: frac, rhs: frac },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] FLe { lhs: frac, rhs: frac },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] FGe { lhs: frac, rhs: frac },

        #[op(Punctuation::Eq)] #[def(lhs == rhs)] CEq { lhs: char, rhs: char },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] CNe { lhs: char, rhs: char },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] CLt { lhs: char, rhs: char },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] CGt { lhs: char, rhs: char },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] CLe { lhs: char, rhs: char },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] CGe { lhs: char, rhs: char },

        // TODO: text

        // unary

        #[op(Punctuation::Not)] #[def(!rhs)] Not { , rhs: bool },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for uint as pub UIntOps<'src> {
        // binary

        #[op(Punctuation::And)] #[def(lhs & rhs)] And { lhs: uint, rhs: uint },
        #[op(Punctuation::Or)] #[def(lhs | rhs)] Or { lhs: uint, rhs: uint },
        #[op(Punctuation::Xor)] #[def(lhs ^ rhs)] Xor { lhs: uint, rhs: uint },

        #[op(Punctuation::Nand)] #[def(!(lhs & rhs))] Nand { lhs: uint, rhs: uint },
        #[op(Punctuation::Nor)] #[def(!(lhs | rhs))] Nor { lhs: uint, rhs: uint },
        #[op(Punctuation::Xnor)] #[def(!(lhs ^ rhs))] Xnor { lhs: uint, rhs: uint },

        #[op(Punctuation::Add)]
        #[def(lhs.saturating_add(rhs))]
        Add { lhs: uint, rhs: uint },

        #[op(Punctuation::SubNeg)]
        #[def(lhs.saturating_sub(rhs))]
        Sub { lhs: uint, rhs: uint },

        #[op(Punctuation::Mul)]
        #[def(lhs.saturating_mul(rhs))]
        Mul { lhs: uint, rhs: uint },

        #[op(Punctuation::Div)]
        #[def(lhs.checked_div(rhs).unwrap_or(uint::MAX))]
        Div { lhs: uint, rhs: uint },

        #[op(Punctuation::Rem)]
        #[def(lhs.checked_rem(rhs).unwrap_or(0))]
        Rem { lhs: uint, rhs: uint },

        #[op(Punctuation::Pow)]
        #[def(rhs.try_into().map_or(uint::MAX, |rhs| lhs.pow(rhs)))]
        Pow { lhs: uint, rhs: uint },

        #[op(Punctuation::Shl)]  #[def(lhs.bshl(rhs))] Shl { lhs: uint, rhs: uint },
        #[op(Punctuation::Shr)]  #[def(lhs.bshr(rhs))] Shr { lhs: uint, rhs: uint },
        #[op(Punctuation::Rotl)] #[def(lhs.rotl(rhs))] Rotl { lhs: uint, rhs: uint },
        #[op(Punctuation::Rotr)] #[def(lhs.rotr(rhs))] Rotr { lhs: uint, rhs: uint },

        #[op(Punctuation::Shl)]  #[def(lhs.bshl(rhs))] SShl { lhs: uint, rhs: sint },
        #[op(Punctuation::Shr)]  #[def(lhs.bshr(rhs))] SShr { lhs: uint, rhs: sint },
        #[op(Punctuation::Rotl)] #[def(lhs.rotl(rhs))] SRotl { lhs: uint, rhs: sint },
        #[op(Punctuation::Rotr)] #[def(lhs.rotr(rhs))] SRotr { lhs: uint, rhs: sint },

        // unary

        #[op(Punctuation::Not)] #[def(!rhs)] Not { , rhs: uint },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for sint as pub SIntOps<'src> {
        // binary

        #[op(Punctuation::And)] #[def(lhs & rhs)] And { lhs: sint, rhs: sint },
        #[op(Punctuation::Or)]  #[def(lhs | rhs)] Or { lhs: sint, rhs: sint },
        #[op(Punctuation::Xor)] #[def(lhs ^ rhs)] Xor { lhs: sint, rhs: sint },

        #[op(Punctuation::Nand)] #[def(!(lhs & rhs))] Nand { lhs: sint, rhs: sint },
        #[op(Punctuation::Nor)]  #[def(!(lhs | rhs))] Nor { lhs: sint, rhs: sint },
        #[op(Punctuation::Xnor)] #[def(!(lhs ^ rhs))] Xnor { lhs: sint, rhs: sint },

        #[op(Punctuation::Add)]
        #[def(lhs.saturating_add(rhs))]
        Add { lhs: sint, rhs: sint },

        #[op(Punctuation::SubNeg)]
        #[def(lhs.saturating_sub(rhs))]
        Sub { lhs: sint, rhs: sint },

        #[op(Punctuation::Mul)]
        #[def(lhs.saturating_mul(rhs))]
        Mul { lhs: sint, rhs: sint },

        #[op(Punctuation::Div)]
        #[def(lhs.checked_div(rhs).unwrap_or(sint::MAX))]
        Div { lhs: sint, rhs: sint },

        #[op(Punctuation::Rem)]
        #[def(lhs.checked_rem(rhs).unwrap_or(0))]
        Rem { lhs: sint, rhs: sint },

        #[op(Punctuation::Pow)]
        #[def(rhs.try_into().map_or(sint::MAX, |rhs| lhs.pow(rhs)))]
        Pow { lhs: sint, rhs: sint },

        #[op(Punctuation::Shl)]  #[def(lhs.bshl(rhs))] Shl { lhs: sint, rhs: uint },
        #[op(Punctuation::Shr)]  #[def(lhs.bshr(rhs))] Shr { lhs: sint, rhs: uint },
        #[op(Punctuation::Rotl)] #[def(lhs.rotl(rhs))] Rotl { lhs: sint, rhs: uint },
        #[op(Punctuation::Rotr)] #[def(lhs.rotr(rhs))] Rotr { lhs: sint, rhs: uint },

        #[op(Punctuation::Shl)]  #[def(lhs.bshl(rhs))] SShl { lhs: sint, rhs: sint },
        #[op(Punctuation::Shr)]  #[def(lhs.bshr(rhs))] SShr { lhs: sint, rhs: sint },
        #[op(Punctuation::Rotl)] #[def(lhs.rotl(rhs))] SRotl { lhs: sint, rhs: sint },
        #[op(Punctuation::Rotr)] #[def(lhs.rotr(rhs))] SRotr { lhs: sint, rhs: sint },

        // unary

        #[op(Punctuation::Not)] #[def(!rhs)] Not { , rhs: sint },

        #[op(Punctuation::SubNeg)] #[def(rhs.saturating_neg())] Neg { , rhs: sint }, // TBD: is saturating neg really what we want in this case?...
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for frac as pub FracOps<'src> {
        // binary

        #[op(Punctuation::Add)]    #[def(lhs + rhs)]     Add { lhs: frac, rhs: frac },
        #[op(Punctuation::SubNeg)] #[def(lhs - rhs)]     Sub { lhs: frac, rhs: frac },
        #[op(Punctuation::Mul)]    #[def(lhs * rhs)]     Mul { lhs: frac, rhs: frac },
        #[op(Punctuation::Div)]    #[def(lhs / rhs)]     Div { lhs: frac, rhs: frac },
        #[op(Punctuation::Rem)]    #[def(lhs % rhs)]     Rem { lhs: frac, rhs: frac },
        #[op(Punctuation::Pow)]    #[def(lhs.powf(rhs))] Pow { lhs: frac, rhs: frac },

        // unary

        #[op(Punctuation::SubNeg)] #[def(-rhs)] Neg { , rhs: frac },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for char as pub CharOps<'src> {
        // binary

        #[op(Punctuation::Add)]
        #[def(char::from_u32(lhs.to_u32().saturating_add(rhs.try_into().expect("TODO"))).expect("TODO"))]
        UAdd { lhs: char, rhs: uint },

        #[op(Punctuation::SubNeg)]
        #[def(char::from_u32(lhs.to_u32().saturating_sub(rhs.try_into().expect("TODO"))).expect("TODO"))]
        USub { lhs: char, rhs: uint },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for text as pub TextOps<'src> {
        // binary

        // TBD: it would be lovely if we could replace all these with a "implements Display" so it works for custom types too...

        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TLAdd { lhs: text, rhs: bool },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TUAdd { lhs: text, rhs: uint },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TSAdd { lhs: text, rhs: sint },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TFAdd { lhs: text, rhs: frac },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TCAdd { lhs: text, rhs: char },

        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] LTAdd { lhs: bool, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] UTAdd { lhs: uint, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] STAdd { lhs: sint, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] FTAdd { lhs: frac, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] CTAdd { lhs: char, rhs: text },

        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TTAdd { lhs: text, rhs: text },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for none as pub NoneOps<'src> {
        // unary

        #[op(Punctuation::Coalesce)] #[def(lhs)] Coalesce { lhs: none, }, // TODO: don't we want to coalesce from other types?
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for fail as pub FailOps<'src> {
        // unary

        #[op(Punctuation::Coalesce)] #[def(lhs)] Coalesce { lhs: fail, }, // TODO: this is a placeholder
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Literal<'src, T> {
    pub src: Lexeme<'src>,
    pub value: T,
}

impl<'src, T: Clone> Evaluate<'src, T> for Literal<'src, T> {
    fn eval(&self, _env: &Environment<'src>) -> Result<T, Failure> {
        Ok(self.value.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Variable<'src, T> {
    pub ident: Lexeme<'src>,
    _marker: PhantomData<T>, // TODO
}

impl<'src, T: Clone + Primitive> Evaluate<'src, T> for Variable<'src, T> {
    fn eval(&self, env: &Environment<'src>) -> Result<T, Failure> {
        let stack = env.stack();
        let ptr = stack
            .tlookup(self.ident.lex)
            .expect("should be guaranteed by analysis");
        // SAFETY: static analysis promises that the variable is of the type it claims to be
        let val = unsafe { stack.get::<T>(ptr) }.expect("should be guaranteed by analysis");
        Ok(val.clone())
    }
}

/// Post-analysis expression - static analysis promises all operations satisfy type restrictions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<'src, T: Output> {
    Op {
        /// The executable operation
        op: Box<T::Ops<'src>>,
        /// Lexeme for the operator punctuation
        lex: Lexeme<'src>,
    },
    /// A literal value
    Lit(Literal<'src, T>),
    /// A variable value
    Var(Variable<'src, T>),
}

pub trait EvalOutput:
    Sized // required for `Evaluate`
    + Clone // required for `Expr::Lit` and `Expr::Var`
    + Output // required for `Expr::Op`
    + Primitive // required for `Expr::Var`
{}

impl<T: Sized + Clone + Output + Primitive> EvalOutput for T {}

impl<'src, T: EvalOutput> Evaluate<'src, T> for Expr<'src, T> {
    fn eval(&self, env: &Environment<'src>) -> Result<T, Failure> {
        match self {
            Expr::Op { op, .. } => op.eval(env),
            Expr::Lit(lit) => lit.eval(env),
            Expr::Var(var) => var.eval(env),
        }
    }
}

/// Like [`Evaluate`], but returns nothing and gets to mutate state
pub trait Execute<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure>;
}

#[expect(non_camel_case_types, reason = "enables macro magic")]
#[derive(Debug, Clone)]
pub enum AnyExpr<'src> {
    none(Expr<'src, none>),
    fail(Expr<'src, fail>),
    bool(Expr<'src, bool>),
    uint(Expr<'src, uint>),
    sint(Expr<'src, sint>),
    frac(Expr<'src, frac>),
    char(Expr<'src, char>),
    text(Expr<'src, text>),
}

/// Assigns to a variable of type `T`
#[derive(Debug, Clone)]
pub struct Assignment<'src, T: Output> {
    pub var: Pointer<T>,
    pub val: Expr<'src, T>,
}

impl<'src, T: EvalOutput> Execute<'src> for Assignment<'src, T> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        let tmp = self.val.eval(env)?;
        // SAFETY: invariant promised by static analysis
        *unsafe { env.stack_mut().get_mut(self.var) }.expect("guaranteed by static analysis") = tmp;
        Ok(())
    }
}

// TODO: this style doesn't allow for custom types
#[derive(Debug, Clone)]
pub enum AssignmentStmt<'src> {
    None(Assignment<'src, none>),
    Fail(Assignment<'src, fail>),
    UInt(Assignment<'src, uint>),
    SInt(Assignment<'src, sint>),
    Frac(Assignment<'src, frac>),
    Char(Assignment<'src, char>),
    Text(Assignment<'src, text>),
}

impl<'src> Execute<'src> for AssignmentStmt<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        match self {
            Self::None(x) => x.exec(env),
            Self::Fail(x) => x.exec(env),
            Self::UInt(x) => x.exec(env),
            Self::SInt(x) => x.exec(env),
            Self::Frac(x) => x.exec(env),
            Self::Char(x) => x.exec(env),
            Self::Text(x) => x.exec(env),
        }
    }
}

/// Mainly used for calling a function or mutating state
// TBD: but Evaluate doesnt allow state to be mutated?
#[derive(Debug, Clone)]
pub struct ExprStmt<'src> {
    pub expr: Expr<'src, ()>,
}

impl<'src> Execute<'src> for ExprStmt<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        self.expr.eval(env)
    }
}

#[derive(Debug, Clone)]
pub struct PrintStmt<'src> {
    pub what: Expr<'src, text>, // TBD: what about literals?
}

impl<'src> Execute<'src> for PrintStmt<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        print!("{}", self.what.eval(env)?);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum OrStmt<'src> {
    More(Box<IfStmt<'src>>),
    Final(BlockStmt<'src>),
}

impl<'src> Execute<'src> for OrStmt<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        match self {
            Self::More(stmt) => stmt.exec(env),
            Self::Final(stmt) => stmt.exec(env),
        }
    }
}

/// Distinct from an if-*expression*
#[derive(Debug, Clone)]
pub struct IfStmt<'src> {
    pub pred: Expr<'src, bool>,
    pub then: BlockStmt<'src>,
    pub or: Option<OrStmt<'src>>,
}

impl<'src> Execute<'src> for IfStmt<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        if self.pred.eval(env)? {
            self.then.exec(env)
        } else if let Some(or) = self.or.as_mut() {
            or.exec(env)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlockStmt<'src> {
    pub items: Vec<Statement<'src>>,
}

impl<'src> Execute<'src> for BlockStmt<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        for item in &mut self.items {
            item.exec(env)?;
        }
        Ok(())
    }
}

/// Post-analysis statement
#[derive(Debug, Clone)]
pub enum Statement<'src> {
    Assign(Assignment<'src, ()>), // TODO: what type do we assign?
    Expr(ExprStmt<'src>),
    Print(PrintStmt<'src>),
    If(IfStmt<'src>),
    Block(BlockStmt<'src>),
}

impl<'src> Execute<'src> for Statement<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        match self {
            Self::Assign(stmt) => stmt.exec(env),
            Self::Expr(stmt) => stmt.exec(env),
            Self::Print(stmt) => stmt.exec(env),
            Self::If(stmt) => stmt.exec(env),
            Self::Block(stmt) => stmt.exec(env),
        }
    }
}

/// Post-analysis program
#[derive(Debug, Clone, Default)]
pub struct Program<'src> {
    pub entry: BlockStmt<'src>,
}

impl Program<'_> {
    pub const fn new() -> Self {
        Self {
            entry: BlockStmt { items: Vec::new() },
        }
    }
}

impl<'src> Execute<'src> for Program<'src> {
    fn exec(&mut self, env: &mut Environment<'src>) -> Result<(), Failure> {
        self.entry.exec(env)
    }
}

pub trait FromLiteral: Output {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>>;
}

impl FromLiteral for none {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::Keyword(Keyword::None) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value: (),
            }))
        } else {
            None
        }
    }
}
impl FromLiteral for bool {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::BoolLiteral(value) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            None
        }
    }
}
impl FromLiteral for uint {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::UIntLiteral(value) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            None
        }
    }
}
impl FromLiteral for sint {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::SIntLiteral(value) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            None
        }
    }
}
impl FromLiteral for frac {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::FracLiteral(value) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            None
        }
    }
}
impl FromLiteral for char {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::CharLiteral(value) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value: value.ch,
            }))
        } else {
            None
        }
    }
}
impl FromLiteral for text {
    fn from_literal<'src>(token: Token<'src>) -> Option<Expr<'src, Self>> {
        if let LexValue::TextLiteral(value) = token.val {
            Some(Expr::Lit(Literal {
                src: token.into(),
                value: value.content.to_string(),
            }))
        } else {
            None
        }
    }
}

fn analyze_typed_expr<'src, T: FromLiteral>(
    source: &'src str,
    static_env: &StaticEnvironment<'src>,
    expr: ast::Expr<'src>,
) -> Result<Expr<'src, T>, Vec<ContextError<'src>>> {
    let op_args = match expr {
        ast::Expr::Literal(x) => return T::from_literal(x.value).ok_or_else(|| todo!("error")),

        ast::Expr::Variable(x) => todo!(),

        ast::Expr::Grouping(x) => return analyze_typed_expr(source, static_env, x.expr),

        ast::Expr::Type(x) => todo!(),

        ast::Expr::Binary(x) => {
            let lhs = analyze_expr(source, static_env, x.lhs);
            let rhs = analyze_expr(source, static_env, x.rhs);
            let (lhs, rhs) = match (lhs, rhs) {
                (Ok(lhs), Ok(rhs)) => Ok((lhs, rhs)),
                (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                (Err(e1), Err(e2)) => Err(std::iter::chain(e1, e2).collect()),
            }?;
            let op = x.op.val.to_token_eq().expect("only TokenEq ops supported");
            let lex = Lexeme::from(x.op);
            (op, Some(lhs), Some(rhs), lex)
        }

        ast::Expr::Unary(x) => {
            let operand = analyze_expr(source, static_env, x.operand)?;
            let op = x.op.val.to_token_eq().expect("only TokenEq ops supported");
            let lex = Lexeme::from(x.op);
            match x.side {
                OpSide::Left => (op, Some(operand), None, lex),
                OpSide::Right => (op, None, Some(operand), lex),
            }
        }
    };
    <T::Ops<'src> as Operators<'src>>::new_expr(op_args).map_err(|_| todo!("incompatible error"))
}

fn analyze_expr<'src>(
    source: &'src str,
    static_env: &StaticEnvironment<'src>,
    expr: ast::Expr<'src>,
) -> Result<AnyExpr<'src>, Vec<ContextError<'src>>> {
    let op_args = match expr {
        ast::Expr::Literal(x) => {
            let src = x.value.into();
            return match x.value.val {
                LexValue::UIntLiteral(value) => {
                    Ok(AnyExpr::uint(Expr::Lit(Literal { src, value })))
                }
                LexValue::SIntLiteral(value) => {
                    Ok(AnyExpr::sint(Expr::Lit(Literal { src, value })))
                }
                LexValue::FracLiteral(value) => {
                    Ok(AnyExpr::frac(Expr::Lit(Literal { src, value })))
                }
                LexValue::CharLiteral(value) => Ok(AnyExpr::char(Expr::Lit(Literal {
                    src,
                    value: value.ch,
                }))),
                LexValue::TextLiteral(value) => Ok(AnyExpr::text(Expr::Lit(Literal {
                    src,
                    value: value.content.to_string(),
                }))),
                LexValue::BoolLiteral(value) => {
                    Ok(AnyExpr::bool(Expr::Lit(Literal { src, value })))
                }
                LexValue::Keyword(Keyword::None) => {
                    Ok(AnyExpr::none(Expr::Lit(Literal { src, value: () })))
                }
                _ => unimplemented!("should be handled by grammar"),
            };
        }

        ast::Expr::Variable(x) => todo!(),

        ast::Expr::Grouping(x) => return analyze_expr(source, static_env, x.expr),

        ast::Expr::Type(x) => todo!(),

        ast::Expr::Binary(x) => {
            let lhs = analyze_expr(source, static_env, x.lhs);
            let rhs = analyze_expr(source, static_env, x.rhs);
            let (lhs, rhs) = match (lhs, rhs) {
                (Ok(lhs), Ok(rhs)) => Ok((lhs, rhs)),
                (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                (Err(e1), Err(e2)) => Err(std::iter::chain(e1, e2).collect()),
            }?;
            let op = x.op.val.to_token_eq().expect("only TokenEq ops supported");
            let lex = Lexeme::from(x.op);
            (op, Some(lhs), Some(rhs), lex)
        }

        ast::Expr::Unary(x) => {
            let operand = analyze_expr(source, static_env, x.operand)?;
            let op = x.op.val.to_token_eq().expect("only TokenEq ops supported");
            let lex = Lexeme::from(x.op);
            match x.side {
                OpSide::Left => (op, Some(operand), None, lex),
                OpSide::Right => (op, None, Some(operand), lex),
            }
        }
    };
    Err(op_args)
        .or_else(BoolOps::new_any_expr)
        .or_else(UIntOps::new_any_expr)
        .or_else(SIntOps::new_any_expr)
        .or_else(FracOps::new_any_expr)
        .or_else(CharOps::new_any_expr)
        .or_else(TextOps::new_any_expr)
        .or_else(NoneOps::new_any_expr)
        .or_else(FailOps::new_any_expr)
        .map_err(|_| todo!("incompatible error"))
}

fn analyze_decl<'src>(
    source: &'src str,
    static_env: &mut StaticEnvironment<'src>,
    decl: ast::Declaration<'src>,
) -> Option<Result<Statement<'src>, Vec<ContextError<'src>>>> {
    match decl {
        ast::Declaration::Var(def) => {
            match def.var_kw.val {
                LexValue::Keyword(Keyword::Let) => (), // TODO
                LexValue::Keyword(Keyword::Uni) => (), // TODO
                LexValue::Keyword(Keyword::Pvt) => (), // TODO
                _ => unimplemented!("should be caught by grammar"),
            }
            def.ident.lex;
            if let Some(x) = def.assignment {
                let expr = analyze_expr(source, static_env, x.expr);
                Some(todo!())
            } else {
                None
            }
        }

        ast::Declaration::Stmt(stmt) => match stmt {
            ast::Statement::Expr(stmt) => todo!(),

            ast::Statement::Print(stmt) => Some(
                analyze_typed_expr(source, static_env, stmt.group.expr)
                    .map(|what| Statement::Print(PrintStmt { what })),
            ),

            ast::Statement::If(stmt) => todo!(),

            ast::Statement::Block(stmt) => Some(
                stmt.body
                    .into_iter()
                    .filter_map(|decl| analyze_decl(source, static_env, decl))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|items| Statement::Block(BlockStmt { items })),
            ),
        },
    }
}

// TODO: can this be refactored to an iterator?
pub fn analyze<'src>(
    source: &'src str,
    prgm: ast::Program<'src>,
) -> Result<Program<'src>, Vec<ContextError<'src>>> {
    let mut static_env = StaticEnvironment::new();
    Ok(Program {
        entry: BlockStmt {
            items: prgm
                .into_iter()
                .filter_map(|decl| analyze_decl(source, &mut static_env, decl))
                .collect::<Result<_, _>>()?,
        },
    })
}
