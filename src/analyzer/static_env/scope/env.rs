use crate::{
    analyzer::static_env::{ItemId, VarId},
    parse::ast,
    src_code::span::Span,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, ast::AstNodeEnum)]
pub enum Decl {
    Mod(ast::DeclMod),
    Rec(ast::DeclRec),
    Field(ast::Field),
    Cat(ast::DeclCat),
    Var(ast::DeclVar),
    DeferParam(ast::DeferParam),
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
impl From<ast::DeferParam> for Decl {
    fn from(value: ast::DeferParam) -> Self {
        Self::DeferParam(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LibEntry<'src> {
    pub name: String,
    pub def: Module<'src>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModEntry<'src> {
    pub name: &'src str,
    pub decl: ast::DeclMod,
    pub def: Module<'src>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct DuplicateLibraryError;

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateModuleError<'env> {
    /// The site of the original declaration
    pub og: &'env ast::DeclMod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct DuplicateItemError {
    /// The ID of the original declaration
    pub og: ItemId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UndeclaredModuleError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DuplicateMarkingError {
    /// The existing reference marked on the span
    pub og: ItemId,
}

#[derive(Debug, Default)]
pub struct ScopeEnvironment<'src> {
    /// Indexed with [`ItemId`]
    decls: Vec<Decl>,
    members: HashMap<ItemId, HashMap<&'src str, ItemId>>,
    ids: HashMap<Span, ItemId>,
    /// In the order their items would appear in [`Self::items`]
    libraries: Vec<LibEntry<'src>>,
}

impl<'src> ScopeEnvironment<'src> {
    pub fn new() -> Self {
        Self {
            decls: Vec::new(),
            members: HashMap::new(),
            ids: HashMap::new(),
            libraries: Vec::new(),
        }
    }

    pub fn get_library(&self, name: &str) -> Option<&LibEntry<'src>> {
        self.libraries.iter().find(|lib| name == lib.name)
    }

    pub fn get_library_mut(&mut self, name: &str) -> Option<&mut LibEntry<'src>> {
        self.libraries.iter_mut().find(|lib| name == lib.name)
    }

    pub fn define_library<'env>(
        &'env mut self,
        name: String,
    ) -> Result<ModScopeEnvironment<'env, 'src>, DuplicateLibraryError> {
        if self.get_library(name.as_str()).is_some() {
            Err(DuplicateLibraryError)
        } else {
            let entry = self.libraries.push_mut(LibEntry {
                name,
                def: Module::new(),
            });
            Ok(ModScopeEnvironment {
                decls: &mut self.decls,
                members: &mut self.members,
                ids: &mut self.ids,
                module: &mut entry.def,
            })
        }
    }
}

#[derive(Debug)]
pub struct ModScopeEnvironment<'env, 'src> {
    /// Indexed with [`ItemId`]
    decls: &'env mut Vec<Decl>,
    members: &'env mut HashMap<ItemId, HashMap<&'src str, ItemId>>,
    ids: &'env mut HashMap<Span, ItemId>,
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
    pub fn decl_module(
        &mut self,
        name: &'src str,
        decl: impl FnOnce() -> ast::DeclMod,
    ) -> Result<(), DuplicateModuleError<'_>> {
        if let Some(ent) = self.module.get_submodule(name) {
            Err(DuplicateModuleError { og: &ent.decl })
        } else {
            self.module.submodules.push(ModEntry {
                name,
                decl: decl(),
                def: Module::new(),
            });
            Ok(())
        }
    }

    pub fn define_module<'sub>(
        &'sub mut self,
        name: &str,
    ) -> Result<ModScopeEnvironment<'sub, 'src>, UndeclaredModuleError> {
        if let Some(ent) = self.module.get_submodule_mut(name) {
            Ok(ModScopeEnvironment {
                decls: self.decls,
                members: self.members,
                ids: self.ids,
                module: &mut ent.def,
            })
        } else {
            Err(UndeclaredModuleError)
        }
    }

    pub fn decl_item(
        &mut self,
        name: &'src str,
        name_span: Span,
        decl: impl Into<Decl>,
    ) -> Result<ItemId, DuplicateItemError> {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self.module.decl_item(name, id)?;
        self.decls.push(decl.into()); // now `id` refers to an item
        let res = self.mark_span_item(name_span, id);
        debug_assert_eq!(res, Ok(()), "span should only refer to one item");
        Ok(id)
    }

    pub fn decl_member(
        &mut self,
        on: ItemId,
        name: &'src str,
        name_span: Span,
        decl: impl Into<Decl>,
    ) -> Result<ItemId, DuplicateItemError> {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        if let Err(e) = self.members.entry(on).or_default().try_insert(name, id) {
            return Err(DuplicateItemError { og: e.value });
        }
        self.decls.push(decl.into()); // now `id` refers to an item
        let res = self.mark_span_item(name_span, id);
        debug_assert_eq!(res, Ok(()), "span should only refer to one item");
        Ok(id)
    }

    pub fn decl_var(&mut self, name: &'src str, name_span: Span, decl: impl Into<Decl>) -> VarId {
        let id = ItemId(self.decls.len()); // does not actually refer to an item yet!
        self.module.decl_var(name, id);
        self.decls.push(decl.into()); // now `id` refers to an item
        let res = self.mark_span_item(name_span, id);
        debug_assert_eq!(res, Ok(()), "span should only refer to one item");
        id
    }

    /// Indicate that a particular span represents a particular item
    pub fn mark_span_item(&mut self, span: Span, id: ItemId) -> Result<(), DuplicateMarkingError> {
        if let Err(e) = self.ids.try_insert(span, id) {
            Err(DuplicateMarkingError { og: e.value })
        } else {
            Ok(())
        }
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module<'src> {
    /// In the order their items would appear in [`Self::items`]
    submodules: Vec<ModEntry<'src>>,
    /// NOTE: this does NOT represent "the scope where all global variables belong",
    /// this is just the scope of all things outside of any function!!
    /// It is used to guarantee the stack is never empty.
    global: Scope<'src>,
    layers: Vec<Layer<'src>>,
}

impl<'src> Module<'src> {
    pub fn new() -> Self {
        Self {
            submodules: Vec::new(),
            global: Scope::new(),
            layers: Vec::new(),
        }
    }

    fn get_submodule<'env>(&'env self, name: &str) -> Option<&'env ModEntry<'src>> {
        self.submodules.iter().find(|item| name == item.name)
    }

    fn get_submodule_mut<'env>(&'env mut self, name: &str) -> Option<&'env mut ModEntry<'src>> {
        self.submodules.iter_mut().find(|item| name == item.name)
    }

    fn decl_module(&mut self, name: &'src str, decl: ast::DeclMod) {
        self.submodules.push(ModEntry {
            name,
            decl,
            def: Module::new(),
        });
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
        todo!("find tightest declaration visible")
    }

    pub fn lookup_var(&self, name: &str) -> Option<VarId> {
        todo!("find most recent declaration visible")
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
