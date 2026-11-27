use crate::{
    analyzer::static_env::{ItemId, LabelId, LibId, ModId, VarId},
    parse::ast::{self},
    src_code::span::Span,
};
use std::{
    collections::{BTreeMap, HashMap},
    num::NonZero,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ScopeId {
    #[default]
    Global,
    Local {
        layer: NonZero<usize>,
        scope: usize,
    },
}
const _: () = {
    assert!(
        std::mem::size_of::<ScopeId>() <= 2 * std::mem::size_of::<usize>(),
        "should use NonZero niche for Global"
    );
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemKind {
    /// Declared with `pvt`
    Pvt,
    /// Declared with `uni`
    Uni,
    /// Declared with `let` or a `for` loop
    Var,
    /// A `me` parameter
    Me,
    /// A `fun` parameter (but not `me`)
    Param,
    /// A `dfer` parameter
    DferParam,
    /// A generic parameter
    TypeParam,
    /// Declared with `for` in a satisfies/expression bound
    BoundParam,
    /// Declared with `rec`
    Rec,
    /// A `rec` field
    Field,
    /// Declared with `cat`
    Cat,
    /// A `cat` discriminant
    Discrim,
    /// Declared with `sup`
    SubType,
    /// Declared with `sub`
    SuperType,
    /// Declared with `alt`
    Alt,
    /// Declared with `fun`
    Fun,
    /// Declared with `.name:` where `name` is the label name
    Label,
}

impl std::fmt::Debug for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let decl = match self {
            Self::Pvt => "pvt",
            Self::Uni => "uni",
            Self::Var => "var",
            Self::TypeParam => "generic",
            Self::BoundParam => "bound",
            Self::Rec | Self::Field => "rec",
            Self::Cat | Self::Discrim => "cat",
            Self::SubType => "sub",
            Self::SuperType => "sup",
            Self::Alt => "alt",
            Self::Me | Self::Param | Self::Fun => "fun",
            Self::DferParam => "dfer",
            Self::Label => "loop",
        };
        f.write_str(decl)?;
        let decl_spec = match self {
            Self::Me => Some("me"),
            Self::Param | Self::TypeParam | Self::BoundParam | Self::DferParam => Some("param"),
            Self::Field => Some("field"),
            Self::Discrim => Some("discr"),
            Self::Label => Some("label"),
            _ => None,
        };
        if let Some(decl_spec) = decl_spec {
            write!(f, ".{decl_spec}")?;
        }
        let kind = match self {
            Self::Uni
            | Self::Var
            | Self::Me
            | Self::Param
            | Self::BoundParam
            | Self::Field
            | Self::Pvt
            | Self::Discrim
            | Self::DferParam => "data",

            Self::Rec
            | Self::Cat
            | Self::SubType
            | Self::SuperType
            | Self::Alt
            | Self::TypeParam => "layout",

            Self::Fun | Self::Label => "flow",
        };
        write!(f, ">{kind}")?;
        let kind_spec = match self {
            Self::BoundParam | Self::Field | Self::TypeParam => Some("hypothetical"),
            Self::Pvt | Self::Discrim => Some("constant"),
            Self::Alt | Self::Label => Some("alias"),

            _ => None,
        };
        if let Some(kind_spec) = kind_spec {
            write!(f, ".{kind_spec}")?;
        }
        Ok(())
    }
}

