use crate::{
    Todo,
    analyzer::{
        GetType,
        static_env::old::StaticEnvironment,
        types::{AbstractType, StaticTypeSize, TypeSize, TypeSizeError},
    },
};

pub mod morph;
#[deprecated = "changing to multi-pass analyzer"]
pub mod old;
pub mod vis;

/// A unique ID for distinguishing items that may have the same identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub(self) usize);

impl std::fmt::Display for VarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
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
impl<'src> GetType<'src> for ConstantValue {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        todo!()
    }
}
