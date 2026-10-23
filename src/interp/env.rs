use crate::{analyzer::static_env::VarId, interp::value::InterpVal};
use std::{cell::RefCell, collections::HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InterpVar<'src> {
    pub ident: &'src str,
    pub id: VarId,
}

#[derive(Debug, Clone)]
pub struct InterpEnvironment<'src> {
    pub vars: HashMap<VarId, RefCell<InterpVal<'src>>>,
}

impl InterpEnvironment<'_> {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}
