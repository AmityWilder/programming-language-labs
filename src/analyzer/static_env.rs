use super::{AnalysisBlockStmt, AnalysisFnDef, BlockStmt};
use crate::{
    Todo,
    analyzer::{
        GetType,
        types::{AbstractType, FunShape, StaticTypeSize, TypeCompatible, TypeSize, TypeSizeError},
    },
    grammar::ast::AstNodeFinal,
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
impl<'src> GetType<'src> for ConstantValue {
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
    Fun((FunShape<'src>, AnalysisBlockStmt<'src>)),
    Var(Option<AbstractType<'src>>, bool),
    Pvt(ConstantValue),
}

impl<'src> GetType<'src> for EnvItem<'src> {
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

pub struct StaticItem<'src> {
    pub key: EnvKey<'src>,
    pub val: EnvItem<'src>,
    pub node: Option<Box<dyn 'src + AstNodeFinal<'src>>>,
}

impl std::fmt::Debug for StaticItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticItem")
            .field("key", &self.key)
            .field("val", &self.val)
            .field("decl", &self.node.as_ref().map(|_| ()))
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct Scope<'src> {
    pub items: Vec<(EnvKey<'src>, VarId)>,
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

impl std::fmt::Display for OperandTypes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binary(lhs, rhs) => write!(f, "({lhs}, {rhs})"),
            // TBD: style
            Self::UnaryPost(lhs) => write!(f, "({lhs}, _)"),
            Self::UnaryPre(rhs) => write!(f, "(_, {rhs})"),
        }
    }
}

#[derive(Debug)]
pub struct OpOverload<'src> {
    pub operands: OperandTypes<'src>,
    pub output: AbstractType<'src>,
    pub fun: FunShape<'src>,
    pub def: AnalysisFnDef<'src>,
}

/// A unique [`EnvItem`] ID
pub type VarId = usize;

/// A unique [`Scope`] ID
pub type ScopeId = usize;

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
pub struct MemLoc {
    /// Item will stack atop this variable
    ///
    /// [`None`] if this is the first item in either a function or global scope
    pub atop: Option<VarId>,
    /// Where the variable will actually be stored in the runtime environment
    pub range: Option<Range<usize>>,
}

#[derive(Debug)]
pub struct DeclItem<'src> {
    loc: MemLoc,
    item: StaticItem<'src>,
}

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
#[derive(Debug)]
pub struct StaticEnvironment<'src> {
    /// Actual storage for the definitions, independent of visibility
    decls: Vec<DeclItem<'src>>,
    /// Blocks that get executed when their associated scope pops
    defers: Vec<(ScopeId, AnalysisBlockStmt<'src>)>,
    curr_scope_id: usize,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope<'src>,
    layers: Vec<Layer<'src>>,
}

impl<'src> StaticEnvironment<'src> {
    pub const fn new() -> Self {
        Self {
            decls: Vec::new(),
            defers: Vec::new(),
            curr_scope_id: 0,
            global: Scope::new(),
            layers: Vec::new(),
        }
    }