impl crate::highlight::syntax::GetSyntax for ItemKind {
    fn syntax(&self) -> crate::highlight::syntax::Syntax {
        use crate::highlight::syntax::Syntax;
        match self {
            Self::Pvt => Syntax::Constant,
            Self::Uni | Self::Var => Syntax::Variable,
            Self::Me => Syntax::LanguageVariable,
            Self::Param | Self::DferParam | Self::BoundParam => Syntax::Param,
            Self::Field => Syntax::Field,
            Self::Discrim => Syntax::Discriminant,
            Self::TypeParam
            | Self::Rec
            | Self::Cat
            | Self::SubType
            | Self::SuperType
            | Self::Alt => Syntax::Typename,
            Self::Fun => Syntax::Callable,
            Self::Label => Syntax::Label,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub decl: Span,
    pub kind: ItemKind,
    pub parent: Option<ItemId>,
    pub children: BTreeMap<String, ItemId>,

    pub scope: ScopeId,
}

impl Item {
    pub const fn new(decl: Span, kind: ItemKind, scope: ScopeId) -> Self {
        Self {
            decl,
            kind,
            parent: None,
            children: BTreeMap::new(),
            scope,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModEntry {
    pub name: String,
    pub def: Module,
    /// [`Some`] if a submodule
    pub decl: Option<ast::DeclMod>,
}

/// A module with the provided name has already been declared in this module; duplicates are not allowed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct DuplicateModuleError {
    /// The ID of the first declaration of a module by this name
    pub og: ModId,
}

/// An item with the provided name has already been declared in this scope; duplicates are not allowed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct DuplicateItemError {
    /// The ID of the first declaration of an item by this name in the current scope
    pub og: ItemId,
}

/// An ancestor loop shares a name with this label
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct LabelJrError {
    /// The ID of the ancestor sharing a name with this label
    pub og: LabelId,
}

/// Attempted to push a shadowable variable to global scope (only unique items can be at global scope)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Default)]
pub struct GlobalVarError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkMemberError {
    Access(std::slice::GetDisjointMutError),
    ReParent { existing: ItemId },
    AlreadyLinked,
    DuplicateChild(DuplicateItemError),
}

/// Attempted to reference a module that was never declared
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UndeclaredModuleError;

#[derive(Debug, Clone, PartialEq)]
pub enum CreateSubmoduleError {
    /// The parent module was never declared
    UnknownParent(UndeclaredModuleError),
    /// The parent module was declared, but the child module was already present in that parent
    DuplicateChild(DuplicateModuleError),
}

#[derive(Debug, Default)]
pub struct ScopeEnvironment {
    /// Indexed with [`ItemId`]
    decls: Vec<Item>,

    /// The set of all modules in the program
    ///
    /// Indexed with [`ModId`]
    modules: Vec<ModEntry>,
}

impl std::ops::Index<ModId> for ScopeEnvironment {
    type Output = Module;

    fn index(&self, index: ModId) -> &Self::Output {
        #[expect(
            clippy::indexing_slicing,
            reason = "ModId can only be obtained by pushing a module, and modules cannot be removed or rearranged"
        )]
        &self.modules[index.0].def
    }
}

impl std::ops::IndexMut<ModId> for ScopeEnvironment {
    fn index_mut(&mut self, index: ModId) -> &mut Self::Output {
        #[expect(
            clippy::indexing_slicing,
            reason = "ModId can only be obtained by pushing a module, and modules cannot be removed or rearranged"
        )]
        &mut self.modules[index.0].def
    }
}

impl ScopeEnvironment {
    pub const fn new() -> Self {
        Self {
            decls: Vec::new(),
            modules: Vec::new(),
        }
    }

    pub fn get_decl(&self, id: ItemId) -> Option<&Item> {
        self.decls.get(id.0)
    }

    pub fn find_library(&self, name: &str) -> Option<LibId> {
        self.modules
            .iter()
            .position(|ent| ent.decl.is_none() && *name == ent.name)
            .map(ModId)
    }

    pub fn get_module(&self, id: ModId) -> Option<&ModEntry> {
        self.modules.get(id.0)
    }

    pub fn get_module_mut(&mut self, id: ModId) -> Option<&mut ModEntry> {
        self.modules.get_mut(id.0)
    }

    pub fn decl_library(&mut self, name: String) -> Result<LibId, DuplicateModuleError> {
        if let Some(og) = self.find_library(name.as_str()) {
            Err(DuplicateModuleError { og })
        } else {
            let id = ModId(self.modules.len());
            self.modules.push(ModEntry {
                name,
                def: Module::new(),
                decl: None,
            });
            Ok(id)
        }
    }

