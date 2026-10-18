use crate::{
    Todo,
    analyzer::{
        BlockStmt, Expression, FnDef, Operation, Variable,
        types::{AbstractType, FunShape, StaticTypeSize, TypeCompatible, TypeSize, TypeSizeError},
    },
    scanner::token::value::Operator,
};
use std::range::Range;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
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
    Fun(&'ty FunShape<'src>),
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
    Fun((FunShape<'src>, BlockStmt<Operation<'src>, Variable<'src>>)),
    Var(Option<AbstractType<'src>>, bool),
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
            EnvItem::Op(OpOverload { fun, .. }) | EnvItem::Fun((fun, _)) => Some(VarType::Fun(fun)),
            EnvItem::Var(x, _) => x.as_ref().map(VarType::Abstract),
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

#[derive(Debug)]
pub struct ScopeItem<'src> {
    pub id: usize,
    pub key: EnvKey<'src>,
    pub val: EnvItem<'src>,
}

#[derive(Debug, Default)]
pub struct Scope<'src> {
    pub items: Vec<ScopeItem<'src>>,
}

impl Scope<'_> {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Default)]
pub struct Layer<'src> {
    pub root: Scope<'src>,
    pub scopes: Vec<Scope<'src>>,
}

impl<'src> Layer<'src> {
    pub const fn new() -> Self {
        Self {
            root: Scope::new(),
            scopes: Vec::new(),
        }
    }

    fn curr_scope(&self) -> &Scope<'src> {
        self.scopes.last().unwrap_or(&self.root)
    }

    fn curr_scope_mut(&mut self) -> &mut Scope<'src> {
        self.scopes.last_mut().unwrap_or(&mut self.root)
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
    pub fun: FunShape<'src>,
    pub def: FnDef<Operation<'src>, Variable<'src>>,
}

/// A unique [`EnvItem`] ID
pub type VarId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SizeUpdateError {
    AlreadySized(usize),
}

impl std::fmt::Display for SizeUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadySized(_) => write!(
                f,
                "tried to give a size to a variable that already has a size"
            ),
        }
    }
}

impl std::error::Error for SizeUpdateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemPos {
    //// Type unknown, should be stacked on top of the variable with the `atop` ID
    Unknown,
    /// Type known; waiting on the the variable with the `atop` ID variable to be sized,
    /// as this variable should stack on top of it
    Blocked(usize),
    /// Type and position known; if the variable is within a function,
    /// this will be **relative** to the stack pointer
    Fixed(Range<usize>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemLoc<'src> {
    /// Item will stack atop this variable
    ///
    /// [`None`] if this is the first item in either a function or global scope
    pub atop: Option<VarId>,
    /// Allows errors to be provided for the item even after it has popped
    pub key: EnvKey<'src>,
    /// Where the variable will actually be stored in the runtime environment
    pub pos: MemPos,
}

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
#[derive(Debug)]
pub struct StaticEnvironment<'src> {
    locs: Vec<MemLoc<'src>>,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope<'src>,
    layers: Vec<Layer<'src>>,
}

impl<'src> StaticEnvironment<'src> {
    pub const fn new() -> Self {
        Self {
            locs: Vec::new(),
            global: Scope::new(),
            layers: Vec::new(),
        }
    }

