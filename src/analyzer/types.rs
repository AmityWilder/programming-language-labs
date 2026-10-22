use crate::{
    Todo,
    analyzer::static_env::ConstantValue,
    error::{ContextError, ErrorType, OpSide},
    exec::error::Failure,
    grammar::ast,
    scanner::token::value::TokenEq,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    range::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeReq {
    ArrayElement,
}

impl std::fmt::Display for SizeReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArrayElement => write!(f, "array elements"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TypeSizeError {
    #[default]
    TooBig,
    IllegalUnsized(SizeReq),
}

impl std::fmt::Display for TypeSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "type size exceeds {} bytes", usize::MAX),
            Self::IllegalUnsized(x) => write!(f, "{x} must have static size"),
        }
    }
}

impl std::error::Error for TypeSizeError {}

pub trait TypeSize {
    /// Returns [`None`] if size is determined at runtime
    ///
    /// Default implementation assumes an unsized type
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        None
    }
}

/// Type has a definitive size known during static analysis, and can be pushed to the stack
pub trait StaticTypeSize: TypeSize {
    fn static_size(&self) -> Result<usize, TypeSizeError>;
}

/// Display implementation is be how the type will be shown in error messages
pub trait TypeCompatible<'src>: std::fmt::Display {
    /// TODO: this is not currently commutative!! check both ways until this is fixed!
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool;

    // TODO: change this to return an actual error
    fn expect_type<T: ast::AstNodeFinal<'src>>(
        &self,
        expect: &AbstractType<'src>,
        source: &'src str,
        node: &T,
    ) -> Result<(), ContextError<'src>>
    where
        Self: std::fmt::Debug,
    {
        if self.is_compatible(expect) {
            Ok(())
        } else {
            Err(ContextError::node_error(
                source,
                node,
                ErrorType::TypeMismatch(Box::new((expect.clone(), self.to_abstract()))),
            ))
        }
    }

    fn to_abstract(&self) -> AbstractType<'src>;
}

impl<'src, T: ?Sized + TypeCompatible<'src>> TypeCompatible<'src> for &T {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        (*self).is_compatible(other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        (*self).to_abstract()
    }
}
impl<T: ?Sized + TypeSize> TypeSize for &T {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        (*self).size()
    }
}

/// Like an [`ArrayType`], but the length is determined at runtime
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct ViewType<'src> {
    pub item: StaticType<'src>,
}

impl TypeSize for ViewType<'_> {}
impl<'src> TypeCompatible<'src> for ViewType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::View(other)) if *self == **other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<ViewType<'src>> for AbstractType<'src> {
    fn from(value: ViewType<'src>) -> Self {
        AbstractType::Concrete(StaticType::View(Box::new(value)))
    }
}
impl std::fmt::Display for ViewType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { item } = self;
        write!(f, "[{item}]")
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct ArrayType<'src> {
    pub item: StaticType<'src>,
    pub len: usize, // TODO: constant expression? bounds?
}

impl StaticTypeSize for ArrayType<'_> {
    fn static_size(&self) -> Result<usize, TypeSizeError> {
        // TODO: items should be statically sized
        // TODO: if the size exceeds what is possible, this should produce an error
        self.item
            .size()
            .ok_or(TypeSizeError::IllegalUnsized(SizeReq::ArrayElement))
            .flatten()?
            .checked_mul(self.len)
            .ok_or(TypeSizeError::TooBig)
    }
}
impl TypeSize for ArrayType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        Some(self.static_size())
    }
}
impl<'src> TypeCompatible<'src> for ArrayType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Arr(other)) if *self == **other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<ArrayType<'src>> for AbstractType<'src> {
    fn from(value: ArrayType<'src>) -> Self {
        AbstractType::Concrete(StaticType::Arr(Box::new(value)))
    }
}
impl std::fmt::Display for ArrayType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { item, len } = self;
        write!(f, "[{item}; {len}]")
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Ptr<'src> {
    pub to: StaticType<'src>,
}

