use crate::{
    Todo,
    analyzer::{Expression, Operands, Operation, types::AbstractType},
    scanner::token::value::Operator,
};
use std::collections::LinkedList;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ConstantValue(Todo);

impl<'src> Expression<'src> for ConstantValue {
    fn get_type<'ty>(&self, _env: &'ty StaticEnvironment<'src>) -> Option<&'ty AbstractType<'src>> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Var<'src> {
    Run(Option<AbstractType<'src>>),
    Pvt(ConstantValue),
}

impl<'src> Expression<'src> for Var<'src> {
    fn get_type<'ty>(
        &'ty self,
        env: &'ty StaticEnvironment<'src>,
    ) -> Option<&'ty AbstractType<'src>> {
        match self {
            Var::Run(x) => x.as_ref(),
            Var::Pvt(x) => x.get_type(env),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scope<'src> {
    pub vars: Vec<(&'src str, Var<'src>)>,
}

impl<'src> Scope<'src> {
    pub const fn new() -> Self {
        Self { vars: Vec::new() }
    }

    /// Returns [`None`] if no item exists by the provided name
    pub fn get<Q>(&self, name: &Q) -> Option<&Var<'src>>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.vars.iter().rfind(|(k, _)| name == *k).map(|(_, v)| v)
    }

    /// Returns [`None`] if no item exists by the provided name
    pub fn get_mut<Q>(&mut self, name: &Q) -> Option<&mut Var<'src>>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.vars
            .iter_mut()
            .rfind(|(k, _)| name == *k)
            .map(|(_, v)| v)
    }

    pub fn decl_var(
        &mut self,
        name: &'src str,
        ty: Option<AbstractType<'src>>,
    ) -> &mut Option<AbstractType<'src>> {
        match &mut self.vars.push_mut((name, Var::Run(ty))).1 {
            Var::Run(ty) => ty,
            _ => unreachable!("pushed a Var::Run"),
        }
    }

    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) {
        self.vars.push((name, Var::Pvt(value)));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperandTypes<'src> {
    Binary(AbstractType<'src>, AbstractType<'src>),
    UnaryPost(AbstractType<'src>),
    UnaryPre(AbstractType<'src>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpOverload<'src> {
    pub op: Operator,
    pub operands: OperandTypes<'src>,
    pub output: AbstractType<'src>,
}

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StaticEnvironment<'src> {
    global: Scope<'src>,
    stack: LinkedList<Scope<'src>>,
    overloads: Vec<OpOverload<'src>>,
}

impl<'src> StaticEnvironment<'src> {
    pub const fn new(init_overloads: Vec<OpOverload<'src>>) -> Self {
        Self {
            global: Scope::new(),
            stack: LinkedList::new(),
            overloads: init_overloads,
        }
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn get<Q>(&self, name: &Q) -> Option<&Var<'src>>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .or_else(|| self.global.get(name))
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn get_mut<Q>(&mut self, name: &Q) -> Option<&mut Var<'src>>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.stack
            .iter_mut()
            .rev()
            .find_map(|scope| scope.get_mut(name))
            .or_else(|| self.global.get_mut(name))
    }

    pub fn push_scope(&mut self) -> &mut Scope<'src> {
        self.stack.push_back_mut(Scope::new())
    }

    // TODO: needs to `Drop` all elements in memory (somehow)!!
    pub fn pop_scope(&mut self) {
        self.stack.pop_back();
    }

    /// Returns a mutable reference to the undetermined type
    pub fn decl_global_var(
        &mut self,
        name: &'src str,
        ty: Option<AbstractType<'src>>,
    ) -> &mut Option<AbstractType<'src>> {
        self.global.decl_var(name, ty)
    }

    /// Returns [`None`] if there is no local scope, or a mutable reference to the undetermined type if there is
    #[must_use]
    pub fn decl_local_var(
        &mut self,
        name: &'src str,
        ty: Option<AbstractType<'src>>,
    ) -> Option<&mut Option<AbstractType<'src>>> {
        self.stack.back_mut().map(|scope| scope.decl_var(name, ty))
    }

    /// Declares at global scope if there is no local scope
    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) {
        self.stack
            .back_mut()
            .unwrap_or(&mut self.global)
            .decl_pvt(name, value);
    }

    pub fn decl_overload(&mut self, def: OpOverload<'src>) {
        // TODO: what if the overload already exists?
        self.overloads.push(def);
    }

    /// Returns [`None`] if the overload isn't defined
    pub fn get_overload<'env>(&'env self, k: &Operation) -> Option<&'env OpOverload<'src>> {
        self.overloads.iter().find(|o| {
            o.op == k.op
                && match (&o.operands, &k.operands) {
                    (OperandTypes::Binary(l1, r1), Operands::Binary(l2, r2)) => {
                        l2.get_type(self).is_some_and(|l2| l1 == l2)
                            && r2.get_type(self).is_some_and(|r2| r1 == r2)
                    }

                    (OperandTypes::UnaryPre(a), Operands::UnaryPre(b))
                    | (OperandTypes::UnaryPost(a), Operands::UnaryPost(b)) => {
                        b.get_type(self).is_some_and(|b| a == b)
                    }

                    _ => false,
                }
        })
    }
}