    pub fn decl_submodule(
        &mut self,
        parent: ModId,
        name: String,
        decl: ast::DeclMod,
    ) -> Result<ModId, CreateSubmoduleError> {
        let id = ModId(self.modules.len());
        self.modules
            .get_mut(parent.0)
            .ok_or(CreateSubmoduleError::UnknownParent(UndeclaredModuleError))?
            .def
            .decl_submodule(name.clone(), id)
            .map_err(CreateSubmoduleError::DuplicateChild)?;
        self.modules.push(ModEntry {
            name,
            def: Module::new(),
            decl: Some(decl),
        });
        Ok(id)
    }

    pub fn decl_member(
        &mut self,
        within: ModId,
        parent: ItemId,
        name: String,
        kind: ItemKind,
        decl: Span,
    ) -> Result<ItemId, LinkMemberError> {
        let child = ItemId(self.decls.len());
        self.decls
            .push(Item::new(decl, kind, self[within].curr_scope_id()));
        // TODO: duplicate name check on parent
        let [parent_item, child_item] = self
            .decls
            .get_disjoint_mut([parent.0, child.0])
            .map_err(LinkMemberError::Access)?;
        // if there is already a parent, we don't want to add this as a child
        if let Some(existing) = child_item.parent {
            return Err(if existing == child {
                LinkMemberError::AlreadyLinked
            } else {
                LinkMemberError::ReParent { existing }
            });
        }
        child_item.parent = Some(parent);

        // if the child is already a member, then nothing structurally changes
        if let Some(og) = parent_item.children.insert(name, child) {
            Err(LinkMemberError::DuplicateChild(DuplicateItemError { og }))
        } else {
            Ok(child)
        }
    }

    /// Declare an item at the current scope, generating a new ID. \
    /// Returns [`Err`] if an item with that name is already in-scope.
    pub fn decl_item(
        &mut self,
        within: ModId,
        name: String,
        kind: ItemKind,
        decl: Span,
    ) -> Result<ItemId, DuplicateItemError> {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self[within].decl_item(name, id)?;
        let scope = self[within].curr_scope_id();
        self.decls.push(Item::new(decl, kind, scope)); // now `id` refers to an item
        Ok(id)
    }

    /// Declare an item at the current scope, generating a new ID. \
    /// Returns [`Err`] if an item with that name is already in-scope.
    pub fn decl_label(
        &mut self,
        within: ModId,
        name: String,
        decl: Span,
    ) -> (LabelId, Result<(), LabelJrError>) {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        let res = self[within].decl_label(name, id);
        let scope = self[within].curr_scope_id();
        self.decls.push(Item::new(decl, ItemKind::Label, scope)); // now `id` refers to an item
        (id, res)
    }

    /// Declare a variable at the current scope, generating a new ID.
    pub fn decl_var(&mut self, within: ModId, name: String, kind: ItemKind, decl: Span) -> VarId {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self[within].decl_var(name, id);
        let scope = self[within].curr_scope_id();
        self.decls.push(Item::new(decl, kind, scope)); // now `id` refers to an item
        id
    }

    #[cfg(debug_assertions)]
    pub fn debug_exports(&self) {
        for ent in &self.modules {
            println!("{}:", ent.name);
            ent.def.debug_exports();
        }
    }
}

/// Does not support shadowing, every item in a scope must be unique
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemScope(HashMap<String, ItemId>);

impl Default for ItemScope {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemScope {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    fn decl(&mut self, key: String, id: ItemId) -> Result<(), DuplicateItemError> {
        self.0
            .try_insert(key, id)
            .map_err(|e| DuplicateItemError { og: *e.entry.get() })
            .map(|_| ())
    }

    pub fn get(&self, key: &str) -> Option<&ItemId> {
        self.0.get(key)
    }
}

/// Supports shadowing; does not store all previous names, just the most recent item by a particular name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowScope<T>(HashMap<String, T>);

impl<T> Default for ShadowScope<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ShadowScope<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    fn decl(&mut self, key: String, id: T) {
        self.0.insert(key, id);
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        self.0.get(key)
    }
}

pub type LabelScope = ShadowScope<LabelId>;
pub type VarScope = ShadowScope<VarId>;

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    submodules: BTreeMap<String, ModId>,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Layer,
    layers: Vec<Layer>,
}

