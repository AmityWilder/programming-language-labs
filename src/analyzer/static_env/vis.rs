use crate::{
    analyzer::{
        static_env::VarId,
        types::{AbstractType, FunShape},
    },
    grammar::ast::AstNodeFinal,
    scanner::token::value::Operator,
};
use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
};

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Default)]
pub struct BuiltinOps<'src> {
    pub shape: FunShape<'src>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub enum BuiltinOpParamTypes {
    Binary {
        lhs: AbstractType<'static>,
        rhs: AbstractType<'static>,
    },
    UnaryPre {
        rhs: AbstractType<'static>,
    },
    UnaryPost {
        lhs: AbstractType<'static>,
    },
}

impl BuiltinOpParamTypes {
    pub const fn lhs(&self) -> Option<&AbstractType<'static>> {
        if let Self::Binary { lhs, .. } | Self::UnaryPost { lhs } = self {
            Some(lhs)
        } else {
            None
        }
    }

    pub const fn rhs(&self) -> Option<&AbstractType<'static>> {
        if let Self::Binary { rhs, .. } | Self::UnaryPre { rhs } = self {
            Some(rhs)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct BuiltinOpSignature {
    pub operands: BuiltinOpParamTypes,
    pub ret: AbstractType<'static>,
}

#[derive(Debug)]
pub struct Item<'ast, 'src> {
    pub node: Box<dyn 'ast + AstNodeFinal<'src>>,

    /// If the item is a function or overloads the call-operator, this will be [`Some`].
    pub shape: Option<FunShape<'src>>,
}

impl std::hash::Hash for Item<'_, '_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.shape.hash(state);
    }
}

#[derive(Debug, Default)]
pub struct VisEnvironment<'ast, 'src> {
    builtin: HashMap<Operator, HashSet<BuiltinOpSignature>>,
    items: Vec<Item<'ast, 'src>>,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope<'src>,
    layers: Vec<Layer<'src>>,
}

impl<'ast, 'src> VisEnvironment<'ast, 'src> {
    pub const fn new(builtin: HashMap<Operator, HashSet<BuiltinOpSignature>>) -> Self {
        Self {
            builtin,
            items: Vec::new(),
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

    /// Returns an iterator over item IDs in order of newest to oldest
    fn visible_ids<'env>(
        global: &'env Scope<'src>,
        layers: &'env Vec<Layer<'src>>,
    ) -> impl DoubleEndedIterator<Item = (&'src str, VarId)> {
        let g_it = global.items.iter();
        let l_it = layers
            .last()
            .into_iter()
            .flat_map(|layer| layer.scopes.iter().flat_map(|scope| &scope.items));

        g_it.chain(l_it).copied().rev()
    }

    /// Converts an ID to the number of steps away from the previous one (assuming reverse order)
    fn id_offset(num_items: usize) -> impl FnMut(VarId) -> usize {
        // items: 1, ids: [0]: [0]
        // items: 2, ids: [0]: [1]
        // items: 2, ids: [1, 0]: [0, 0]
        // items: 3, ids: [2, 0]: [0, 1]
        let mut prev = VarId(num_items);
        move |id| {
            let diff = prev.0.checked_sub(id.0).and_then(NonZero::new).unwrap_or_else(|| {
                panic!(
                    "IDs should be in strictly increasing order and iterated in reverse (decreasing) order\
                    \n      prev: {prev}\
                    \n      next: {id}\
                    \n num_items: {num_items}"
                );
            }).get() - 1;
            prev = id;
            diff
        }
    }

    /// An iterator over all visible items in order of most recent to least recent declaration.
    ///
    /// Does not include builtin items.
    fn search(&self) -> impl Iterator<Item = (&'src str, VarId, &Item<'ast, 'src>)> {
        // Relies on the assumption that IDs increase monotonically along the stack
        let Self {
            items,
            global,
            layers,
            ..
        } = self;
        let mut f = Self::id_offset(items.len());
        let mut items = items.iter().rev();

        Self::visible_ids(global, layers)
            .map(move |(k, id)| (k, id, items.nth(f(id)).expect("ID should be valid")))
    }

    /// An iterator over all visible items in order of most recent to least recent declaration.
    ///
    /// Does not include builtin items.
    fn search_mut(&mut self) -> impl Iterator<Item = (&'src str, VarId, &mut Item<'ast, 'src>)> {
        // Relies on the assumption that IDs increase monotonically along the stack
        let Self {
            items,
            global,
            layers,
            ..
        } = self;
        let mut items = items.iter_mut().rev();
        let mut f = Self::id_offset(items.len());

        Self::visible_ids(global, layers)
            .map(move |(k, id)| (k, id, items.nth(f(id)).expect("ID should be valid")))
    }

    /// Finds visible items matching the key in order of most recent to least recent declaration.
    ///
    /// Does not include builtin items.
    pub fn lookup<Q>(&self, key: &Q) -> impl Iterator<Item = (&'src str, VarId, &Item<'ast, 'src>)>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.search().filter(|&(k, _, _)| *key == *k)
    }

    /// Finds visible items matching the key in order of most recent to least recent declaration.
    ///
    /// Does not include builtin items.
    pub fn lookup_mut<Q>(
        &mut self,
        key: &Q,
    ) -> impl Iterator<Item = (&'src str, VarId, &mut Item<'ast, 'src>)>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.search_mut().filter(|&(k, _, _)| *key == *k)
    }

    /// Entering a function definition
    pub fn push_layer(&mut self) {
        self.layers.push(Layer::new());
    }

    /// exiting a function definition
    #[must_use = "indicates success"]
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

    /// Declare an item in the current scope
    fn decl_item_internal(
        &mut self,
        key: &'src str,
        node: Box<dyn 'ast + AstNodeFinal<'src>>,
    ) -> VarId {
        let id = VarId(self.items.len());
        self.items.push(Item { node, shape: None });
        let curr_scope = if let Some(scope) = self.layers.last_mut().map(Layer::curr_scope_mut) {
            scope
        } else {
            &mut self.global
        };
        curr_scope.items.push((key, id));
        id
    }

    pub fn decl_item<T>(&mut self, key: &'src str, node: T) -> VarId
    where
        T: 'ast + AstNodeFinal<'src>,
    {
        self.decl_item_internal(key, Box::new(node))
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Scope<'src> {
    items: Vec<(&'src str, VarId)>,
}

impl<'src> Scope<'src> {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn insert(&mut self, key: &'src str, id: VarId) {
        self.items.push((key, id));
    }
}