    /// Returns [`None`] if not currently in a function
    const fn curr_layer(&self) -> Option<&Layer<'src>> {
        self.layers.last()
    }

    /// Returns [`None`] if not currently in a function
    const fn curr_layer_mut(&mut self) -> Option<&mut Layer<'src>> {
        self.layers.last_mut()
    }

    fn curr_scope(&self) -> &Scope<'src> {
        if let Some(scope) = self.curr_layer().map(Layer::curr_scope) {
            scope
        } else {
            &self.global
        }
    }

    fn curr_scope_mut(&mut self) -> &mut Scope<'src> {
        if let Some(scope) = self.curr_layer_mut().map(Layer::curr_scope_mut) {
            scope
        } else {
            &mut self.global
        }
    }

    fn search(&self) -> impl DoubleEndedIterator<Item = &ScopeItem<'src>> {
        self.layers
            .last()
            .into_iter()
            .flat_map(|layer| layer.scopes.iter().flat_map(|scope| &scope.items))
            .chain(&self.global.items)
            .rev()
    }

    fn search_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut ScopeItem<'src>> {
        self.layers
            .last_mut()
            .into_iter()
            .flat_map(|layer| layer.scopes.iter_mut().flat_map(|scope| &mut scope.items))
            .chain(&mut self.global.items)
            .rev()
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn lookup<Q>(&self, key: &Q) -> impl DoubleEndedIterator<Item = &ScopeItem<'src>>
    where
        Q: ?Sized + EnvKeyEq,
    {
        self.search().filter(|item| key.is_matching(&item.key))
    }

    /// Returns [`None`] if no item exists with the provided name
    pub fn lookup_mut<Q>(
        &mut self,
        key: &Q,
    ) -> impl DoubleEndedIterator<Item = &mut ScopeItem<'src>>
    where
        Q: ?Sized + EnvKeyEq,
    {
        self.search_mut().filter(|item| key.is_matching(&item.key))
    }

    /// # Panics
    /// This method may panic if `id` is not a valid ID
    #[must_use = "represents success"]
    pub fn update_size(&mut self, id: VarId, size: usize) -> Result<(), SizeUpdateError> {
        let item = self.locs.get_mut(id).expect("ID should be valid");
        match item.pos {
            MemPos::Unknown => item.pos = MemPos::Blocked(size),
            MemPos::Blocked(n) => return Err(SizeUpdateError::AlreadySized(n)),
            MemPos::Fixed(range) => {
                return Err(SizeUpdateError::AlreadySized(
                    range.end.strict_sub(range.start),
                ));
            }
        }
        Ok(())
    }

    /// Entering a function definition
    pub fn push_layer(&mut self) {
        self.layers.push(Layer::new());
    }

    /// exiting a function definition
    #[must_use = "scopes and their items need to be dropped"]
    pub fn pop_layer(&mut self) -> Option<Layer<'src>> {
        self.layers.pop()
    }

    /// Entering a block
    ///
    /// Returns [`None`] if not in a function; since
    #[must_use = "indicates success"]
    pub fn push_scope(&mut self) -> Option<()> {
        self.curr_layer_mut()
            .map(|layer| layer.scopes.push(Scope::new()))
    }

    /// Exiting a block
    ///
    /// Remember to destruct local items when you call this!
    #[must_use = "items need to be dropped"]
    pub fn pop_scope(&mut self) -> Option<Scope<'src>> {
        self.curr_layer_mut().and_then(|layer| layer.scopes.pop())
    }

    fn decl_item(&mut self, key: EnvKey<'src>, val: EnvItem<'src>, size: Option<usize>) {
        // TBD: should variables, globals, constants, and functions all be stored together?
        // should constants even be stored at runtime at all?
        let atop = self
            .search()
            .find(|item| matches!(item.val, EnvItem::Var(_, _) | EnvItem::Pvt(_)))
            .map(|scope| scope.id);
        let id = self.locs.len();
        self.locs.push(MemLoc {
            atop,
            key,
            pos: size.map_or(MemPos::Unknown, MemPos::Blocked),
        });
        self.curr_scope_mut().items.push(ScopeItem { id, key, val });
    }

    /// `is_global`:
    /// - If true, the variable cannot have its location overwritten by scope changing
    /// - Can still push/pop from the stack
    ///
    /// TODO: This is the intent, but not the implementation!
    pub fn decl_var(
        &mut self,
        name: &'src str,
        ty: Option<AbstractType<'src>>,
        is_global: bool,
    ) -> Result<(), TypeSizeError> {
        let size = ty
            .as_ref()
            .and_then(|t| match t {
                AbstractType::Concrete(t) => t.size(),
                _ => None, // awaiting monomorphization
            })
            .transpose()?;
        self.decl_item(EnvKey::Ident(name), EnvItem::Var(ty, is_global), size);
        Ok(())
    }

    pub fn decl_pvt(&mut self, name: &'src str, value: ConstantValue) -> Result<(), TypeSizeError> {
        let size = value.static_size()?;
        self.decl_item(EnvKey::Ident(name), EnvItem::Pvt(value), Some(size));
        Ok(())
    }

    pub fn decl_fun(
        &mut self,
        name: &'src str,
        shape: FunShape<'src>,
        def: BlockStmt<Operation<'src>, Variable<'src>>,
    ) {
        self.decl_item(
            EnvKey::Ident(name),
            EnvItem::Fun((shape, def)),
            None, // TBD
        );
    }

    pub fn decl_op(&mut self, op: Operator, def: OpOverload<'src>) {
        self.decl_item(EnvKey::Op(op), EnvItem::Op(def), None /* TBD */);
    }

    pub fn cement_ranges<'loc>(
        &'loc mut self,
    ) -> Result<Box<[Range<usize>]>, impl Iterator<Item = (VarId, &'loc EnvKey<'src>)>> {
        let mut ranges: Vec<Range<usize>> = Vec::with_capacity(self.locs.len());
        let mut locs = self.locs.iter().enumerate();
        while let Some((id, loc)) = locs.next() {
            let range = match loc.pos {
                MemPos::Unknown => {
                    if let EnvKey::Op(_) = loc.key {
                        // TODO: where are functions stored?
                        let start = ranges.last().map_or(0, |range| range.end);
                        Range { start, end: start }
                    } else {
                        return Err(std::iter::once((id, &loc.key)).chain(
                            locs.filter(|(_, loc)| matches!(loc.pos, MemPos::Unknown))
                                .map(|(id, loc)| (id, &loc.key)),
                        ));
                    }
                }

                MemPos::Blocked(size) => {
                    let start = loc
                        .atop
                        // relative range when in function calls
                        .map_or(0, |atop| {
                            assert!(atop < id, "`atop` should always be an earlier element");
                            ranges
                                .get(atop)
                                .expect("`atop` should be present in `ranges`")
                                .end
                        });

                    Range {
                        start,
                        end: start.strict_add(size),
                    }
                }

                MemPos::Fixed(range) => range,
            };
            ranges.push(range);
        }
        Ok(ranges.into_boxed_slice())
    }
}
