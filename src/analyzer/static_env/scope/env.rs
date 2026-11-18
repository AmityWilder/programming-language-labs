use crate::{
    analyzer::static_env::{ItemId, LibId, ModId, VarId},
    parse::ast::{self},
    src_code::span::Span,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub decl: Span,
    pub parent: Option<ItemId>,
    pub children: BTreeMap<String, ItemId>,
}

impl Item {
    pub const fn new(decl: Span) -> Self {
        Self {
            decl,
            parent: None,
            children: BTreeMap::new(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct DuplicateModuleError {
    /// The ID of the original declaration
    pub og: ModId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct DuplicateItemError {
    /// The ID of the original declaration
    pub og: ItemId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkMemberError {
    Access(std::slice::GetDisjointMutError),
    ReParent { existing: ItemId },
    AlreadyLinked,
    DuplicateChild(DuplicateItemError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UndeclaredModuleError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DuplicateMarkingError {
    /// The existing reference marked on the span
    pub og: ItemId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreateSubmoduleError {
    UnknownParent(UndeclaredModuleError),
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

    pub fn define_module(
        &mut self,
        id: ModId,
    ) -> Result<ModScopeEnvironment<'_>, UndeclaredModuleError> {
        self.modules
            .get_mut(id.0)
            .map(|ent| ModScopeEnvironment {
                decls: &mut self.decls,
                module: &mut ent.def,
            })
            .ok_or(UndeclaredModuleError)
    }
}

#[derive(Debug)]
pub struct ModScopeEnvironment<'env> {
    /// Indexed with [`ItemId`]
    decls: &'env mut Vec<Item>,
    module: &'env mut Module,
}

impl std::ops::Deref for ModScopeEnvironment<'_> {
    type Target = Module;

    fn deref(&self) -> &Self::Target {
        self.module
    }
}

impl std::ops::DerefMut for ModScopeEnvironment<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.module
    }
}

impl ModScopeEnvironment<'_> {
    pub fn get_decl(&self, id: ItemId) -> Option<&Item> {
        self.decls.get(id.0)
    }

    pub fn decl_item(&mut self, name: String, decl: Span) -> Result<ItemId, DuplicateItemError> {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self.module.decl_item(name, id)?;
        self.decls.push(Item::new(decl)); // now `id` refers to an item
        Ok(id)
    }

    pub fn decl_member(
        &mut self,
        parent: ItemId,
        name: String,
        decl: Span,
    ) -> Result<ItemId, LinkMemberError> {
        let child = ItemId(self.decls.len());
        self.decls.push(Item::new(decl));
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

    pub fn decl_var(&mut self, name: String, decl: Span) -> VarId {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self.module.decl_var(name, id);
        self.decls.push(Item::new(decl)); // now `id` refers to an item
        id
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    submodules: BTreeMap<String, ModId>,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope,
    layers: Vec<Layer>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            submodules: BTreeMap::new(),
            global: Scope::new(),
            layers: Vec::new(),
        }
    }

    pub fn find_submodule(&self, name: &str) -> Option<ModId> {
        self.submodules.get(name).copied()
    }

    fn decl_submodule(&mut self, name: String, id: ModId) -> Result<(), DuplicateModuleError> {
        if let Some(og) = self.submodules.insert(name, id) {
            Err(DuplicateModuleError { og })
        } else {
            Ok(())
        }
    }

    fn decl_item(&mut self, name: String, id: ItemId) -> Result<(), DuplicateItemError> {
        self.layers
            .last_mut()
            .map_or(&mut self.global, |layer| {
                layer.scopes.last_mut().unwrap_or(&mut layer.root)
            })
            .decl_item(name, id)
    }

    fn decl_var(&mut self, name: String, id: VarId) {
        self.layers
            .last_mut()
            .map_or(&mut self.global, |layer| {
                layer.scopes.last_mut().unwrap_or(&mut layer.root)
            })
            .decl_var(name, id);
    }

    pub fn lookup_item(&self, name: &str) -> Option<ItemId> {
        std::iter::once(&self.global)
            .chain(self.layers.iter().flat_map(|layer| layer.scopes.iter()))
            .rev()
            .find_map(|scope| scope.items.get(name).copied())
    }

    pub fn lookup_var(&self, name: &str) -> Option<VarId> {
        self.global
            .vars
            .iter()
            .chain(
                self.layers
                    .iter()
                    .flat_map(|layer| layer.scopes.iter().flat_map(|scope| scope.vars.iter())),
            )
            .rev()
            .find(|(key, _)| *key == name)
            .map(|(_, val)| *val)
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scope {
    /// Functions and types; unordered and must be unique within a scope
    items: HashMap<String, ItemId>,

    /// Variables; ordered and can be shadowed and reused within the same scope
    vars: Vec<(String, VarId)>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            vars: Vec::new(),
        }
    }

    fn decl_item(&mut self, key: String, id: ItemId) -> Result<(), DuplicateItemError> {
        self.items
            .try_insert(key, id)
            .map_err(|e| DuplicateItemError { og: *e.entry.get() })
            .map(|x| debug_assert_eq!(*x, id, "these should be the same, by definition"))
    }

    fn decl_var(&mut self, key: String, id: VarId) {
        self.vars.push((key, id));
    }
}
