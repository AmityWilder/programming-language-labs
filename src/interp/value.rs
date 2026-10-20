use crate::{
    analyzer::{Expr, types::StaticType},
    exec::error::Failure,
    types::*,
};
use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    range::Range,
};

pub trait LangVarCast {
    type Upcast: LangVar;
    fn cast_into(self) -> Self::Upcast;
    fn cast_from(value: &dyn LangVar) -> Option<&Self>;
    fn cast_from_mut(value: &mut dyn LangVar) -> Option<&mut Self>;
}

macro_rules! impl_lang_var_cast {
    ($([$fn:ident, $fn_mut:ident] => $Variant:ident($Type:ty, $(#[$ctor:meta])* $Object:ident);)*) => {
        pub trait LangVar: Debug {$(
            fn $fn(&self) -> Option<&$Type> {
                None
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                None
            }
        )*}

        $(impl LangVarCast for $Type {
            type Upcast = $Object;

            fn cast_into(self) -> Self::Upcast {
                $(#[$ctor])*
                $Object(self)
            }

            fn cast_from(value: &dyn LangVar) -> Option<&Self> {
                value.$fn()
            }

            fn cast_from_mut(value: &mut dyn LangVar) -> Option<&mut Self> {
                value.$fn_mut()
            }
        }
        impl LangVar for $Object {
            fn $fn(&self) -> Option<&$Type> {
                Some(&self.0)
            }

            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                Some(&mut self.0)
            }
        })*

        impl LangVar for PrimitiveVar {$(
            fn $fn(&self) -> Option<&$Type> {
                match self {
                    Self::$Variant(x) => Some(&x.0),
                    _ => None,
                }
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                match self {
                    Self::$Variant(x) => Some(&mut x.0),
                    _ => None,
                }
            }
        )*}

        impl<T: LangVar> LangVar for OrVar<'_, T> {$(
            fn $fn(&self) -> Option<&$Type> {
                match self {
                    Self::Ok(x) => x.$fn(),
                    Self::Or(x) => x.$fn(),
                }
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                match self {
                    Self::Ok(x) => x.$fn_mut(),
                    Self::Or(x) => x.$fn_mut(),
                }
            }
        )*}

        impl LangVar for SupVar<'_, '_> {$(
            fn $fn(&self) -> Option<&$Type> {
                self.variant.$fn()
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                self.variant.$fn_mut()
            }
        )*}

        impl LangVar for SubVar<'_> {$(
            fn $fn(&self) -> Option<&$Type> {
                self.base.$fn()
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                self.base.$fn_mut()
            }
        )*}

        impl LangVar for CatVar<'_, '_> {$(
            fn $fn(&self) -> Option<&$Type> {
                self.value.$fn()
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                self.value.$fn_mut()
            }
        )*}

        impl LangVar for InterpVar<'_, '_> {$(
            fn $fn(&self) -> Option<&$Type> {
                match self {
                    Self::Primitive(x) => x.$fn(),
                    // Self::View(x) => x.$fn(),
                    Self::Arr(x) => x.$fn(),
                    Self::Ptr(x) => x.$fn(),
                    Self::OrNone(x) => x.$fn(),
                    Self::OrFail(x) => x.$fn(),
                    Self::OrNevr(x) => x.$fn(),
                    Self::Rec(x) => x.$fn(),
                    Self::Sup(x) => x.$fn(),
                    Self::Sub(x) => x.$fn(),
                    Self::Cat(x) => x.$fn(),
                    Self::Fun(x) => x.$fn(),
                }
            }
            fn $fn_mut(&mut self) -> Option<&mut $Type> {
                match self {
                    Self::Primitive(x) => x.$fn_mut(),
                    // Self::View(x) => x.$fn_mut(),
                    Self::Arr(x) => x.$fn_mut(),
                    Self::Ptr(x) => x.$fn_mut(),
                    Self::OrNone(x) => x.$fn_mut(),
                    Self::OrFail(x) => x.$fn_mut(),
                    Self::OrNevr(x) => x.$fn_mut(),
                    Self::Rec(x) => x.$fn_mut(),
                    Self::Sup(x) => x.$fn_mut(),
                    Self::Sub(x) => x.$fn_mut(),
                    Self::Cat(x) => x.$fn_mut(),
                    Self::Fun(x) => x.$fn_mut(),
                }
            }
        )*}
    };
}

impl_lang_var_cast! {
    [as_none, as_mut_none] => None(none, NoneObject);
    [as_nevr, as_mut_nevr] => Nevr(nevr, #[expect(unreachable_code, reason = "not allowed to be constructed")] NevrObject);
    [as_fail, as_mut_fail] => Fail(fail, FailObject);
    [as_bool, as_mut_bool] => Bool(bool, BoolObject);
    [as_byte, as_mut_byte] => Byte(byte, ByteObject);
    [as_uint, as_mut_uint] => Uint(uint, UintObject);
    [as_sint, as_mut_sint] => Sint(sint, SintObject);
    [as_flpt, as_mut_flpt] => Flpt(flpt, FlptObject);
    [as_char, as_mut_char] => Char(char, CharObject);
    [as_text, as_mut_text] => Text(text, TextObject);
}

impl<'inr> dyn 'inr + LangVar {
    pub fn downcast_to<T: LangVarCast>(&self) -> Option<&T> {
        T::cast_from(self)
    }

    pub fn downcast_to_mut<T: LangVarCast>(&mut self) -> Option<&mut T> {
        T::cast_from_mut(self)
    }
}

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
pub enum PrimitiveVar {
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

impl Default for PrimitiveVar {
    fn default() -> Self {
        Self::None(NoneObject(()))
    }
}

impl Debug for PrimitiveVar {
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

pub struct ViewVar<'vew, 'inr: 'vew> {
    pub data: &'vew [Box<dyn 'inr + LangVar>],
}

impl Debug for ViewVar<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        let mut has_prev = false;
        for item in self.data {
            if has_prev {
                f.write_str(", ")?;
            }
            has_prev = true;
            (**item).fmt(f)?;
        }
        f.write_str("]")
    }
}

