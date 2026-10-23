use crate::{
    analyzer::{
        InterpExpr,
        types::{FunShape, StaticType},
    },
    exec::error::Failure,
    types::{byte, fail, flpt, nevr, none, sint, text, uint},
};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    range::Range,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct NoneObject(pub none);

impl NoneObject {
    pub const fn new<'a>(value: &'a none) -> &'a Self {
        // SAFETY: NoneObject is a transparent wrapper for none
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut none) -> &'a mut Self {
        // SAFETY: NoneObject is a transparent wrapper for none
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for NoneObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<none>")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NevrObject(pub nevr);

impl NevrObject {
    pub const fn new<'a>(value: &'a nevr) -> &'a Self {
        // SAFETY: NevrObject is a transparent wrapper for nevr
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut nevr) -> &'a mut Self {
        // SAFETY: NevrObject is a transparent wrapper for nevr
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for NevrObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<nevr>")
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct FailObject(pub fail);

impl FailObject {
    pub const fn new<'a>(value: &'a fail) -> &'a Self {
        // SAFETY: FailObject is a transparent wrapper for fail
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut fail) -> &'a mut Self {
        // SAFETY: FailObject is a transparent wrapper for fail
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for FailObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<fail: {:?}>", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct BoolObject(pub bool);

impl BoolObject {
    pub const fn new<'a>(value: &'a bool) -> &'a Self {
        // SAFETY: BoolObject is a transparent wrapper for bool
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut bool) -> &'a mut Self {
        // SAFETY: BoolObject is a transparent wrapper for bool
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for BoolObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            true => "true",
            false => "fals",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ByteObject(pub byte);

impl ByteObject {
    pub const fn new<'a>(value: &'a byte) -> &'a Self {
        // SAFETY: ByteObject is a transparent wrapper for byte
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut byte) -> &'a mut Self {
        // SAFETY: ByteObject is a transparent wrapper for byte
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for ByteObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct UintObject(pub uint);

impl UintObject {
    pub const fn new<'a>(value: &'a uint) -> &'a Self {
        // SAFETY: UintObject is a transparent wrapper for uint
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut uint) -> &'a mut Self {
        // SAFETY: UintObject is a transparent wrapper for uint
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for UintObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}u", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct SintObject(pub sint);

impl SintObject {
    pub const fn new<'a>(value: &'a sint) -> &'a Self {
        // SAFETY: SintObject is a transparent wrapper for sint
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut sint) -> &'a mut Self {
        // SAFETY: SintObject is a transparent wrapper for sint
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for SintObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Default)]
#[repr(transparent)]
pub struct FlptObject(pub flpt);

impl FlptObject {
    pub const fn new<'a>(value: &'a flpt) -> &'a Self {
        // SAFETY: FlptObject is a transparent wrapper for flpt
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut flpt) -> &'a mut Self {
        // SAFETY: FlptObject is a transparent wrapper for flpt
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for FlptObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}f", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct CharObject(pub char);

impl CharObject {
    pub const fn new<'a>(value: &'a char) -> &'a Self {
        // SAFETY: CharObject is a transparent wrapper for char
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut char) -> &'a mut Self {
        // SAFETY: CharObject is a transparent wrapper for char
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for CharObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct TextObject(pub text);

impl TextObject {
    pub const fn new<'a>(value: &'a text) -> &'a Self {
        // SAFETY: TextObject is a transparent wrapper for text
        unsafe { std::mem::transmute(value) }
    }

    pub const fn new_mut<'a>(value: &'a mut text) -> &'a mut Self {
        // SAFETY: TextObject is a transparent wrapper for text
        unsafe { std::mem::transmute(value) }
    }
}

impl Debug for TextObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Clone, PartialEq)]
pub enum PrimitiveVal {
    /// Return type of a statement
    None(NoneObject),
    /// Return type of an endless loop - not constructable, only usable in analysis
    Nevr(NevrObject),
    /// Exception type
    Fail(FailObject),

    /// Boolean
    Bool(BoolObject),
    /// A single byte
    Byte(ByteObject),
    /// Unsigned integer
    Uint(UintObject),
    /// Signed integer
    Sint(SintObject),
    /// Floating point
    Flpt(FlptObject),
    /// Unicode character
    Char(CharObject),
    /// String
    Text(TextObject),
}

impl Default for PrimitiveVal {
    fn default() -> Self {
        Self::None(NoneObject(()))
    }
}

impl Debug for PrimitiveVal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::None(x) => x.fmt(f),
            Self::Nevr(x) => x.fmt(f),
            Self::Fail(x) => x.fmt(f),
            Self::Bool(x) => x.fmt(f),
            Self::Byte(x) => x.fmt(f),
            Self::Uint(x) => x.fmt(f),
            Self::Sint(x) => x.fmt(f),
            Self::Flpt(x) => x.fmt(f),
            Self::Char(x) => x.fmt(f),
            Self::Text(x) => x.fmt(f),
        }
    }
}

pub struct ViewVal<'brw, T> {
    pub data: &'brw [T],
}

