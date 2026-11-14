use crate::{
    analyzer::static_env::{ItemId, LibId, ModId, VarId},
    parse::ast,
    src_code::span::RelSpan,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, ast::AstNodeEnum)]
pub enum Decl {
    Mod(ast::DeclMod),
    Rec(ast::DeclRec),
    Field(ast::Field),
    Cat(ast::DeclCat),
    Var(ast::DeclVar),
    DferParam(ast::DferParam),
}

impl Decl {
    pub const fn identifier(&self) -> RelSpan {
        match self {
            Self::Mod(inner) => inner.name,
            Self::Rec(inner) => inner.name,
            Self::Field(inner) => inner.name,
            Self::Cat(inner) => inner.name,
            Self::Var(inner) => inner.binding.ident,
            Self::DferParam(inner) => inner.param.ident,
        }
    }
}

impl From<ast::DeclMod> for Decl {
    fn from(value: ast::DeclMod) -> Self {
        Self::Mod(value)
    }
}
impl From<ast::DeclRec> for Decl {
    fn from(value: ast::DeclRec) -> Self {
        Self::Rec(value)
    }
}
impl From<ast::Field> for Decl {
    fn from(value: ast::Field) -> Self {
        Self::Field(value)
    }
}
impl From<ast::DeclCat> for Decl {
    fn from(value: ast::DeclCat) -> Self {
        Self::Cat(value)
    }
}
impl From<ast::DeclVar> for Decl {
    fn from(value: ast::DeclVar) -> Self {
        Self::Var(value)
    }
}
impl From<ast::DferParam> for Decl {
    fn from(value: ast::DferParam) -> Self {
        Self::DferParam(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item<'src> {
    pub decl: Decl,
    pub parent: Option<ItemId>,
    pub children: BTreeMap<&'src str, ItemId>,
}

impl Item<'_> {
    pub const fn new(decl: Decl) -> Self {
        Self {
            decl,
            parent: None,
            children: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubEntry<'src> {
    pub name: &'src str,
    pub decl: ast::DeclMod,
    pub def: Module<'src>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LibEntry<'src> {
    pub name: String,
    pub def: Module<'src>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModEntry<'src> {
    Submodule(SubEntry<'src>),
    Library(LibEntry<'src>),
}

impl<'src> ModEntry<'src> {
    pub const fn is_submodule(&self) -> bool {
        matches!(self, Self::Submodule(_))
    }

    pub const fn is_library(&self) -> bool {
        matches!(self, Self::Library(_))
    }

    pub const fn decl(&self) -> Option<&ast::DeclMod> {
        match self {
            Self::Submodule(ent) => Some(&ent.decl),
            Self::Library(_) => None,
        }
    }

    pub const fn name(&self) -> &str {
        match self {
            Self::Submodule(ent) => ent.name,
            Self::Library(ent) => ent.name.as_str(),
        }
    }

    pub const fn def(&self) -> &Module<'src> {
        match self {
            Self::Submodule(ent) => &ent.def,
            Self::Library(ent) => &ent.def,
        }
    }

    pub const fn def_mut(&mut self) -> &mut Module<'src> {
        match self {
            Self::Submodule(ent) => &mut ent.def,
            Self::Library(ent) => &mut ent.def,
        }
    }
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
pub struct ScopeEnvironment<'src> {
    /// Indexed with [`ItemId`]
    decls: Vec<Item<'src>>,

    /// The set of all modules in the program
    ///
    /// Indexed with [`ModId`]
    modules: Vec<ModEntry<'src>>,
}

impl<'src> ScopeEnvironment<'src> {
    pub fn new() -> Self {
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
            .position(|ent| matches!(ent, ModEntry::Library(lib) if name == &lib.name))
            .map(ModId)
    }

    pub fn get_module(&self, id: ModId) -> Option<&ModEntry<'src>> {
        self.modules.get(id.0)
    }

    pub fn get_module_mut(&mut self, id: ModId) -> Option<&mut ModEntry<'src>> {
        self.modules.get_mut(id.0)
    }

    pub fn decl_library(&mut self, name: String) -> Result<LibId, DuplicateModuleError> {
        if let Some(og) = self.find_library(name.as_str()) {
            Err(DuplicateModuleError { og })
        } else {
            let id = ModId(self.modules.len());
            self.modules.push(ModEntry::Library(LibEntry {
                name,
                def: Module::new(),
            }));
            Ok(id)
        }
    }

    pub fn decl_submodule(
        &mut self,
        parent: ModId,
        name: &'src str,
        decl: ast::DeclMod,
    ) -> Result<ModId, CreateSubmoduleError> {
        let id = ModId(self.modules.len());
        self.modules
            .get_mut(parent.0)
            .ok_or(CreateSubmoduleError::UnknownParent(UndeclaredModuleError))
            .and_then(|parent| {
                parent
                    .def_mut()
                    .decl_submodule(name, id)
                    .map_err(CreateSubmoduleError::DuplicateChild)
            })?;
        self.modules.push(ModEntry::Submodule(SubEntry {
            name,
            decl,
            def: Module::new(),
        }));
        Ok(id)
    }

    pub fn define_module(
        &mut self,
        id: ModId,
    ) -> Result<ModScopeEnvironment<'_, 'src>, UndeclaredModuleError> {
        self.modules
            .get_mut(id.0)
            .map(|ent| ModScopeEnvironment {
                decls: &mut self.decls,
                module: ent.def_mut(),
            })
            .ok_or(UndeclaredModuleError)
    }
}

#[derive(Debug)]
pub struct ModScopeEnvironment<'env, 'src> {
    /// Indexed with [`ItemId`]
    decls: &'env mut Vec<Item<'src>>,
    module: &'env mut Module<'src>,
}

impl<'src> std::ops::Deref for ModScopeEnvironment<'_, 'src> {
    type Target = Module<'src>;

    fn deref(&self) -> &Self::Target {
        self.module
    }
}

impl std::ops::DerefMut for ModScopeEnvironment<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.module
    }
}