pub type MinOneIntoIter<T> = std::iter::Chain<std::iter::Once<T>, std::vec::IntoIter<T>>;
pub type MinOneIter<'a, T> = std::iter::Chain<std::iter::Once<&'a T>, std::slice::Iter<'a, T>>;
pub type MinOneIterMut<'a, T> =
    std::iter::Chain<std::iter::Once<&'a mut T>, std::slice::IterMut<'a, T>>;

pub type LayerIntoIter = MinOneIntoIter<Layer>;
pub type LayerIter<'a> = MinOneIter<'a, Layer>;
pub type LayerIterMut<'a> = MinOneIterMut<'a, Layer>;

pub type ScopeIntoIter = MinOneIntoIter<Scope>;
pub type ScopeIter<'a> = MinOneIter<'a, Scope>;
pub type ScopeIterMut<'a> = MinOneIterMut<'a, Scope>;

pub type ModScopeIntoIter =
    std::iter::FlatMap<LayerIntoIter, ScopeIntoIter, fn(Layer) -> ScopeIntoIter>;
pub type ModScopeIter<'a> =
    std::iter::FlatMap<LayerIter<'a>, ScopeIter<'a>, fn(&Layer) -> ScopeIter<'_>>;
pub type ModScopeIterMut<'a> =
    std::iter::FlatMap<LayerIterMut<'a>, ScopeIterMut<'a>, fn(&mut Layer) -> ScopeIterMut<'_>>;

impl Module {
    pub fn new() -> Self {
        Self {
            submodules: BTreeMap::new(),
            global: Layer::new(),
            layers: Vec::new(),
        }
    }