    pub fn get_decl(&self, id: VarId) -> Option<&StaticItem<'src>> {
        self.decls.get(id).map(|decl| &decl.item)
    }

    /// Returns [`None`] if not currently in a function
    const fn curr_layer(&self) -> Option<&Layer<'src>> {
        self.layers.last()
    }

    /// Returns [`None`] if not currently in a function
    const fn curr_layer_mut(&mut self) -> Option<&mut Layer<'src>> {
        self.layers.last_mut()
    }

    /// Returns an iterator over item IDs in order of newest to oldest
    fn visible_ids<'env>(
        global: &'env Scope<'src>,
        layers: &'env Vec<Layer<'src>>,
    ) -> impl DoubleEndedIterator<Item = VarId> {
        let g_it = global.items.iter();
        let l_it = layers
            .last()
            .into_iter()
            .flat_map(|layer| layer.scopes.iter().flat_map(|scope| &scope.items));

        g_it.chain(l_it).map(|(_, id)| *id).rev()
    }

    fn id_offset(num_items: usize) -> impl FnMut(VarId) -> usize {
        // items: 1, ids: [0]: [0]
        // items: 2, ids: [0]: [1]
        // items: 2, ids: [1, 0]: [0, 0]
        // items: 3, ids: [2, 0]: [0, 1]
        let mut prev = num_items;
        move |id| {
            let prev = std::mem::replace(&mut prev, id);
            assert!(
                prev > id,
                "IDs should be in strictly increasing order (and iterated in reverse (decreasing) order)\
                \n prev: {prev}\
                \n next: {id}\
                \n num_items: {num_items}"
            );
            #[expect(clippy::arithmetic_side_effects, reason = "asserted")]
            {
                prev - id - 1
            }
        }
    }

    fn search(&self) -> impl Iterator<Item = (VarId, &StaticItem<'src>)> {
        // Relies on the assumption that IDs increase monotonically along the stack
        let StaticEnvironment {
            decls: items,

            global,
            layers,
            ..
        } = self;
        let mut f = Self::id_offset(items.len());
        let mut items = items.iter().rev();

        Self::visible_ids(global, layers)
            .map(move |id| (id, f(id)))
            .map(move |(id, diff)| {
                (
                    id,
                    &items
                        .nth(diff)
                        .unwrap_or_else(|| panic!("ID should be valid\n ID: {id}\n diff: {diff}"))
                        .item,
                )
            })
    }

    fn search_mut(&mut self) -> impl Iterator<Item = (VarId, &mut StaticItem<'src>)> {
        // Relies on the assumption that IDs increase monotonically along the stack
        let StaticEnvironment {
            decls: items,
            global,
            layers,
            ..
        } = self;
        let mut items = items.iter_mut().rev();
        let mut f = Self::id_offset(items.len());

        Self::visible_ids(global, layers)
            .map(move |id| (id, f(id)))
            .map(move |(id, diff)| (id, &mut items.nth(diff).expect("ID should be valid").item))
    }

    pub fn lookup<Q>(&self, key: &Q) -> impl Iterator<Item = (VarId, &StaticItem<'src>)>
    where
        Q: ?Sized + EnvKeyEq,
    {
        self.search().filter(|(_, item)| key.is_matching(&item.key))
    }

    pub fn lookup_mut<Q>(&mut self, key: &Q) -> impl Iterator<Item = (VarId, &mut StaticItem<'src>)>
    where
        Q: ?Sized + EnvKeyEq,
    {
        self.search_mut()
            .filter(|(_, item)| key.is_matching(&item.key))
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
        self.curr_scope_id = self.curr_scope_id.strict_add(1);
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

    fn decl_item(
        &mut self,
        key: EnvKey<'src>,
        val: EnvItem<'src>,
        node: Option<Box<dyn 'src + AstNodeFinal<'src>>>,
    ) -> (VarId, &mut StaticItem<'src>) {
        // TBD: should variables, globals, constants, and functions all be stored together?
        // should constants even be stored at runtime at all?
        let atop = self
            .search()
            .find(|(_, item)| matches!(item.val, EnvItem::Var(_, _) | EnvItem::Pvt(_)))
            .map(|(id, _)| id);
        let id = self.decls.len();
        let decl = self.decls.push_mut(DeclItem {
            loc: MemLoc { atop, range: None },
            item: StaticItem { key, val, node },
        });
        let local_scope = if let Some(scope) = self.layers.last_mut().map(Layer::curr_scope_mut) {
            scope
        } else {
            &mut self.global
        };
        local_scope.items.push((decl.item.key, id));
        (id, &mut decl.item)
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
        node: Option<Box<dyn 'src + AstNodeFinal<'src>>>,
    ) -> Result<(VarId, &mut Option<AbstractType<'src>>, bool), TypeSizeError> {
        if let Some(ref ty) = ty
            && let Some(Err(e)) = ty.size()
        {
            return Err(e);
        }
        match self.decl_item(EnvKey::Ident(name), EnvItem::Var(ty, is_global), node) {
            (
                id,
                StaticItem {
                    key: _,
                    val: EnvItem::Var(ty, is_global),
                    node: _,
                },
            ) => Ok((id, ty, *is_global)),

            _ => unreachable!("pushed a var"),
        }
    }

    pub fn decl_pvt(
        &mut self,
        name: &'src str,
        value: ConstantValue,
        node: Option<Box<dyn 'src + AstNodeFinal<'src>>>,
    ) -> Result<(), TypeSizeError> {
        // TODO: apply size since we know it here
        self.decl_item(EnvKey::Ident(name), EnvItem::Pvt(value), node);
        Ok(())
    }

    pub fn decl_fun(
        &mut self,
        name: &'src str,
        shape: FunShape<'src>,
        def: AnalysisBlockStmt<'src>,
        node: Option<Box<dyn AstNodeFinal<'src>>>,
    ) {
        self.decl_item(EnvKey::Ident(name), EnvItem::Fun((shape, def)), node);
    }

    pub fn decl_op(
        &mut self,
        op: Operator,
        def: OpOverload<'src>,
        node: Option<Box<dyn 'src + AstNodeFinal<'src>>>,
    ) {
        self.decl_item(EnvKey::Op(op), EnvItem::Op(def), node);
    }

    pub fn decl_defer(&mut self, on_pop: AnalysisBlockStmt<'src>) {
        // TBD: how will scope pop be detected?
        self.defers.push((self.curr_scope_id, on_pop));
    }

    pub fn finalize_vars(&mut self) -> Result<(), Vec<VarId>> {
        let mut unresolved = Vec::new();
        for id in 0..self.decls.len() {
            let prev_end = if let Some(atop) = self.decls[id].loc.atop {
                if let Some(range) = self.decls[atop].loc.range {
                    range.end
                } else {
                    println!("ambiguous type position cascade");
                    continue;
                }
            } else {
                0
            };
            let size = match self.decls[id].item.val.get_type(self) {
                Some(VarType::Fun(_)) => {
                    // TODO: function sizes
                    0
                }
                Some(VarType::Abstract(AbstractType::Concrete(ty))) => {
                    // TODO: what about constants?
                    ty.size()
                        .expect("variable sizes must be static")
                        .map_err(|e| vec![todo!("error: {e}")])? // TODO: how do we report this?
                }
                _ => {
                    println!("error: type never resolved for {:?}", self.decls[id]);
                    unresolved.push(id);
                    continue;
                }
            };
            self.decls[id].loc.range = Some(Range {
                start: prev_end,
                end: prev_end + size,
            });
        }
        if unresolved.is_empty() {
            Ok(())
        } else {
            Err(unresolved)
        }
    }

    pub fn cement_ranges(mut self) -> Result<Box<[Range<usize>]>, impl IntoIterator<Item = VarId>> {
        self.finalize_vars().map(|()| {
            self.decls
                .iter()
                .map(|item| {
                    item.loc
                        .range
                        .expect("all ranges should be confirmed if finalize_vars didn't error")
                })
                .collect()
        })
    }
}