impl TypeSize for Ptr<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        Some(self.static_size())
    }
}
impl StaticTypeSize for Ptr<'_> {
    fn static_size(&self) -> Result<usize, TypeSizeError> {
        // TODO: what about wide pointers?
        // TODO: are pointers ALWAYS going to be ranges?
        Ok(std::mem::size_of::<Range<usize>>())
    }
}
impl<'src> TypeCompatible<'src> for Ptr<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Ptr(other)) if *self == **other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<Ptr<'src>> for AbstractType<'src> {
    fn from(value: Ptr<'src>) -> Self {
        AbstractType::Concrete(StaticType::Ptr(Box::new(value)))
    }
}
impl std::fmt::Display for Ptr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { to } = self;
        write!(f, "@{to}") // TBD: design
    }
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

impl std::fmt::Display for OrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Nevr => "nevr",
            Self::Fail => "fail",
        })
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct Fallible<'src> {
    pub then: StaticType<'src>,
    pub or: OrType,
}

impl TypeSize for Fallible<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        match self.then.size() {
            Some(Ok(then_size)) => {
                let or_size = match self.or {
                    OrType::None | OrType::Nevr => 0, // TODO: needs to account for bit used to distinguish then/or!!
                    OrType::Fail => std::mem::size_of::<Failure>(),
                };
                Some(then_size.checked_add(or_size).ok_or(TypeSizeError::TooBig))
            }
            res => res,
        }
    }
}
impl<'src> TypeCompatible<'src> for Fallible<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        match other {
            AbstractType::Concrete(StaticType::OrTy(other)) => {
                *self == **other
                    || self.is_compatible(&AbstractType::Concrete(StaticType::Primitive(
                        match other.or {
                            OrType::None => PrimitiveType::None,
                            OrType::Nevr => PrimitiveType::Nevr,
                            OrType::Fail => PrimitiveType::Fail,
                        },
                    )))
                    || self.is_compatible(&other.then.to_abstract())
            }
            _ => false,
        }
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<Fallible<'src>> for AbstractType<'src> {
    fn from(value: Fallible<'src>) -> Self {
        AbstractType::Concrete(StaticType::OrTy(Box::new(value)))
    }
}
impl std::fmt::Display for Fallible<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { then, or } = self;
        write!(f, "{then} | {or}")
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct RecType<'src> {
    pub name: &'src str,
    pub fields: BTreeMap<&'src str, StaticType<'src>>,
}

impl TypeSize for RecType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        todo!("need to determine layout")
    }
}
impl<'src> TypeCompatible<'src> for RecType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Rec(other)) if self == other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<RecType<'src>> for AbstractType<'src> {
    fn from(value: RecType<'src>) -> Self {
        AbstractType::Concrete(StaticType::Rec(value))
    }
}
impl std::fmt::Display for RecType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, .. } = self;
        write!(f, "{name}") // TODO: what if there are multiple with the same name?
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct SupType<'src> {
    pub name: &'src str,
    pub variants: BTreeSet<StaticType<'src>>,
}

impl TypeSize for SupType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        // TODO: tag?
        let mut max_size = 0;
        for variant in &self.variants {
            max_size = match variant.size() {
                Some(Ok(size)) => max_size.max(size),
                res => return res,
            };
        }
        Some(Ok(max_size))
    }
}
impl<'src> TypeCompatible<'src> for SupType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Sup(other)) if self == other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<SupType<'src>> for AbstractType<'src> {
    fn from(value: SupType<'src>) -> Self {
        AbstractType::Concrete(StaticType::Sup(value))
    }
}
impl std::fmt::Display for SupType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, .. } = self;
        write!(f, "{name}") // TODO: what if there are multiple with the same name?
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct SubType<'src> {
    pub name: &'src str,
    pub base: StaticType<'src>,
    pub reqs: Vec<TypeBound<'src>>,
}

impl TypeSize for SubType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        // TODO: can unsized types be in subtypes?
        self.base.size()
    }
}
impl<'src> TypeCompatible<'src> for SubType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Sub(other)) if *self == **other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<SubType<'src>> for AbstractType<'src> {
    fn from(value: SubType<'src>) -> Self {
        AbstractType::Concrete(StaticType::Sub(Box::new(value)))
    }
}
impl std::fmt::Display for SubType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, .. } = self;
        write!(f, "{name}") // TODO: what if there are multiple with the same name?
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct CatType<'src> {
    pub name: &'src str,
    pub repr: Box<StaticType<'src>>,
    pub values: BTreeSet<ConstantValue>,
}

