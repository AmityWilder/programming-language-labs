use crate::{
    Todo,
    analyzer::{
        Expression, FnDef,
        types::{AbstractType, FunType, TypeCompatible},
    },
    scanner::token::value::Operator,
};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ConstantValue(Todo);

impl<'src> Expression<'src> for ConstantValue {
    type Output<'ty>
        = &'ty AbstractType<'src>
    where
        'src: 'ty;

    fn get_type<'ty>(&self, _env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarType<'src, 'ty> {
    Fun(&'ty FunType<'src>),
    Abstract(&'ty AbstractType<'src>),
}

impl<'src> TypeCompatible<'src> for VarType<'src, '_> {
    fn is_compatible(&self, other: &AbstractType<'_>) -> bool {
        match self {
            VarType::Fun(ty) => ty.is_compatible(other),
            VarType::Abstract(ty) => ty.is_compatible(other),
        }
    }

    fn to_abstract(&self) -> AbstractType<'src> {
        self.clone().into()
    }
}
impl<'src> From<VarType<'src, '_>> for AbstractType<'src> {
    fn from(value: VarType<'src, '_>) -> Self {
        match value {
            VarType::Fun(t) => t.to_abstract(),
            VarType::Abstract(t) => t.to_abstract(),
        }
    }
}
impl std::fmt::Display for VarType<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fun(t) => t.fmt(f),
            Self::Abstract(t) => t.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StaticVar<'src> {
    Fun(FunType<'src>), // TBD: should this be a shape, or a type?
    Var(Option<AbstractType<'src>>),
    Pvt(ConstantValue),
}

impl<'src> Expression<'src> for StaticVar<'src> {
    type Output<'ty>
        = VarType<'src, 'ty>
    where
        'src: 'ty,
        Self: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            StaticVar::Fun(x) => Some(VarType::Fun(x)),
            StaticVar::Var(x) => x.as_ref().map(VarType::Abstract),
            StaticVar::Pvt(x) => x.get_type(env).map(VarType::Abstract),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scope<'src> {
    pub vars: Vec<(&'src str, StaticVar<'src>)>,
}

impl<'src> Scope<'src> {
    pub const fn new() -> Self {
        Self { vars: Vec::new() }
    }

    /// Returns [`None`] if no item exists by the provided name
    pub fn get<Q>(&self, name: &Q) -> Option<&StaticVar<'src>>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.vars.iter().rfind(|(k, _)| name == *k).map(|(_, v)| v)
    }

    /// Returns [`None`] if no item exists by the provided name
    pub fn get_mut<Q>(&mut self, name: &Q) -> Option<&mut StaticVar<'src>>
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
        match &mut self.vars.push_mut((name, StaticVar::Var(ty))).1 {
            StaticVar::Var(ty) => ty,
            _ => unreachable!("pushed a Var::Run"),
        }
    }

    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) {
        self.vars.push((name, StaticVar::Pvt(value)));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperandTypes<'src> {
    Binary(AbstractType<'src>, AbstractType<'src>),
    UnaryPost(AbstractType<'src>),
    UnaryPre(AbstractType<'src>),
}

#[derive(Debug, PartialEq)]
pub struct OpOverload<'src> {
    pub op: Operator,
    pub operands: OperandTypes<'src>,
    pub output: AbstractType<'src>,
    pub def: FunType<'src>,
}

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
#[derive(Debug, Default)]
pub struct StaticEnvironment<'src> {
    global: Scope<'src>,
    stack: Vec<Scope<'src>>,
    overloads: Vec<(OpOverload<'src>, FnDef<'src>)>,
}

impl<'src> StaticEnvironment<'src> {
    pub const fn new() -> Self {
        Self {
            global: Scope::new(),
            stack: Vec::new(),
            overloads: Vec::new(),
        }
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn get<Q>(&self, name: &Q) -> Option<&StaticVar<'src>>
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
    pub fn get_mut<Q>(&mut self, name: &Q) -> Option<&mut StaticVar<'src>>
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
        self.stack.push_mut(Scope::new())
    }

    // TODO: needs to `Drop` all elements in memory (somehow)!!
    pub fn pop_scope(&mut self) {
        self.stack.pop();
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
        self.stack.last_mut().map(|scope| scope.decl_var(name, ty))
    }

    /// Declares at global scope if there is no local scope
    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) {
        self.stack
            .last_mut()
            .unwrap_or(&mut self.global)
            .decl_pvt(name, value);
    }

    pub fn decl_operator(&mut self, shape: OpOverload<'src>, def: FnDef<'src>) {
        // TODO: what if the overload already exists?
        self.overloads.push((shape, def));
    }

    /// Returns [`None`] if the overload isn't defined
    pub fn get_operator<'env>(
        &'env self,
        op: &Operator,
        operands: &OperandTypes<'src>,
    ) -> Option<&'env (OpOverload<'src>, FnDef<'src>)> {
        self.overloads
            .iter()
            .rfind(|(shape, _)| shape.op == *op && shape.operands == *operands)
    }
}
