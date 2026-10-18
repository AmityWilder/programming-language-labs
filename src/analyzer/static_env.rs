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

#[derive(Debug)]
pub enum EnvItem<'src> {
    Op(OpOverload<'src>),
    Fun(FunType<'src>), // TBD: should this be a shape, or a type?
    Var(Option<AbstractType<'src>>),
    Pvt(ConstantValue),
}

impl<'src> Expression<'src> for EnvItem<'src> {
    type Output<'ty>
        = VarType<'src, 'ty>
    where
        'src: 'ty,
        Self: 'ty;

    fn get_type<'ty>(&'ty self, env: &'ty StaticEnvironment<'src>) -> Option<Self::Output<'ty>> {
        match self {
            EnvItem::Op(x) => todo!(),
            EnvItem::Fun(x) => Some(VarType::Fun(x)),
            EnvItem::Var(x) => x.as_ref().map(VarType::Abstract),
            EnvItem::Pvt(x) => x.get_type(env).map(VarType::Abstract),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvKey<'src> {
    Ident(&'src str),
    Op(Operator),
}

pub trait EnvKeyEq {
    fn is_matching(&self, key: &EnvKey<'_>) -> bool;
}
impl<T: ?Sized + EnvKeyEq> EnvKeyEq for &T {
    fn is_matching(&self, key: &EnvKey<'_>) -> bool {
        (*self).is_matching(key)
    }
}
impl EnvKeyEq for str {
    fn is_matching(&self, key: &EnvKey<'_>) -> bool {
        matches!(key, EnvKey::Ident(name) if self == *name)
    }
}
impl EnvKeyEq for Operator {
    fn is_matching(&self, key: &EnvKey<'_>) -> bool {
        matches!(key, EnvKey::Op(op) if self == op)
    }
}

#[derive(Debug, Default)]
struct Scope<'src> {
    items: Vec<(EnvKey<'src>, EnvItem<'src>)>,
}

impl<'src> Scope<'src> {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn decl_var(
        &mut self,
        name: &'src str,
        ty: Option<AbstractType<'src>>,
    ) -> &mut Option<AbstractType<'src>> {
        match &mut self
            .items
            .push_mut((EnvKey::Ident(name), EnvItem::Var(ty)))
            .1
        {
            EnvItem::Var(ty) => ty,
            _ => unreachable!("pushed a Var::Run"),
        }
    }

    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) {
        self.items.push((EnvKey::Ident(name), EnvItem::Pvt(value)));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperandTypes<'src> {
    Binary(AbstractType<'src>, AbstractType<'src>),
    UnaryPost(AbstractType<'src>),
    UnaryPre(AbstractType<'src>),
}

#[derive(Debug)]
pub struct OpOverload<'src> {
    pub operands: OperandTypes<'src>,
    pub output: AbstractType<'src>,
    pub fun: FunType<'src>,
    pub def: FnDef<'src>,
}

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
#[derive(Debug, Default)]
pub struct StaticEnvironment<'src> {
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope<'src>,
    stack: Vec<Scope<'src>>,
}

impl<'src> StaticEnvironment<'src> {
    pub const fn new() -> Self {
        Self {
            global: Scope::new(),
            stack: Vec::new(),
        }
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn lookup<Q>(&self, key: &Q) -> impl DoubleEndedIterator<Item = &EnvItem<'src>>
    where
        Q: ?Sized + EnvKeyEq,
    {
        self.stack
            .iter()
            .flat_map(|scope| &scope.items)
            .chain(&self.global.items)
            .rev()
            .filter(|(k, _)| key.is_matching(k))
            .map(|(_, v)| v)
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn lookup_mut<Q>(&mut self, key: &Q) -> impl DoubleEndedIterator<Item = &mut EnvItem<'src>>
    where
        Q: ?Sized + EnvKeyEq,
    {
        self.stack
            .iter_mut()
            .flat_map(|scope| &mut scope.items)
            .chain(&mut self.global.items)
            .rev()
            .filter(|(k, _)| key.is_matching(k))
            .map(|(_, v)| v)
    }

    pub fn push_scope(&mut self) {
        self.stack.push(Scope::new());
    }

    // TODO: needs to `Drop` all elements in memory (somehow)!!
    pub fn pop_scope(&mut self) {
        self.stack.pop();
    }

    fn curr_scope(&mut self) -> &mut Scope<'src> {
        self.stack.last_mut().unwrap_or(&mut self.global)
    }

    /// Returns a mutable reference to the undetermined type
    ///
    /// `is_global`: Whether the variable can have its memory overwritten (does not affect visibility)
    pub fn decl_var(&mut self, name: &'src str, ty: Option<AbstractType<'src>>, is_global: bool) {
        self.curr_scope()
            .items
            .push((EnvKey::Ident(name), EnvItem::Var(ty)));
    }

    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) {
        self.curr_scope()
            .items
            .push((EnvKey::Ident(name), EnvItem::Pvt(value)));
    }

    /// UNDER CONSTRUCTION
    pub fn _decl_fun(&mut self, _shape: OpOverload<'src>, _def: FnDef<'src>) {
        todo!()
    }

    pub fn decl_op(&mut self, op: Operator, def: OpOverload<'src>) {
        // TODO: what if the overload already exists?
        self.curr_scope()
            .items
            .push((EnvKey::Op(op), EnvItem::Op(def)));
    }
}