impl LangVar for ViewVar<'_, '_> {}

// TBD: how is this different from `ViewVar`?
pub struct ArrVar<'inr> {
    pub data: Box<[Box<dyn 'inr + LangVar>]>,
}

impl<'inr> ArrVar<'inr> {
    pub const fn as_view(&self) -> ViewVar<'inr, '_> {
        ViewVar { data: &self.data }
    }
}

impl Debug for ArrVar<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_view().fmt(f)
    }
}

impl LangVar for ArrVar<'_> {}

// TBD: how does it know what type it's pointing to?
pub struct PtrVar {
    pub env_range: Range<usize>,
}

impl Debug for PtrVar {
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

impl LangVar for PtrVar {}

pub enum OrVar<'inr, T> {
    Ok(Box<dyn 'inr + LangVar>),
    Or(T),
}

pub type OrNone<'inr> = OrVar<'inr, NoneObject>;
pub type OrNevr<'inr> = OrVar<'inr, NevrObject>;
pub type OrFail<'inr> = OrVar<'inr, FailObject>;

impl<T: Debug> Debug for OrVar<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok(val) => val.fmt(f),
            Self::Or(val) => val.fmt(f),
        }
    }
}

pub struct RecVar<'src, 'inr> {
    pub name: &'src str,
    pub fields: HashMap<&'src str, Box<dyn 'inr + LangVar>>,
}

impl Debug for RecVar<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct(self.name);
        for (key, val) in &self.fields {
            f.field(key, val);
        }
        f.finish()
    }
}

impl LangVar for RecVar<'_, '_> {}

pub struct SupVar<'src, 'inr> {
    pub name: &'src str,
    pub variant_ty: StaticType<'src>,
    pub variant: Box<dyn 'inr + LangVar>,
}

impl Debug for SupVar<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[repr(transparent)]
        struct SupInner<'a, 'b>(SupVar<'a, 'b>);

        impl<'a, 'b> SupInner<'a, 'b> {
            const fn new<'c>(value: &'c SupVar<'a, 'b>) -> &'c Self {
                // SAFETY: SupInner is a transparent wrapper for SupVar
                unsafe { std::mem::transmute(value) }
            }
        }

        impl Debug for SupInner<'_, '_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_tuple(&self.0.variant_ty.to_string())
                    .field(&*self.0.variant)
                    .finish()
            }
        }

        f.debug_tuple(&self.variant_ty.to_string())
            .field(SupInner::new(self))
            .finish()
    }
}

/// Identical to its inner type; the requirements are only enforced during static analysis.
pub struct SubVar<'inr> {
    pub base: Box<dyn 'inr + LangVar>,
}

impl Debug for SubVar<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.base.fmt(f)
    }
}

pub struct CatVar<'src, 'inr> {
    pub discriminant: &'src str,
    pub value: Box<dyn 'inr + LangVar>,
}

impl Debug for CatVar<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.discriminant)
    }
}

pub struct FunVar {
    // TODO
}

// TBD: can a function debug its definition?
impl Debug for FunVar {
    fn fmt(&self, _f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl LangVar for FunVar {}

pub enum InterpVar<'src, 'inr> {
    /// Language-defined
    Primitive(PrimitiveVar),

    // /// Slice of another type
    // View(ViewVar<'vew, 'inr>), // TODO: is this a slice reference, or the actual slice?
    /// Array of another type
    Arr(ArrVar<'inr>),

    /// Reference to another type
    Ptr(PtrVar),

    OrNone(OrVar<'inr, NoneObject>),
    OrFail(OrVar<'inr, FailObject>),
    OrNevr(OrVar<'inr, NevrObject>),

    /// Structure
    Rec(RecVar<'src, 'inr>),
    /// Union
    Sup(SupVar<'src, 'inr>),
    /// Another type, but with runtime restrictions
    Sub(SubVar<'inr>),
    /// Constant group
    Cat(CatVar<'src, 'inr>),
    /// Callable
    Fun(FunVar),
}

impl Debug for InterpVar<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
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

pub type InterpBinaryOpFn = for<'inr> fn(
    &(dyn 'inr + LangVar),
    &(dyn 'inr + LangVar),
) -> std::result::Result<Box<dyn 'inr + LangVar>, Failure>;

pub type InterpUnaryOpFn =
    for<'inr> fn(&(dyn 'inr + LangVar)) -> std::result::Result<Box<dyn 'inr + LangVar>, Failure>;

pub enum InterpOp<'src, 'linr, 'rinr> {
    Binary(
        StaticType<'src>,
        InterpBinaryOpFn,
        Expr<Self, InterpVar<'src, 'linr>>,
        Expr<Self, InterpVar<'src, 'rinr>>,
    ),
    UnaryPre(
        StaticType<'src>,
        InterpUnaryOpFn,
        Expr<Self, InterpVar<'src, 'rinr>>,
    ),
    UnaryPost(
        StaticType<'src>,
        InterpUnaryOpFn,
        Expr<Self, InterpVar<'src, 'linr>>,
    ),
}

impl Debug for InterpOp<'_, '_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binary(o, fun, l, r) => write!(f, "{fun:?}({l:?}, {r:?}) -> {o:?}"),
            Self::UnaryPre(o, fun, x) => write!(f, "{fun:?}({x:?}) -> {o:?}"),
            Self::UnaryPost(o, fun, x) => write!(f, "({x:?}){fun:?} -> {o:?}"),
        }
    }
}
