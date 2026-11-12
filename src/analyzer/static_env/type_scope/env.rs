use crate::{analyzer::static_env::ItemId, parse::ast, src_code::span::Span};
use std::collections::HashMap;

#[derive(Debug, PartialEq, ast::AstNodeEnum)]
pub enum Item {
    Type(ast::DeclType),
    Var(ast::DeclVar),
    Mod(ast::DeclMod),
}

#[derive(Debug, Clone, PartialEq)]
struct LibEntry<'src> {
    pub name: &'src str,
    pub def: Module<'src>,
}

#[derive(Debug, Clone, PartialEq)]
struct ModEntry<'src> {
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
pub struct TyScopeEnvironment<'src> {
    /// Indexed with [`ItemId`]
    items: Vec<Item>,
    ids: HashMap<Span, ItemId>,
    /// In the order their items would appear in [`Self::items`]
    libraries: Vec<LibEntry<'src>>,
}

impl<'src> TyScopeEnvironment<'src> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
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
        name: &'src str, // TBD: where does the name come from?
    ) -> Result<ModScopeEnvironment<'env, 'src>, DuplicateLibraryError> {
        if self.get_library(name).is_some() {
            Err(DuplicateLibraryError)
        } else {
            let entry = self.libraries.push_mut(LibEntry {
                name,
                def: Module::new(),
            });
            Ok(ModScopeEnvironment {
                items: &mut self.items,
                ids: &mut self.ids,
                module: &mut entry.def,
            })
        }
    }
}

#[derive(Debug)]
pub struct ModScopeEnvironment<'env, 'src> {
    /// Indexed with [`ItemId`]
    items: &'env mut Vec<Item>,
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
                items: self.items,
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
        item: Item,
    ) -> Result<ItemId, DuplicateItemError> {
        let id = ItemId(self.items.len()); // does not actually refer to an item yet!
        self.module.decl_item(name, id)?;
        self.items.push(item); // now `id` refers to an item
        let res = self.mark_ref(name_span, id);
        debug_assert_eq!(res, Ok(()), "span should only refer to one item");
        Ok(id)
    }

    pub fn mark_ref(&mut self, span: Span, id: ItemId) -> Result<(), DuplicateMarkingError> {
        if let Some(og) = self.ids.insert(span, id) {
            Err(DuplicateMarkingError { og })
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

    /// Returns [`None`] if not currently in a function
    const fn curr_layer(&self) -> Option<&Layer<'src>> {
        self.layers.last()
    }

    /// Returns [`None`] if not currently in a function
    const fn curr_layer_mut(&mut self) -> Option<&mut Layer<'src>> {
        self.layers.last_mut()
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

    pub fn lookup(&self, name: &str) -> Option<ItemId> {
        self.layers
            .last()
            .map_or(&self.global, |layer| {
                layer.scopes.last().unwrap_or(&layer.root)
            })
            .items
            .get(name)
            .copied()
    }
}

/// Functions have special scope limited to only themselves and global scope
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Layer<'src> {
    pub root: Scope<'src>,
    pub scopes: Vec<Scope<'src>>,
}

impl<'src> Layer<'src> {
    pub fn new() -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scope<'src> {
    items: HashMap<&'src str, ItemId>,
}

impl<'src> Scope<'src> {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    fn decl_item(&mut self, key: &'src str, id: ItemId) -> Result<(), DuplicateItemError> {
        if let Some(og) = self.items.insert(key, id) {
            Err(DuplicateItemError { og })
        } else {
            Ok(())
        }
    }
}
