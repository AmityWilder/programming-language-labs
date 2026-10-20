//! UNDER CONSTRUCTION

use crate::Todo;
use std::fmt::Debug;

trait LangVar: Debug {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NoneObject(crate::types::none);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NevrObject(crate::types::nevr);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailObject(crate::types::fail);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BoolObject(crate::types::bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ByteObject(crate::types::byte);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UintObject(crate::types::uint);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SintObject(crate::types::sint);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct FlptObject(crate::types::flpt);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CharObject(crate::types::char);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TextObject(crate::types::text);

#[derive(Debug, Clone, PartialEq)]
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

pub struct ViewVar<'vew, 'inr: 'vew> {
    pub data: &'vew [Box<dyn 'inr + LangVar>],
}

impl Debug for ViewVar<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        let mut has_prev = false;
        for item in self.data {
            if has_prev {
                f.write_str(", ")?;
            }
            has_prev = true;
            Debug::fmt(&**item, f);
        }
        f.write_str("]")
    }
}

// TBD: how is this different from `ViewVar`?
pub struct ArrVar<'inr> {
    pub data: Box<[Box<dyn 'inr + LangVar>]>,
}

impl<'inr> ArrVar<'inr> {
    fn as_view(&self) -> ViewVar<'inr, '_> {
        ViewVar {
            data: self.data.as_ref(),
        }
    }
}

impl Debug for ArrVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_view().fmt(f)
    }
}

// TBD: this doesnt feel much like a pointer...
pub struct PtrVar<'inr> {
    pub data: Box<dyn 'inr + LangVar>,
}

pub enum OrVar<'inr, T> {
    Ok(Box<dyn 'inr + LangVar>),
    Or(T),
}

impl<T: Debug> Debug for OrVar<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok(val) => val.fmt(f),
            Self::Or(val) => val.fmt(f),
        }
    }
}

pub struct RecVar {}

pub enum InterpVar<'vew, 'inr: 'vew> {
    /// Language-defined
    Primitive(PrimitiveVar),

    /// Slice of another type
    View(ViewVar<'vew, 'inr>),
    /// Array of another type
    Arr(ArrVar<'inr>),

    /// Reference to another type
    Ptr(PtrVar<'inr>),

    OrNone(OrVar<'inr, NoneObject>),
    OrFail(OrVar<'inr, FailObject>),
    OrNevr(OrVar<'inr, NevrObject>),

    /// Structure
    Rec(Todo),
    /// Union
    Sup(Todo),
    /// Another type, but with runtime restrictions
    Sub(Todo),
    /// Constant group
    Cat(Todo),
    /// Callable
    Fun(Todo),
}

pub enum InterpOp {
    // Binary(
    //     #[cfg(debug_assertions)] StaticType<'src>,
    //     #[cfg(not(debug_assertions))] (),
    //     BinaryOpFn,
    //     // output size
    //     usize,
    //     Expr<Self, RuntimeVar<'src>>,
    //     Expr<Self, RuntimeVar<'src>>,
    // ),
    // UnaryPre(
    //     #[cfg(debug_assertions)] StaticType<'src>,
    //     #[cfg(not(debug_assertions))] (),
    //     UnaryOpFn,
    //     // output size
    //     usize,
    //     Expr<Self, RuntimeVar<'src>>,
    // ),
    // UnaryPost(
    //     #[cfg(debug_assertions)] StaticType<'src>,
    //     #[cfg(not(debug_assertions))] (),
    //     UnaryOpFn,
    //     // output size
    //     usize,
    //     Expr<Self, RuntimeVar<'src>>,
    // ),
}
