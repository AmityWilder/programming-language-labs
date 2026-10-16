use crate::{
    analyzer::static_env::{Domain, StaticEnvironment},
    error::{ContextError, ErrorType, OpSide},
    exec::{
        env::{Environment, Primitive},
        error::Failure,
        eval::ValueType,
    },
    grammar::ast,
    scanner::token::{
        ExpansionData, Token,
        punc::Punctuation,
        value::{Keyword, LexValue, TextLiteral, TokenEq},
    },
    spec_ops::{BitRotate, BitShift},
    types::{fail, flpt, nevr, none, sint, text, uint},
};
use std::{marker::PhantomData, range::Range};

pub mod static_env;

// TODO: all of this is very yucky. it desperately needs to be completely refactored.

/// Valueless token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lexeme<'src> {
    pub lex: &'src str,
    pub mac: Option<ExpansionData<'src>>,
}

impl<'src> Lexeme<'src> {
    pub fn lex_range(&self, source: &'src str) -> Range<usize> {
        source
            .substr_range(self.lex)
            .expect("lexeme should be a substring of source")
    }
}

impl<'src> From<Token<'src>> for Lexeme<'src> {
    fn from(Token { lex, val: _, mac }: Token<'src>) -> Self {
        Self { lex, mac }
    }
}

/// Post-analysis evaluation
pub trait Evaluate<T> {
    fn eval(&self, env: &mut Environment) -> Result<T, Failure>;
}

pub trait LangType {
    const VALUE_TYPE: ValueType;

    /// In case the runtime type is different
    fn get_type(&self) -> ValueType {
        Self::VALUE_TYPE
    }

    fn szof(&self) -> usize {
        std::mem::size_of_val(self)
    }

    fn align(&self) -> usize {
        std::mem::align_of_val(self)
    }
}

impl LangType for none {
    const VALUE_TYPE: ValueType = ValueType::None;
}
impl LangType for nevr {
    const VALUE_TYPE: ValueType = ValueType::Nevr;
}
impl LangType for fail {
    const VALUE_TYPE: ValueType = ValueType::Fail;
}
impl LangType for bool {
    const VALUE_TYPE: ValueType = ValueType::Bool;
}
impl LangType for uint {
    const VALUE_TYPE: ValueType = ValueType::UInt;
}
impl LangType for sint {
    const VALUE_TYPE: ValueType = ValueType::SInt;
}
impl LangType for flpt {
    const VALUE_TYPE: ValueType = ValueType::Flpt;
}
impl LangType for char {
    const VALUE_TYPE: ValueType = ValueType::Char;
}
impl LangType for text {
    const VALUE_TYPE: ValueType = ValueType::Text;
}
impl LangType for str {
    const VALUE_TYPE: ValueType = ValueType::Text;
}
impl LangType for std::borrow::Cow<'_, str> {
    const VALUE_TYPE: ValueType = ValueType::Text;
}

pub trait Output: Sized + LangType {
    /// Structure for operations that **result** in `Self`
    type Ops<'src>: 'src + Operators<'src, Output = Self> + Evaluate<Self> + std::fmt::Debug + Clone;
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

