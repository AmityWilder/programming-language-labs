use crate::{
    Todo, analyzer::static_env::ConstantValue, error::OpSide, scanner::token::value::TokenEq,
};
use std::collections::{BTreeMap, BTreeSet};

/// Like an [`ArrayType`], but the length is determined at runtime
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct ViewType<'src> {
    pub item: StaticType<'src>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct ArrayType<'src> {
    pub item: StaticType<'src>,
    pub len: usize, // TODO: constant expression? bounds?
}

/// Alternative for operations that can fail
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub enum OrType {
    /// Produces null on fail
    None,
    /// Promises to never fail
    #[default]
    Nevr,
    /// Raises an exception on fail
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct Fallible<'src> {
    pub then: StaticType<'src>,
    pub or: OrType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct RecType<'src> {
    pub name: &'src str,
    pub fields: BTreeMap<&'src str, StaticType<'src>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct SupType<'src> {
    pub name: &'src str,
    pub variants: BTreeSet<StaticType<'src>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct SubType<'src> {
    pub name: &'src str,
    pub base: StaticType<'src>,
    pub reqs: Vec<TypeBound<'src>>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct CatType<'src> {
    pub name: &'src str,
    pub repr: Box<StaticType<'src>>,
    pub values: BTreeSet<ConstantValue>,
}

/// A matcher for any function with the specified combination of parameters and return
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct FunShape<'src> {
    pub params: Vec<StaticType<'src>>,
    pub ret: StaticType<'src>,
    // TBD: captures?
}

/// A specific, concrete function
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct FunType<'src> {
    pub name: &'src str,
    pub shape: FunShape<'src>,
}

/// Monomorphized or concrete type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub enum StaticType<'src> {
    /// Return type of a statement
    None,
    /// Return type of an endless loop - not constructable, only usable in analysis
    #[default]
    Nevr,
    /// Exception type
    Fail,

    /// Boolean
    Bool,
    /// A single byte
    Byte,
    /// Unsigned integer
    Uint,
    /// Signed integer
    Sint,
    /// Floating point
    Flpt,
    /// Unicode character
    Char,
    /// String
    Text,

    /// Slice of another type
    View(Box<ViewType<'src>>),
    /// Array of another type
    Arr(Box<ArrayType<'src>>),

    /// Reference to another type
    Ptr(Box<Self>),
    /// Anonymous supertype
    OrTy(Box<Fallible<'src>>),
    /// Structure
    Rec(RecType<'src>),
    /// Union
    Sup(SupType<'src>),
    /// Another type, but with runtime restrictions
    Sub(Box<SubType<'src>>),
    /// Constant group
    Cat(Box<Self>),
    /// Callable
    Fun(Box<FunType<'src>>),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq)]
pub enum TypeBound<'src> {
    /// Matches another type
    Exactly(AbstractType<'src>),

    /// Possesses a member (method or field) meeting the qualifications
    Has {
        /// Name of the member
        member: &'src str,
        /// Requirements the member must satisfy
        reqs: Vec<TypeBound<'src>>,
    },

    /// Fits a function shape
    Shape(FunShape<'src>),

    /// An operation the type must support
    Supports {
        side: OpSide,
        op: TokenEq,
        other: AbstractType<'src>,
        output: AbstractType<'src>,
    },

    /// An expression the type must be guaranteed to satisfy at runtime, such as "x!=0", "x>=0", "6<=x<=10", "x.foo==x.bar", etc.
    /// Mainly enforced by checking if conditions/assertions guard this, or if the user promises it unsafely.
    Satisfies(Todo),
}

impl Default for TypeBound<'_> {
    fn default() -> Self {
        Self::Exactly(AbstractType::default())
    }
}

/// A type with generic parameters
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct GenericType<'src> {
    pub ident: &'src str,
    pub params: Vec<GenericParam<'src>>,
}

/// A type that is not yet known
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default)]
pub struct GenericParam<'src> {
    pub ident: &'src str,
    pub reqs: Vec<TypeBound<'src>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum AbstractType<'src> {
    /// All types should be this variant after monomorphization
    Concrete(StaticType<'src>),
    Generic(GenericType<'src>),
    Param(GenericParam<'src>),
}

impl Default for AbstractType<'_> {
    fn default() -> Self {
        Self::Concrete(StaticType::default())
    }
}
