use crate::types::{byte, fail, flpt, none, sint, text, uint};

pub mod builtin_op;
pub mod env;
pub mod fmt;
pub mod funcs;
pub mod tree;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EnvId(pub usize);

impl std::fmt::Debug for EnvId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "var@{:X}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FunId(pub usize);

impl std::fmt::Debug for FunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "func@{:X}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LblId(pub usize);

impl std::fmt::Debug for LblId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".{:X}", self.0)
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlFlow<E, T, U = T, L = LblId> {
    Ok(T),
    Err(E),
    Skip { label: Option<L> },
    Break { label: Option<L>, value: U },
    Return { value: U },
}

impl<E, T, U, L> ControlFlow<E, T, U, L> {
    #[inline]
    pub fn ok(self) -> Result<T, ControlFlow<E, !, U, L>> {
        match self {
            Self::Ok(x) => Ok(x),
            Self::Err(e) => Err(ControlFlow::Err(e)),
            Self::Skip { label } => Err(ControlFlow::Skip { label }),
            Self::Break { label, value } => Err(ControlFlow::Break { label, value }),
            Self::Return { value } => Err(ControlFlow::Return { value }),
        }
    }

    #[inline]
    pub fn map<F, O>(self, f: F) -> ControlFlow<E, O, U, L>
    where
        F: FnOnce(T) -> O,
    {
        match self {
            Self::Ok(x) => ControlFlow::Ok(f(x)),
            Self::Err(e) => ControlFlow::Err(e),
            Self::Skip { label } => ControlFlow::Skip { label },
            Self::Break { label, value } => ControlFlow::Break { label, value },
            Self::Return { value } => ControlFlow::Return { value },
        }
    }
}

impl<E, T, U, L> std::ops::Residual<T> for ControlFlow<E, !, U, L> {
    type TryType = ControlFlow<E, T, U, L>;
}

impl<E, T, U, L> std::ops::FromResidual<ControlFlow<E, !, U, L>> for ControlFlow<E, T, U, L> {
    #[inline]
    fn from_residual(residual: ControlFlow<E, !, U, L>) -> Self {
        match residual {
            ControlFlow::Ok(_) => unreachable!(),
            ControlFlow::Err(e) => Self::Err(e),
            ControlFlow::Skip { label } => Self::Skip { label },
            ControlFlow::Break { label, value } => Self::Break { label, value },
            ControlFlow::Return { value } => Self::Return { value },
        }
    }
}

impl<E, T, U, L> std::ops::Try for ControlFlow<E, T, U, L> {
    type Output = T;
    type Residual = ControlFlow<E, !, U, L>;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        Self::Ok(output)
    }

    #[inline]
    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        use std::ops::ControlFlow::{Break, Continue};
        match self {
            Self::Ok(val) => Continue(val),
            Self::Err(e) => Break(ControlFlow::Err(e)),
            Self::Skip { label } => Break(ControlFlow::Skip { label }),
            Self::Break { label, value } => Break(ControlFlow::Break { label, value }),
            Self::Return { value } => Break(ControlFlow::Return { value }),
        }
    }
}

impl<T: Default, U, E, L> Default for ControlFlow<E, T, U, L> {
    fn default() -> Self {
        Self::Ok(T::default())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum Value {
    #[default]
    None,
    Fail(fail),
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Flpt(flpt),
    Char(char),
    Text(text),

    /// A reference
    Var(EnvId),

    /// Technically also a reference
    View {
        start: EnvId,
        count: usize,
    },
}

impl From<none> for Value {
    fn from((): none) -> Self {
        Self::None
    }
}
impl From<fail> for Value {
    fn from(value: fail) -> Self {
        Self::Fail(value)
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<byte> for Value {
    fn from(value: byte) -> Self {
        Self::Byte(value)
    }
}
impl From<uint> for Value {
    fn from(value: uint) -> Self {
        Self::Uint(value)
    }
}
impl From<sint> for Value {
    fn from(value: sint) -> Self {
        Self::Sint(value)
    }
}
impl From<flpt> for Value {
    fn from(value: flpt) -> Self {
        Self::Flpt(value)
    }
}
impl From<char> for Value {
    fn from(value: char) -> Self {
        Self::Char(value)
    }
}
impl From<text> for Value {
    fn from(value: text) -> Self {
        Self::Text(value)
    }
}
impl From<EnvId> for Value {
    fn from(value: EnvId) -> Self {
        Self::Var(value)
    }
}

#[derive(Debug)]
pub struct TypeMismatchError;

impl std::fmt::Display for TypeMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type mismatch")
    }
}

impl std::error::Error for TypeMismatchError {}

impl TryFrom<Value> for none {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.none().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for fail {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.fail().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for bool {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.bool().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for byte {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.byte().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for uint {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.uint().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for sint {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.sint().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for flpt {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.flpt().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for char {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.char().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for text {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.text().ok_or(TypeMismatchError)
    }
}
impl TryFrom<Value> for EnvId {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.var().ok_or(TypeMismatchError)
    }
}

impl Value {
    pub fn none(self) -> Option<none> {
        match self {
            Value::None => Some(()),
            _ => None,
        }
    }

    pub fn fail(self) -> Option<fail> {
        match self {
            Self::Fail(x) => Some(x),
            _ => None,
        }
    }

    pub fn bool(self) -> Option<bool> {
        match self {
            Self::Bool(x) => Some(x),
            _ => None,
        }
    }

    pub fn byte(self) -> Option<byte> {
        match self {
            Self::Byte(x) => Some(x),
            _ => None,
        }
    }

    pub fn uint(self) -> Option<uint> {
        match self {
            Self::Uint(x) => Some(x),
            _ => None,
        }
    }

    pub fn sint(self) -> Option<sint> {
        match self {
            Self::Sint(x) => Some(x),
            _ => None,
        }
    }

    pub fn flpt(self) -> Option<flpt> {
        match self {
            Self::Flpt(x) => Some(x),
            _ => None,
        }
    }

    pub fn char(self) -> Option<char> {
        match self {
            Self::Char(x) => Some(x),
            _ => None,
        }
    }

    pub fn text(self) -> Option<text> {
        match self {
            Self::Text(x) => Some(x),
            _ => None,
        }
    }

    pub fn var(self) -> Option<EnvId> {
        match self {
            Self::Var(x) => Some(x),
            _ => None,
        }
    }
}