impl StaticTypeSize for CatType<'_> {
    fn static_size(&self) -> Result<usize, TypeSizeError> {
        // TBD: should category types use their value as their discriminant, or a separate integer discriminant?

        // while a constant type can be unsized, its "runtime" is static analysis time,
        // so it's allowed to have static type even if its type is runtime-sized
        self.values
            .iter()
            .map(StaticTypeSize::static_size)
            .try_fold(0, |m, v| v.map(|v| m.max(v)))
    }
}
impl TypeSize for CatType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        Some(self.static_size())
    }
}
impl<'src> TypeCompatible<'src> for CatType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Cat(other)) if *self == **other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<CatType<'src>> for AbstractType<'src> {
    fn from(value: CatType<'src>) -> Self {
        AbstractType::Concrete(StaticType::Cat(Box::new(value)))
    }
}
impl std::fmt::Display for CatType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, .. } = self;
        write!(f, "{name}") // TODO: what if there are multiple with the same name?
    }
}

/// A matcher for any function with the specified combination of parameters and return
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct FunShape<'src> {
    pub params: Vec<StaticType<'src>>,
    pub ret: StaticType<'src>,
    // TBD: captures?
}

impl<'src> TypeCompatible<'src> for FunShape<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Fun(other)) if *self == other.shape)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<FunShape<'src>> for AbstractType<'src> {
    fn from(shape: FunShape<'src>) -> Self {
        AbstractType::Concrete(StaticType::Fun(Box::new(FunType {
            name: "", // HACK
            shape,
        })))
    }
}
impl std::fmt::Display for FunShape<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { params, ret } = self;
        write!(f, "fn(")?;
        let mut has_prev = false;
        for param in params {
            if has_prev {
                f.write_str(", ")?;
            }
            write!(f, "{param}")?;
            has_prev = true;
        }
        write!(f, ") -> {ret}")
    }
}

/// A specific, concrete function
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct FunType<'src> {
    pub name: &'src str,
    pub shape: FunShape<'src>,
}

impl TypeSize for FunType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        todo!("where are functions stored?")
    }
}
impl<'src> TypeCompatible<'src> for FunType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Fun(other)) if *self == **other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<FunType<'src>> for AbstractType<'src> {
    fn from(value: FunType<'src>) -> Self {
        AbstractType::Concrete(StaticType::Fun(Box::new(value)))
    }
}
impl std::fmt::Display for FunType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { name, shape } = self;
        write!(f, "{shape}{{{name}}}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PrimitiveType {
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
}

impl StaticTypeSize for PrimitiveType {
    fn static_size(&self) -> Result<usize, TypeSizeError> {
        use crate::types::{byte, fail, flpt, nevr, none, sint, text, uint};
        Ok(match self {
            Self::None => std::mem::size_of::<none>(),
            Self::Nevr => std::mem::size_of::<nevr>(),
            Self::Fail => std::mem::size_of::<fail>(),
            Self::Bool => std::mem::size_of::<bool>(),
            Self::Byte => std::mem::size_of::<byte>(),
            Self::Uint => std::mem::size_of::<uint>(),
            Self::Sint => std::mem::size_of::<sint>(),
            Self::Flpt => std::mem::size_of::<flpt>(),
            Self::Char => std::mem::size_of::<char>(),
            Self::Text => std::mem::size_of::<text>(),
        })
    }
}
impl TypeSize for PrimitiveType {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        Some(self.static_size())
    }
}
impl<'src> TypeCompatible<'src> for PrimitiveType {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(StaticType::Primitive(other)) if self == other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        (*self).into()
    }
}
impl From<PrimitiveType> for AbstractType<'_> {
    fn from(value: PrimitiveType) -> Self {
        AbstractType::Concrete(StaticType::Primitive(value))
    }
}
impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Nevr => "nevr",
            Self::Fail => "fail",
            Self::Bool => "bool",
            Self::Byte => "byte",
            Self::Uint => "uint",
            Self::Sint => "sint",
            Self::Flpt => "flpt",
            Self::Char => "char",
            Self::Text => "text",
        })
    }
}

