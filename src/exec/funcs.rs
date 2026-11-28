use crate::exec::{FunId, tree::Func};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FuncTable {
    // TODO: compress IDs and make this a `Box<[Func]>`
    fns: HashMap<FunId, Func>,
}

impl std::ops::Index<FunId> for FuncTable {
    type Output = Func;

    #[track_caller]
    fn index(&self, index: FunId) -> &Self::Output {
        #[expect(clippy::indexing_slicing, reason = "caller responsibility")]
        &self.fns[&index]
    }
}

impl FromIterator<(FunId, Func)> for FuncTable {
    fn from_iter<I: IntoIterator<Item = (FunId, Func)>>(iter: I) -> Self {
        Self {
            fns: FromIterator::from_iter(iter),
        }
    }
}