    pub fn layer_iter(&self) -> LayerIter<'_> {
        std::iter::once(&self.global).chain(&self.layers)
    }

    pub fn layer_iter_mut(&mut self) -> LayerIterMut<'_> {
        std::iter::once(&mut self.global).chain(&mut self.layers)
    }

    pub fn scope_iter(&self) -> ModScopeIter<'_> {
        self.layer_iter().flat_map(Layer::scope_iter)
    }

    pub fn scope_iter_mut(&mut self) -> ModScopeIterMut<'_> {
        self.layer_iter_mut().flat_map(Layer::scope_iter_mut)
    }

    pub fn curr_layer(&self) -> &Layer {
        self.layers.last().unwrap_or(&self.global)
    }

    pub fn curr_layer_mut(&mut self) -> &mut Layer {
        self.layers.last_mut().unwrap_or(&mut self.global)
    }

    #[cfg(debug_assertions)]
    pub fn debug_exports(&self) {
        println!("exports: {:#?}", self.global);
    }

    /// Clears the scope data without removing any IDs
    pub fn clear_scopes(&mut self) {
        self.global.clear();
        self.layers.clear();
    }

    pub fn curr_scope_id(&self) -> ScopeId {
        self.layers
            .last()
            .map_or(ScopeId::Global, |layer| ScopeId::Local {
                layer: NonZero::new(self.layers.len())
                    .expect("layer count should be nonzero if a last element exists"),
                scope: layer.scopes.len(),
            })
    }

    pub fn find_submodule(&self, name: &str) -> Option<ModId> {
        self.submodules.get(name).copied()
    }

    /// Returns [`Err`] if an module with `name` is already declared in this module \
    /// (module declarations are file-level, so scoping does not allow duplicates)
    pub fn decl_submodule(&mut self, name: String, id: ModId) -> Result<(), DuplicateModuleError> {
        if let Some(og) = self.submodules.insert(name, id) {
            Err(DuplicateModuleError { og })
        } else {
            Ok(())
        }
    }

    /// Insert an item with an existing ID into the current scope. \
    /// Returns [`Err`] if an item with `name` already exists at the current scope.
    pub fn decl_item(&mut self, name: String, id: ItemId) -> Result<(), DuplicateItemError> {
        let layer = self.curr_layer_mut();
        layer.curr_scope_mut().items.decl(name, id)
    }

    /// Insert a label with an existing ID into the current scope. \
    /// Returns [`Err`] if an item with `name` is still visible in an outer scope.
    ///
    /// If the label is labelling the scope itself, not another scope within it, `is_parent` should be true.
    pub fn decl_label(&mut self, name: String, id: LabelId) -> Result<(), LabelJrError> {
        let layer = self.curr_layer_mut();
        if let Some((_, id)) = layer
            .scope_iter()
            .rev()
            .flat_map(|scope| &scope.label)
            .find(|(key, _)| *key == name)
        {
            Err(LabelJrError { og: *id })
        } else {
            layer.curr_scope_mut().set_label(name, id);
            Ok(())
        }
    }

    /// Insert a variable with an existing ID into the current scope. \
    /// Infallible because duplicate names shadow each other and a global scope is guaranteed.
    pub fn decl_var(&mut self, name: String, id: VarId) {
        self.curr_layer_mut().curr_scope_mut().vars.decl(name, id);
    }

    pub fn lookup_item(&self, name: &str) -> Option<ItemId> {
        #[cfg(all(debug_assertions, false))]
        println!("looking for item `{name}`...");

        let res = self
            .scope_iter()
            .map(|scope| &scope.items)
            .rev()
            .find_map(|items| {
                #[cfg(all(debug_assertions, false))]
                println!("    looking in {items:?}");

                items.get(name).copied()
            });
        #[cfg(all(debug_assertions, false))]
        if let Some(res) = &res {
            println!("  found: `{res}`");
        } else {
            println!("  no match found");
        }
        res
    }

    pub fn lookup_label(&self, name: &str) -> Option<LabelId> {
        self.scope_iter()
            .flat_map(|scope| &scope.label)
            .rev()
            .find(|(label, _)| label == name)
            .map(|(_, id)| *id)
    }

    pub fn lookup_var(&self, name: &str) -> Option<VarId> {
        #[cfg(all(debug_assertions, false))]
        println!("looking for variable `{name}`...");

        let res = self
            .scope_iter()
            .map(|scope| &scope.vars)
            .rev()
            .find_map(|scope| scope.get(name))
            .copied();
        #[cfg(all(debug_assertions, false))]
        if let Some(res) = &res {
            println!("  found: `{res}`");
        } else {
            println!("  no match found");
        }
        res
    }

    pub fn push_layer(&mut self) -> &mut Layer {
        self.layers.push_mut(Layer::new())
    }

    #[track_caller]
    pub fn pop_layer(&mut self) -> Layer {
        self.layers
            .pop()
            .expect("layer push/pop should be balanced")
    }

    pub fn push_scope(&mut self) -> &mut Scope {
        self.curr_layer_mut().push_scope()
    }

    #[track_caller]
    pub fn pop_scope(&mut self) -> Scope {
        self.curr_layer_mut()
            .pop_scope()
            .expect("scope push/pop should be balanced")
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Layer {
    pub root: Scope,
    pub scopes: Vec<Scope>,
}

impl Layer {
    pub fn new() -> Self {
        Self {
            root: Scope::new(),
            scopes: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.root.clear();
        self.scopes.clear();
    }

    pub fn push_scope(&mut self) -> &mut Scope {
        self.scopes.push_mut(Scope::new())
    }

    pub fn pop_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    pub fn scope_iter(&self) -> ScopeIter<'_> {
        std::iter::once(&self.root).chain(&self.scopes)
    }

    pub fn scope_iter_mut(&mut self) -> ScopeIterMut<'_> {
        std::iter::once(&mut self.root).chain(&mut self.scopes)
    }

    pub fn curr_scope(&self) -> &Scope {
        self.scopes.last().unwrap_or(&self.root)
    }

    pub fn curr_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap_or(&mut self.root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scope {
    label: Option<(String, LabelId)>,

    /// Functions and types; unordered and must be unique within a scope
    items: ItemScope,

    /// Variables; ordered and can be shadowed and reused within the same scope
    vars: VarScope,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            label: None,
            items: ItemScope::new(),
            vars: VarScope::new(),
        }
    }

    fn set_label(&mut self, name: String, id: LabelId) {
        let existing = self.label.replace((name, id));
        debug_assert_eq!(existing, None, "should not reassign scope label");
    }

    fn clear(&mut self) {
        self.items.clear();
        self.vars.clear();
    }
}