/// Monomorphized or concrete type
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum StaticType<'src> {
    /// Language-defined
    Primitive(PrimitiveType),

    /// Slice of another type
    View(Box<ViewType<'src>>),
    /// Array of another type
    Arr(Box<ArrayType<'src>>),

    /// Reference to another type
    Ptr(Box<Ptr<'src>>),
    /// Anonymous supertype
    OrTy(Box<Fallible<'src>>),
    /// Structure
    Rec(RecType<'src>),
    /// Union
    Sup(SupType<'src>),
    /// Another type, but with runtime restrictions
    Sub(Box<SubType<'src>>),
    /// Constant group
    Cat(Box<CatType<'src>>),
    /// Callable
    Fun(Box<FunType<'src>>),
}

impl Default for StaticType<'_> {
    fn default() -> Self {
        Self::Primitive(PrimitiveType::default())
    }
}

impl TypeSize for StaticType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        match self {
            Self::Primitive(t) => t.size(),
            Self::View(t) => t.size(),
            Self::Arr(t) => t.size(),
            Self::Ptr(t) => t.size(),
            Self::OrTy(t) => t.size(),
            Self::Rec(t) => t.size(),
            Self::Sup(t) => t.size(),
            Self::Sub(t) => t.size(),
            Self::Cat(t) => t.size(),
            Self::Fun(t) => t.size(),
        }
    }
}
impl<'src> TypeCompatible<'src> for StaticType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Concrete(other) if self == other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl std::fmt::Display for StaticType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive(t) => write!(f, "{t}"),
            Self::View(t) => write!(f, "{t}"),
            Self::Arr(t) => write!(f, "{t}"),
            Self::Ptr(t) => write!(f, "{t}"),
            Self::OrTy(t) => write!(f, "{t}"),
            Self::Rec(t) => write!(f, "{t}"),
            Self::Sup(t) => write!(f, "{t}"),
            Self::Sub(t) => write!(f, "{t}"),
            Self::Cat(t) => write!(f, "{t}"),
            Self::Fun(t) => write!(f, "{t}"),
        }
    }
}

impl<'src> From<ViewType<'src>> for StaticType<'src> {
    fn from(value: ViewType<'src>) -> Self {
        Self::from(Box::new(value))
    }
}
impl<'src> From<Box<ViewType<'src>>> for StaticType<'src> {
    fn from(value: Box<ViewType<'src>>) -> Self {
        Self::View(value)
    }
}
impl<'src> From<ArrayType<'src>> for StaticType<'src> {
    fn from(value: ArrayType<'src>) -> Self {
        Self::from(Box::new(value))
    }
}
impl<'src> From<Box<ArrayType<'src>>> for StaticType<'src> {
    fn from(value: Box<ArrayType<'src>>) -> Self {
        Self::Arr(value)
    }
}
impl<'src> From<Fallible<'src>> for StaticType<'src> {
    fn from(value: Fallible<'src>) -> Self {
        Self::from(Box::new(value))
    }
}
impl<'src> From<Box<Fallible<'src>>> for StaticType<'src> {
    fn from(value: Box<Fallible<'src>>) -> Self {
        Self::OrTy(value)
    }
}
impl<'src> From<RecType<'src>> for StaticType<'src> {
    fn from(value: RecType<'src>) -> Self {
        Self::Rec(value)
    }
}
impl<'src> From<SupType<'src>> for StaticType<'src> {
    fn from(value: SupType<'src>) -> Self {
        Self::Sup(value)
    }
}
impl<'src> From<SubType<'src>> for StaticType<'src> {
    fn from(value: SubType<'src>) -> Self {
        Self::from(Box::new(value))
    }
}
impl<'src> From<Box<SubType<'src>>> for StaticType<'src> {
    fn from(value: Box<SubType<'src>>) -> Self {
        Self::Sub(value)
    }
}
impl<'src> From<FunType<'src>> for StaticType<'src> {
    fn from(value: FunType<'src>) -> Self {
        Self::from(Box::new(value))
    }
}
impl<'src> From<Box<FunType<'src>>> for StaticType<'src> {
    fn from(value: Box<FunType<'src>>) -> Self {
        Self::Fun(value)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum TypeBound<'src> {
    /// Satisfies all of the conditions
    All(Vec<TypeBound<'src>>),

    /// Satisfies at least one of the conditions
    Any(Vec<TypeBound<'src>>),

    /// Does not satisfy the condition
    Not(Box<TypeBound<'src>>),

    /// Matches another type
    Exactly(AbstractType<'src>),

    /// Possesses a member (method or field) meeting the qualifications
    Has {
        /// Name of the member
        member: &'src str,
        /// Requirements the member must satisfy
        reqs: Option<Box<TypeBound<'src>>>,
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

impl std::fmt::Display for TypeBound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All(bounds) => {
                f.write_str("(")?;
                let mut has_prev = false;
                for bound in bounds {
                    if has_prev {
                        f.write_str(" & ")?;
                    }
                    has_prev = true;
                    bound.fmt(f)?;
                }
                f.write_str(")")
            }
            Self::Any(bounds) => {
                f.write_str("(")?;
                let mut has_prev = false;
                for bound in bounds {
                    if has_prev {
                        f.write_str(" | ")?;
                    }
                    has_prev = true;
                    bound.fmt(f)?;
                }
                f.write_str(")")
            }
            Self::Not(bound) => write!(f, "!{bound}"),
            Self::Exactly(ty) => write!(f, "{ty}"),
            Self::Has { member, reqs } => {
                write!(f, "has {member}")?;
                if let Some(req) = reqs {
                    write!(f, ": {req}")
                } else {
                    Ok(())
                }
            }
            Self::Shape(shape) => shape.fmt(f),
            Self::Supports { .. } => todo!(),
            Self::Satisfies(_) => todo!(),
        }
    }
}

/// A type with generic parameters
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct GenericType<'src> {
    pub ident: &'src str,
    pub params: Vec<GenericParam<'src>>,
}
impl std::fmt::Display for GenericType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { ident, params } = self;
        write!(f, "{ident}<")?;
        let mut has_prev = false;
        for param in params {
            if has_prev {
                f.write_str(", ")?;
            }
            write!(f, "{param}")?;
            has_prev = true;
        }
        f.write_str(">")
    }
}