        impl Evaluate<$Type> for $Ops<'_> {
            fn eval(&self, env: &mut Environment) -> Result<$Type, Failure> {
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

        // TBD: should float equality be supported?
        #[op(Punctuation::Eq)] #[def(lhs == rhs)] FEq { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] FNe { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] FLt { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] FGt { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] FLe { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] FGe { lhs: flpt, rhs: flpt },

        #[op(Punctuation::Eq)] #[def(lhs == rhs)] CEq { lhs: char, rhs: char },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] CNe { lhs: char, rhs: char },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] CLt { lhs: char, rhs: char },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] CGt { lhs: char, rhs: char },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] CLe { lhs: char, rhs: char },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] CGe { lhs: char, rhs: char },

        // TODO: what about unowned strings?
        #[op(Punctuation::Eq)] #[def(lhs == rhs)] TEq { lhs: text, rhs: text },
        #[op(Punctuation::Ne)] #[def(lhs != rhs)] TNe { lhs: text, rhs: text },
        #[op(Punctuation::Lt)] #[def(lhs <  rhs)] TLt { lhs: text, rhs: text },
        #[op(Punctuation::Gt)] #[def(lhs >  rhs)] TGt { lhs: text, rhs: text },
        #[op(Punctuation::Le)] #[def(lhs <= rhs)] TLe { lhs: text, rhs: text },
        #[op(Punctuation::Ge)] #[def(lhs >= rhs)] TGe { lhs: text, rhs: text },

        // unary

        #[op(Punctuation::Not)] #[def(!rhs)] Not { , rhs: bool },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for uint as pub UIntOps<'src> {
        // binary

        #[op(Punctuation::And)] #[def(lhs & rhs)] And { lhs: uint, rhs: uint },
        #[op(Punctuation::Or)]  #[def(lhs | rhs)] Or  { lhs: uint, rhs: uint },
        #[op(Punctuation::Xor)] #[def(lhs ^ rhs)] Xor { lhs: uint, rhs: uint },

        #[op(Punctuation::Nand)] #[def(!(lhs & rhs))] Nand { lhs: uint, rhs: uint },
        #[op(Punctuation::Nor)]  #[def(!(lhs | rhs))] Nor  { lhs: uint, rhs: uint },
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
        #[op(Punctuation::Or)]  #[def(lhs | rhs)] Or  { lhs: sint, rhs: sint },
        #[op(Punctuation::Xor)] #[def(lhs ^ rhs)] Xor { lhs: sint, rhs: sint },

        #[op(Punctuation::Nand)] #[def(!(lhs & rhs))] Nand { lhs: sint, rhs: sint },
        #[op(Punctuation::Nor)]  #[def(!(lhs | rhs))] Nor  { lhs: sint, rhs: sint },
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
    for flpt as pub FlptOps<'src> {
        // binary

        #[op(Punctuation::Add)]    #[def(lhs + rhs)]     Add { lhs: flpt, rhs: flpt },
        #[op(Punctuation::SubNeg)] #[def(lhs - rhs)]     Sub { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Mul)]    #[def(lhs * rhs)]     Mul { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Div)]    #[def(lhs / rhs)]     Div { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Rem)]    #[def(lhs % rhs)]     Rem { lhs: flpt, rhs: flpt },
        #[op(Punctuation::Pow)]    #[def(lhs.powf(rhs))] Pow { lhs: flpt, rhs: flpt },

        // unary

        #[op(Punctuation::SubNeg)] #[def(-rhs)] Neg { , rhs: flpt },
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
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TFAdd { lhs: text, rhs: flpt },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TCAdd { lhs: text, rhs: char },

        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] LTAdd { lhs: bool, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] UTAdd { lhs: uint, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] STAdd { lhs: sint, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] FTAdd { lhs: flpt, rhs: text },
        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] CTAdd { lhs: char, rhs: text },

        #[op(Punctuation::Add)] #[def(format!("{lhs}{rhs}"))] TTAdd { lhs: text, rhs: text },
    }
}

impl_output! {
    #[derive(Debug, Clone)]
    for none as pub NoneOps<'src> {
        // binary

        // TODO: lvalue type?
        #[op(Punctuation::Assign)] #[def(todo!("assign rhs to lhs"))] Assign { lhs: sint, rhs: sint },

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

impl<T: Clone> Evaluate<T> for Literal<'_, T> {
    fn eval(&self, _env: &mut Environment) -> Result<T, Failure> {
        Ok(self.value.clone())
    }
}

#[derive(Debug)]
pub struct Variable<'src, T: ?Sized> {
    pub ident: Lexeme<'src>,
    ptr: Range<usize>,
    // TODO: can the stack and global share memory so the distinction can be in `ptr`
    // instead of needing a field for identifying domain?
    dom: Domain,
    _marker: PhantomData<T>, // TODO
}

impl<T: ?Sized> Clone for Variable<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Variable<'_, T> {}

impl<T: ?Sized> PartialEq for Variable<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        // TODO: names and pointers both get recycled
        self.ident == other.ident && self.ptr == other.ptr
    }
}

impl<T: ?Sized> Eq for Variable<'_, T> {}

impl<T: ?Sized> Default for Variable<'_, T> {
    fn default() -> Self {
        Self::new(Lexeme::default(), Range::default(), Domain::default())
    }
}

