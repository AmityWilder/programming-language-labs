use crate::{
    analyzer::{
        // GetType,
        // static_env::old::StaticEnvironment,
        types::{StaticTypeSize, TypeSize, TypeSizeError},
    },
    todo_types::Todo,
};

pub mod morph;
pub mod scope;

/// A unique ID for distinguishing items that may have the same identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub(self) usize);

impl ItemId {
    pub const fn get(self) -> usize {
        self.0
    }
}

pub type TypeId = ItemId;
pub type FuncId = ItemId;
pub type VarId = ItemId;

impl std::fmt::Debug for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ItemId({:#X})", self.0)
    }
}

impl std::fmt::Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "item.{:#x}", self.0)
    }
}

/// A unique ID for modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModId(pub(self) usize);

pub type LibId = ModId;

impl std::fmt::Display for ModId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mod.{:#x}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantValue(Todo);

impl TypeSize for ConstantValue {
    fn size(&self) -> Option<Result<usize, TypeSizeError>> {
        Some(self.static_size())
    }
}
impl StaticTypeSize for ConstantValue {
    fn static_size(&self) -> Result<usize, TypeSizeError> {
        todo!("constants must be statically sized")
    }
}
// impl<'src> GetType<'src> for ConstantValue {
//     type Output<'ty>
//         = &'ty AbstractType<'src>
//     where
//         'src: 'ty;

//     fn get_type<'ty>(&self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
//         todo!()
//     }
// }