impl<'src> TypeCompatible<'src> for GenericType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Generic(other) if self == other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}

/// A type that is not yet known
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct GenericParam<'src> {
    pub ident: &'src str,
    pub reqs: Box<TypeBound<'src>>,
}
impl std::fmt::Display for GenericParam<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { ident, .. } = self;
        write!(f, "{ident}")
    }
}

impl<'src> TypeCompatible<'src> for GenericParam<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        matches!(other, AbstractType::Param(other) if self == other)
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum AbstractType<'src> {
    /// All types should be this variant after monomorphization
    Concrete(StaticType<'src>),
    /// Select a concrete type based on where it's going; but it must meet these requirements
    Opaque(Box<TypeBound<'src>>),
    Generic(GenericType<'src>),
    Param(GenericParam<'src>),
}

impl TypeSize for AbstractType<'_> {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        match self {
            AbstractType::Concrete(t) => t.size(),
            _ => None, // TBD
        }
    }
}
impl<'src> TypeCompatible<'src> for AbstractType<'src> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        self == other
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone()
    }
}
impl std::fmt::Display for AbstractType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Concrete(t) => t.fmt(f),
            Self::Opaque(t) => write!(f, "opaque {t}"),
            Self::Generic(t) => t.fmt(f),
            Self::Param(t) => t.fmt(f),
        }
    }
}

impl<'src> From<StaticType<'src>> for AbstractType<'src> {
    fn from(value: StaticType<'src>) -> Self {
        Self::Concrete(value)
    }
}
impl<'src> From<GenericType<'src>> for AbstractType<'src> {
    fn from(value: GenericType<'src>) -> Self {
        Self::Generic(value)
    }
}
impl<'src> From<GenericParam<'src>> for AbstractType<'src> {
    fn from(value: GenericParam<'src>) -> Self {
        Self::Param(value)
    }
}

impl Default for AbstractType<'_> {
    fn default() -> Self {
        Self::Concrete(StaticType::default())
    }
}