impl<'src, T: ?Sized> Variable<'src, T> {
    const fn new(ident: Lexeme<'src>, ptr: Range<usize>, dom: Domain) -> Self {
        Self {
            ident,
            ptr,
            dom,
            _marker: PhantomData,
        }
    }
}

impl<T: Clone + Primitive> Evaluate<T> for Variable<'_, T> {
    fn eval(&self, env: &mut Environment) -> Result<T, Failure> {
        let mem = match self.dom {
            Domain::Stack => env.stack(),
            Domain::Global => env.global(),
        };
        // SAFETY: static analysis promises that the variable is of the type it claims to be
        let val = unsafe { mem.get_as::<T>(self.ptr) }.expect("should be guaranteed by analysis");
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

impl<T: EvalOutput> Evaluate<T> for Expr<'_, T> {
    fn eval(&self, env: &mut Environment) -> Result<T, Failure> {
        match self {
            Expr::Op { op, .. } => op.eval(env),
            Expr::Lit(lit) => lit.eval(env),
            Expr::Var(var) => var.eval(env),
        }
    }
}

/// Like [`Evaluate`], but returns nothing and gets to mutate state
pub trait Execute {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure>;
}

#[expect(non_camel_case_types, reason = "enables macro magic")]
#[derive(Debug, Clone)]
pub enum AnyExpr<'src> {
    none(Expr<'src, none>),
    fail(Expr<'src, fail>),
    bool(Expr<'src, bool>),
    uint(Expr<'src, uint>),
    sint(Expr<'src, sint>),
    flpt(Expr<'src, flpt>),
    char(Expr<'src, char>),
    text(Expr<'src, text>),
}

impl AnyExpr<'_> {
    pub const fn get_type(&self) -> ValueType {
        match self {
            AnyExpr::none(_) => ValueType::None,
            AnyExpr::fail(_) => ValueType::Fail,
            AnyExpr::bool(_) => ValueType::Bool,
            AnyExpr::uint(_) => ValueType::UInt,
            AnyExpr::sint(_) => ValueType::SInt,
            AnyExpr::flpt(_) => ValueType::Flpt,
            AnyExpr::char(_) => ValueType::Char,
            AnyExpr::text(_) => ValueType::Text,
        }
    }
}

/// Assigns to a variable of type `T`
#[derive(Debug, Clone)]
pub struct Assignment<'src, T: Output> {
    pub var: Variable<'src, T>,
    pub val: Expr<'src, T>,
}

impl<T: EvalOutput> Execute for Assignment<'_, T> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        let tmp = self.val.eval(env)?;
        let mem = match self.var.dom {
            Domain::Stack => env.stack_mut(),
            Domain::Global => env.global_mut(),
        };
        // SAFETY: invariant promised by static analysis
        // TODO: how does this react to a String being dropped? can it drop a string when the scope pops??
        let var =
            unsafe { mem.get_mut_as::<T>(self.var.ptr) }.expect("guaranteed by static analysis");
        *var = tmp;
        Ok(())
    }
}

// TODO: this style doesn't allow for custom types
#[derive(Debug, Clone)]
pub enum AssignmentStmt<'src> {
    None(Assignment<'src, none>),
    Fail(Assignment<'src, fail>),
    Bool(Assignment<'src, bool>),
    UInt(Assignment<'src, uint>),
    SInt(Assignment<'src, sint>),
    Flpt(Assignment<'src, flpt>),
    Char(Assignment<'src, char>),
    Text(Assignment<'src, text>),
}

impl Execute for AssignmentStmt<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        match self {
            Self::None(x) => x.exec(env),
            Self::Bool(x) => x.exec(env),
            Self::UInt(x) => x.exec(env),
            Self::SInt(x) => x.exec(env),
            Self::Flpt(x) => x.exec(env),
            Self::Char(x) => x.exec(env),
            Self::Text(x) => x.exec(env),
            Self::Fail(x) => x.exec(env),
        }
    }
}

/// Mainly used for calling a function or mutating state
#[derive(Debug, Clone)]
pub struct ExprStmt<'src> {
    pub expr: Expr<'src, ()>,
}

impl Execute for ExprStmt<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        self.expr.eval(env)
    }
}

#[derive(Debug, Clone)]
pub struct PrintStmt<'src> {
    pub what: Expr<'src, text>, // TBD: what about literals?
}

impl Execute for PrintStmt<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        print!("{}", self.what.eval(env)?);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum OrStmt<'src> {
    More(Box<IfStmt<'src>>),
    Final(BlockStmt<'src>),
}

impl Execute for OrStmt<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
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

impl Execute for IfStmt<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
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

impl Execute for BlockStmt<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        for item in &mut self.items {
            item.exec(env)?;
        }
        Ok(())
    }
}

/// Post-analysis statement
#[derive(Debug, Clone)]
pub enum Statement<'src> {
    Assign(AssignmentStmt<'src>), // TODO: what type do we assign?
    Expr(ExprStmt<'src>),
    Print(PrintStmt<'src>),
    If(IfStmt<'src>),
    Block(BlockStmt<'src>),
}

impl Execute for Statement<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
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

impl Execute for Program<'_> {
    fn exec(&mut self, env: &mut Environment) -> Result<(), Failure> {
        self.entry.exec(env)
    }
}

fn type_mismatch_error<'src, T: LangType>(
    _source: &'src str,
    actual: ValueType,
) -> ContextError<'src> {
    let expected = T::VALUE_TYPE;
    todo!("error: type mismatch error\n expected: {expected}\n actual: {actual}")
}

fn literal_type_mismatch_error<'src, T: LangType>(
    source: &'src str,
    token: &Token<'src>,
) -> ContextError<'src> {
    type_mismatch_error::<T>(
        source,
        match token.val {
            LexValue::UIntLiteral(_) => uint::VALUE_TYPE,
            LexValue::SIntLiteral(_) => sint::VALUE_TYPE,
            LexValue::FlptLiteral(_) => flpt::VALUE_TYPE,
            LexValue::CharLiteral(_) => char::VALUE_TYPE,
            LexValue::TextLiteral(_) => text::VALUE_TYPE,
            LexValue::BoolLiteral(_) => bool::VALUE_TYPE,
            LexValue::Keyword(Keyword::None) => none::VALUE_TYPE,
            x => unimplemented!("{x:?} is not a type"),
        },
    )
}

pub trait FromLiteral: Output {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>>;
}

impl FromLiteral for none {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::Keyword(Keyword::None) = token.val {
            Ok(Expr::Lit(Literal {
                src: token.into(),
                value: (),
            }))
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}
impl FromLiteral for bool {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::BoolLiteral(value) = token.val {
            Ok(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}
impl FromLiteral for uint {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::UIntLiteral(value) = token.val {
            Ok(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}
impl FromLiteral for sint {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::SIntLiteral(value) = token.val {
            Ok(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}
impl FromLiteral for flpt {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::FlptLiteral(value) = token.val {
            Ok(Expr::Lit(Literal {
                src: token.into(),
                value,
            }))
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}
impl FromLiteral for char {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::CharLiteral(value) = token.val {
            Ok(Expr::Lit(Literal {
                src: token.into(),
                value: value.ch,
            }))
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}
fn text_from_literal<'src>(
    source: &'src str,
    token: Token<'src>,
    value: TextLiteral<'src>,
) -> Result<Literal<'src, text>, ContextError<'src>> {
    Ok(Literal {
        src: token.into(),
        value: value
            .process()
            .map_err(|e| ContextError::token_error(source, Some(&token), e))?
            .to_string(),
    })
}
impl FromLiteral for text {
    fn from_literal<'src>(
        source: &'src str,
        token: Token<'src>,
    ) -> Result<Expr<'src, Self>, ContextError<'src>> {
        if let LexValue::TextLiteral(value) = token.val {
            text_from_literal(source, token, value).map(Expr::Lit)
        } else {
            Err(literal_type_mismatch_error::<Self>(source, &token))
        }
    }
}

fn incompatible_op<'src>(
    source: &'src str,
    (op, lhs, rhs, lex): (
        TokenEq,
        Option<AnyExpr<'src>>,
        Option<AnyExpr<'src>>,
        Lexeme<'src>,
    ),
) -> ContextError<'src> {
    ContextError::error(
        source,
        Some(lex.lex_range(source)),
        lex.mac,
        match (lhs, rhs) {
            (Some(l), Some(r)) => ErrorType::Incompatible {
                op,
                l_range: Range::default(), // TODO
                r_range: Range::default(), // TODO
                l_ty: l.get_type(),
                r_ty: r.get_type(),
            },
            (Some(l), None) => ErrorType::Unsupported {
                op,
                r_ty: l.get_type(), // shouldnt there be a distinction between prefix and postfix?...
                r_range: Range::default(), // TODO
            },
            (None, Some(r)) => ErrorType::Unsupported {
                op,
                r_ty: r.get_type(),
                r_range: Range::default(), // TODO
            },
            (None, None) => unimplemented!(),
        },
    )
}

fn analyze_typed_expr<'src, T: FromLiteral>(
    source: &'src str,
    static_env: &StaticEnvironment<'src>,
    expr: ast::Expr<'src>,
) -> Result<Expr<'src, T>, Vec<ContextError<'src>>> {
    let op_args = match expr {
        ast::Expr::Literal(x) => return T::from_literal(source, x.value).map_err(|e| vec![e]),

        ast::Expr::Variable(x) => {
            let ident: Lexeme<'src> = x.ident.into();
            return if let Some((data, domain)) = static_env.lookup(ident.lex) {
                if data.ty == T::VALUE_TYPE {
                    Ok(Expr::Var(Variable::new(ident, data.range, domain)))
                } else {
                    Err(vec![type_mismatch_error::<T>(source, data.ty)])
                }
            } else {
                todo!("variable not found in scope error")
            };
        }

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
    <T::Ops<'src> as Operators<'src>>::new_expr(op_args)
        .map_err(|args| incompatible_op(source, args))
        .map_err(|e| vec![e])
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
                LexValue::FlptLiteral(value) => {
                    Ok(AnyExpr::flpt(Expr::Lit(Literal { src, value })))
                }
                LexValue::CharLiteral(value) => Ok(AnyExpr::char(Expr::Lit(Literal {
                    src,
                    value: value.ch,
                }))),
                LexValue::TextLiteral(value) => Ok(AnyExpr::text(Expr::Lit(
                    text_from_literal(source, x.value, value).map_err(|e| vec![e])?, // HACK: need to make these more consistent without bloat
                ))),
                LexValue::BoolLiteral(value) => {
                    Ok(AnyExpr::bool(Expr::Lit(Literal { src, value })))
                }
                LexValue::Keyword(Keyword::None) => {
                    Ok(AnyExpr::none(Expr::Lit(Literal { src, value: () })))
                }
                _ => unimplemented!("should be handled by grammar"),
            };
        }

        ast::Expr::Variable(x) => {
            let ident: Lexeme<'src> = x.ident.into();
            let (data, dom) = static_env.lookup(ident.lex).unwrap_or_else(|| {
                todo!(
                    "error: variable {} has not been defined at this point",
                    ident.lex
                )
            });
            let ptr = data.range;
            return match data.ty {
                ValueType::None => Ok(AnyExpr::none(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::Bool => Ok(AnyExpr::bool(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::UInt => Ok(AnyExpr::uint(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::SInt => Ok(AnyExpr::sint(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::Flpt => Ok(AnyExpr::flpt(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::Char => Ok(AnyExpr::char(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::Text => Ok(AnyExpr::text(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::Fail => Ok(AnyExpr::fail(Expr::Var(Variable::new(ident, ptr, dom)))),
                ValueType::Ptr => todo!(),
                _ => unimplemented!("not a valid variable type"), // TBD: should this be an error or is it the grammar's job?
            };
        }

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
        .or_else(FlptOps::new_any_expr)
        .or_else(CharOps::new_any_expr)
        .or_else(TextOps::new_any_expr)
        .or_else(NoneOps::new_any_expr)
        .or_else(FailOps::new_any_expr)
        .map_err(|args| incompatible_op(source, args))
        .map_err(|e| vec![e])
}

fn analyze_if_stmt<'src>(
    source: &'src str,
    static_env: &mut StaticEnvironment<'src>,
    stmt: ast::IfStmt<'src>,
) -> Result<IfStmt<'src>, Vec<ContextError<'src>>> {
    let body = stmt.then;
    Ok(IfStmt {
        pred: analyze_typed_expr(source, static_env, stmt.cond.pred)?,
        then: analyze_block(source, static_env, body.then)?,
        or: if let Some(or) = body.or {
            Some(if let Some(cond) = or.cond {
                OrStmt::More(Box::new(analyze_if_stmt(
                    source,
                    static_env,
                    ast::IfStmt {
                        cond,
                        then: or.then,
                    },
                )?))
            } else {
                let body = or.then;
                OrStmt::Final(analyze_block(source, static_env, body.then)?)
            })
        } else {
            None
        },
    })
}

fn analyze_block<'src>(
    source: &'src str,
    static_env: &mut StaticEnvironment<'src>,
    block: ast::BlockStmt<'src>,
) -> Result<BlockStmt<'src>, Vec<ContextError<'src>>> {
    static_env.push_scope();
    block
        .body
        .into_iter()
        .filter_map(|decl| analyze_decl(source, static_env, decl).transpose())
        .collect::<Result<Vec<_>, _>>()
        .map(|items| {
            static_env.pop_scope(); // TODO: will this break evaluation?
            BlockStmt { items }
        })
}

fn analyze_decl<'src>(
    source: &'src str,
    static_env: &mut StaticEnvironment<'src>,
    decl: ast::Declaration<'src>,
) -> Result<Option<Statement<'src>>, Vec<ContextError<'src>>> {
    match decl {
        ast::Declaration::Var(def) => {
            // if there isn't an assignment, no action occurs
            if let Some(x) = def.assignment {
                let expr = analyze_expr(source, static_env, x.expr)?;
                let ty = expr.get_type();
                let (data, dom) = match def.var_kw.val {
                    LexValue::Keyword(Keyword::Let) => static_env
                        .push_local(def.ident.lex, ty)
                        .map(|x| (x, Domain::Stack)),
                    LexValue::Keyword(Keyword::Uni) => static_env
                        .push_global(def.ident.lex, ty)
                        .map(|x| (x, Domain::Global)),
                    LexValue::Keyword(Keyword::Pvt) => todo!(),
                    _ => unimplemented!("should be caught by grammar"),
                }
                .expect("should be constructable"); // TODO: error? or grammar?
                let ident = def.ident.into();
                let ptr = data.range;
                Ok(Some(Statement::Assign(match expr {
                    AnyExpr::none(val) => AssignmentStmt::None(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::fail(val) => AssignmentStmt::Fail(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::bool(val) => AssignmentStmt::Bool(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::uint(val) => AssignmentStmt::UInt(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::sint(val) => AssignmentStmt::SInt(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::flpt(val) => AssignmentStmt::Flpt(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::char(val) => AssignmentStmt::Char(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                    AnyExpr::text(val) => AssignmentStmt::Text(Assignment {
                        var: Variable::new(ident, ptr, dom),
                        val,
                    }),
                })))
            } else {
                // TBD: should we insert a "type TBD" instead?
                Ok(None)
            }
        }

        ast::Declaration::Stmt(stmt) => match stmt {
            ast::Statement::Expr(stmt) => analyze_typed_expr(source, static_env, stmt.expr)
                .map(|expr| Some(Statement::Expr(ExprStmt { expr }))),

            ast::Statement::Print(stmt) => analyze_typed_expr(source, static_env, stmt.group.expr)
                .map(|what| Some(Statement::Print(PrintStmt { what }))),

            ast::Statement::If(stmt) => {
                analyze_if_stmt(source, static_env, stmt).map(|stmt| Some(Statement::If(stmt)))
            }

            ast::Statement::Block(stmt) => {
                analyze_block(source, static_env, stmt).map(|block| Some(Statement::Block(block)))
            }
        },
    }
}

#[derive(Debug, Clone, Default)]
pub struct StaticAnalysis<'src> {
    pub global_memory_needed: usize,
    pub stack_memory_needed: usize,
    pub program: Program<'src>,
}

// TODO: can this be refactored to an iterator?
pub fn analyze<'src>(
    source: &'src str,
    prgm: ast::Program<'src>,
) -> Result<StaticAnalysis<'src>, Vec<ContextError<'src>>> {
    let mut static_env = StaticEnvironment::new();
    let program = Program {
        entry: BlockStmt {
            items: prgm
                .into_iter()
                .filter_map(|decl| analyze_decl(source, &mut static_env, decl).transpose())
                .collect::<Result<_, _>>()?,
        },
    };
    Ok(StaticAnalysis {
        global_memory_needed: static_env.gp_max(),
        stack_memory_needed: static_env.sp_max(),
        program,
    })
}