impl<T: Debug> Debug for ViewVal<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        let mut has_prev = false;
        for item in self.data {
            if has_prev {
                f.write_str(", ")?;
            }
            has_prev = true;
            item.fmt(f)?;
        }
        f.write_str("]")
    }
}

// TBD: how is this different from `ViewVar`?
#[derive(Clone)]
pub struct ArrVal<T> {
    pub data: Box<[T]>,
}

impl<T> ArrVal<T> {
    pub const fn as_view(&self) -> ViewVal<'_, T> {
        ViewVal { data: &self.data }
    }
}

impl<T: Debug> Debug for ArrVal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_view().fmt(f)
    }
}

// TBD: how does it know what type it's pointing to?
#[derive(Clone)]
pub struct PtrVal {
    pub env_range: Range<usize>,
}

impl Debug for PtrVal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const PREFIX_LEN: usize = "0x".len();
        #[expect(
            clippy::as_conversions,
            reason = "usize doesn't implement a `from` impl even for integers smaller than it"
        )]
        const WIDTH: usize = usize::MAX.ilog(16) as usize + PREFIX_LEN;
        write!(f, "@{:#0WIDTH$X}", self.env_range.start)
    }
}

#[derive(Clone)]
pub enum OrVal<T, U> {
    Ok(T),
    Or(U),
}

pub type OrNone<T> = OrVal<T, NoneObject>;
pub type OrNevr<T> = OrVal<T, NevrObject>;
pub type OrFail<T> = OrVal<T, FailObject>;

impl<T: Debug, U: Debug> Debug for OrVal<T, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok(val) => val.fmt(f),
            Self::Or(val) => val.fmt(f),
        }
    }
}

#[derive(Clone)]
pub struct RecVal<'src> {
    pub name: &'src str,
    pub fields: HashMap<&'src str, InterpVal<'src>>,
}

impl Debug for RecVal<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct(self.name);
        for (key, val) in &self.fields {
            f.field(key, val);
        }
        f.finish()
    }
}

#[derive(Clone)]
pub struct SupVal<'src> {
    pub name: &'src str,
    pub variant_ty: StaticType<'src>,
    pub variant: InterpVal<'src>,
}

impl Debug for SupVal<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[repr(transparent)]
        struct SupInner<'a>(SupVal<'a>);

        impl<'a> SupInner<'a> {
            const fn new<'b>(value: &'b SupVal<'a>) -> &'b Self {
                // SAFETY: SupInner is a transparent wrapper for SupVar
                unsafe { std::mem::transmute(value) }
            }
        }

        impl Debug for SupInner<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_tuple(&self.0.variant_ty.to_string())
                    .field(&self.0.variant)
                    .finish()
            }
        }

        f.debug_tuple(&self.variant_ty.to_string())
            .field(SupInner::new(self))
            .finish()
    }
}

/// Identical to its inner type; the requirements are only enforced during static analysis.
#[derive(Clone)]
pub struct SubVal<'src> {
    pub base: InterpVal<'src>,
}

impl Debug for SubVal<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.base.fmt(f)
    }
}

#[derive(Clone)]
pub struct CatVal<'src> {
    pub discriminant: &'src str,
    pub value: InterpVal<'src>,
}

impl Debug for CatVal<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.discriminant)
    }
}

#[derive(Clone)]
pub struct FunVal {
    // TODO
}

// TBD: can a function debug its definition?
impl Debug for FunVal {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// An [`InterpVal`] that **can be** referenced
#[derive(Debug)]
pub enum InterpValue<'brw, 'src> {
    /// Has no accessible address
    ///
    /// Example: literals
    Prvalue(InterpVal<'src>),

    /// Has an address but which is no longer accessible
    ///
    /// Example: temporary
    Rvalue(&'brw InterpVal<'src>),

    /// Has an accessible address
    ///
    /// Example: variable name
    Lvalue(&'brw RefCell<InterpVal<'src>>),
}

impl<'brw, 'src> InterpValue<'brw, 'src> {
    pub fn to_ref(self) -> InterpRef<'brw, 'src> {
        match self {
            Self::Prvalue(prvalue) => InterpRef::Prvalue(prvalue),
            Self::Rvalue(rvalue) => InterpRef::Rvalue(rvalue),
            Self::Lvalue(lvalue) => InterpRef::Lvalue(lvalue.borrow()),
        }
    }

    pub fn to_mut(self) -> Option<RefMut<'brw, InterpVal<'src>>> {
        match self {
            Self::Prvalue(_) | Self::Rvalue(_) => None,
            Self::Lvalue(lvalue) => Some(lvalue.borrow_mut()),
        }
    }

    pub fn to_owned(self) -> InterpVal<'src> {
        match self {
            Self::Prvalue(x) => x.clone(),
            Self::Rvalue(x) => x.clone(),
            Self::Lvalue(x) => x.borrow().clone(),
        }
    }
}

/// An [`InterpVal`] that **is** referenced
#[derive(Debug)]
pub enum InterpRef<'brw, 'src> {
    Prvalue(InterpVal<'src>),
    Rvalue(&'brw InterpVal<'src>),
    Lvalue(Ref<'brw, InterpVal<'src>>),
}

impl<'src> std::ops::Deref for InterpRef<'_, 'src> {
    type Target = InterpVal<'src>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Prvalue(x) => x,
            Self::Rvalue(x) => x,
            Self::Lvalue(x) => x,
        }
    }
}

