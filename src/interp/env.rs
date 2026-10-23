use crate::{
    analyzer::static_env::VarId,
    interp::value::{InterpVal, InterpValue},
};
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

impl<'src> InterpEnvironment<'src> {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn get_ptr(&self, var: InterpValue<'_, 'src>) -> Result<VarId, InterpVal<'src>> {
        match var {
            InterpValue::Prvalue(x) => Err(x),
            InterpValue::Rvalue(_) => todo!(),
            InterpValue::Lvalue(x) => self
                .vars
                .iter()
                .find(|(_, var)| std::ptr::eq(*var, x))
                .map(|(id, _)| *id)
                .ok_or_else(|| x.borrow().clone()),
        }
    }
}
