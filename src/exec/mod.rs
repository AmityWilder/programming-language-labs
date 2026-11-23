use crate::types::{byte, fail, flpt, sint, text, uint};

pub mod env;
pub mod tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EnvId(usize);

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
    Var(EnvId),
}

impl From<()> for Value {
    fn from((): ()) -> Self {
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

impl TryFrom<Value> for () {
    type Error = TypeMismatchError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.is_none().ok_or(TypeMismatchError)
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
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Type {
    #[default]
    None,
    Fail,
    Nevr,
    Bool,
    Byte,
    Uint,
    Sint,
    Flpt,
    Char,
    Text,
}