impl<'src> AsRef<InterpVal<'src>> for InterpRef<'_, 'src> {
    fn as_ref(&self) -> &InterpVal<'src> {
        self
    }
}

#[derive(Clone)]
pub enum InterpVal<'src> {
    Uninit(StaticType<'src>),

    /// Language-defined
    Primitive(PrimitiveVal),

    // /// Slice of another type
    // View(ViewVar<'vew, 'inr>), // TODO: is this a slice reference, or the actual slice?
    /// Array of another type
    Arr(ArrVal<Self>),

    /// Reference to another type
    Ptr(PtrVal),

    OrNone(Box<OrVal<Self, NoneObject>>),
    OrFail(Box<OrVal<Self, FailObject>>),
    OrNevr(Box<OrVal<Self, NevrObject>>),

    /// Structure
    Rec(RecVal<'src>),
    /// Union
    Sup(Box<SupVal<'src>>),
    /// Another type, but with runtime restrictions
    Sub(Box<SubVal<'src>>),
    /// Constant group
    Cat(Box<CatVal<'src>>),
    /// Callable
    Fun(FunVal),
}

macro_rules! convert_interp_primitive {
    ($($(#[$ctor:meta])? $Primitive:ident($Object:ident($Type:ty))),* $(,)?) => {$(
        impl TryFrom<InterpVal<'_>> for $Type {
            type Error = ();

            fn try_from(value: InterpVal<'_>) -> Result<Self, Self::Error> {
                match value {
                    $(#[$ctor])?
                    InterpVal::Primitive(PrimitiveVal::$Primitive($Object(value))) => Ok(value),
                    _ => Err(()),
                }
            }
        }

        impl<'a> TryFrom<&'a InterpVal<'_>> for &'a $Type {
            type Error = ();

            fn try_from(value: &'a InterpVal<'_>) -> Result<Self, Self::Error> {
                match value {
                    $(#[$ctor])?
                    InterpVal::Primitive(PrimitiveVal::$Primitive($Object(value))) => Ok(value),
                    _ => Err(()),
                }
            }
        }

        impl From<$Type> for InterpVal<'_> {
            fn from(value: $Type) -> Self {
                $(#[$ctor])?
                InterpVal::Primitive(PrimitiveVal::$Primitive($Object(value)))
            }
        }
    )*};
}

convert_interp_primitive! {
    None(NoneObject(none)),
    #[expect(unreachable_code, reason = "nevr is not constructable")]
    Nevr(NevrObject(nevr)),
    Fail(FailObject(fail)),
    Bool(BoolObject(bool)),
    Byte(ByteObject(byte)),
    Uint(UintObject(uint)),
    Sint(SintObject(sint)),
    Flpt(FlptObject(flpt)),
    Char(CharObject(char)),
    Text(TextObject(text)),
}

impl Debug for InterpVal<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninit(ty) => write!(f, "uninitialized {ty}"),
            Self::Primitive(val) => val.fmt(f),
            // Self::View(val) => val.fmt(f),
            Self::Arr(val) => val.fmt(f),
            Self::Ptr(val) => val.fmt(f),
            Self::OrNone(val) => val.fmt(f),
            Self::OrFail(val) => val.fmt(f),
            Self::OrNevr(val) => val.fmt(f),
            Self::Rec(val) => val.fmt(f),
            Self::Sup(val) => val.fmt(f),
            Self::Sub(val) => val.fmt(f),
            Self::Cat(val) => val.fmt(f),
            Self::Fun(val) => val.fmt(f),
        }
    }
}

// TBD: can this be baked in?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InterpLabel<'src> {
    pub name: &'src str,
}

pub type InterpBinaryOpFn =
    for<'brw, 'src> fn(
        InterpValue<'brw, 'src>,
        InterpValue<'brw, 'src>,
    ) -> std::result::Result<InterpValue<'brw, 'src>, Failure>;

pub type InterpUnaryOpFn =
    for<'brw, 'src> fn(
        InterpValue<'brw, 'src>,
    ) -> std::result::Result<InterpValue<'brw, 'src>, Failure>;

pub enum InterpOp<'src> {
    Binary(
        FunShape<'src>,
        InterpBinaryOpFn,
        InterpExpr<'src>,
        InterpExpr<'src>,
    ),
    UnaryPre(FunShape<'src>, InterpUnaryOpFn, InterpExpr<'src>),
    UnaryPost(FunShape<'src>, InterpUnaryOpFn, InterpExpr<'src>),
}

impl Debug for InterpOp<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary(o, fun, l, r) => write!(f, "{fun:?}({l:?}, {r:?}) -> {o:?}"),
            Self::UnaryPre(o, fun, x) => write!(f, "{fun:?}({x:?}) -> {o:?}"),
            Self::UnaryPost(o, fun, x) => write!(f, "({x:?}){fun:?} -> {o:?}"),
        }
    }
}