impl<'src> ModScopeEnvironment<'_, 'src> {
    pub fn decl_item(
        &mut self,
        name: &'src str,
        decl: impl Into<Decl>,
    ) -> Result<ItemId, DuplicateItemError> {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self.module.decl_item(name, id)?;
        self.decls.push(Item::new(decl.into())); // now `id` refers to an item
        Ok(id)
    }

    pub fn decl_member(
        &mut self,
        parent: ItemId,
        name: &'src str,
        decl: impl Into<Decl>,
    ) -> Result<ItemId, LinkMemberError> {
        let child = ItemId(self.decls.len());
        self.decls.push(Item::new(decl.into()));
        // TODO: duplicate name check on parent
        self.decls
            .get_disjoint_mut([parent.0, child.0])
            .map_err(LinkMemberError::Access)
            .and_then(|[parent_item, child_item]| {
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
            })
    }

    pub fn decl_var(&mut self, name: &'src str, decl: impl Into<Decl>) -> VarId {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self.module.decl_var(name, id);
        self.decls.push(Item::new(decl.into())); // now `id` refers to an item
        id
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module<'src> {
    submodules: BTreeMap<&'src str, ModId>,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope<'src>,
    layers: Vec<Layer<'src>>,
}

impl<'src> Module<'src> {
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

    fn decl_submodule(&mut self, name: &'src str, id: ModId) -> Result<(), DuplicateModuleError> {
        if let Some(og) = self.submodules.insert(name, id) {
            Err(DuplicateModuleError { og })
        } else {
            Ok(())
        }
    }

    fn decl_item(&mut self, name: &'src str, id: ItemId) -> Result<(), DuplicateItemError> {
        self.layers
            .last_mut()
            .map_or(&mut self.global, |layer| {
                layer.scopes.last_mut().unwrap_or(&mut layer.root)
            })
            .decl_item(name, id)
    }

    fn decl_var(&mut self, name: &'src str, id: VarId) {
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
pub struct Layer<'src> {
    pub root: Scope<'src>,
    pub scopes: Vec<Scope<'src>>,
}

impl Layer<'_> {
    pub fn new() -> Self {
        Self {
            root: Scope::new(),
            scopes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scope<'src> {
    /// Functions and types; unordered and must be unique within a scope
    items: HashMap<&'src str, ItemId>,

    /// Variables; names can be shadowed and reused within the same scope
    vars: Vec<(&'src str, VarId)>,
}

impl<'src> Scope<'src> {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            vars: Vec::new(),
        }
    }

    fn decl_item(&mut self, key: &'src str, id: ItemId) -> Result<(), DuplicateItemError> {
        if let Err(e) = self.items.try_insert(key, id) {
            Err(DuplicateItemError { og: e.value })
        } else {
            Ok(())
        }
    }

    fn decl_var(&mut self, key: &'src str, id: VarId) {
        self.vars.push((key, id));
    }
}
