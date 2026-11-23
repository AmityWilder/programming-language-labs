use crate::{
    analyzer::{
        scope::hir::{self, HirPass, UnlinkedPass},
        static_env::{ModId, scope::ScopeEnvironment},
    },
    error::{
        CheckDiagnostics, Diagnostic, DiagnosticCategory::StaticAnalysis, DiagnosticKind, Severity,
    },
    lex::token::{CharLiteral, NumberLiteral, TextLiteral},
    parse::ast::{self, AstNode as _},
    src_code::{
        map::SourceMap,
        span::{FileId, RelSpan},
    },
};
use std::collections::BTreeMap;

pub struct PostDeclModPass;

impl HirPass for PostDeclModPass {
    type ModDcl = hir::LinkedMod;

    type ItemDcl = <hir::UnlinkedPass as HirPass>::ItemDcl;
    type ItemRef = <hir::UnlinkedPass as HirPass>::ItemRef; // TODO: distinguish from var

    type VarDcl = <hir::UnlinkedPass as HirPass>::VarDcl; // TODO: distinguish from item
    type VarRef = <hir::UnlinkedPass as HirPass>::VarRef;

    type TypeRef = <hir::UnlinkedPass as HirPass>::TypeRef;

    type LabelDcl = <hir::UnlinkedPass as HirPass>::LabelDcl; // TODO: actually implement this
    type LabelRef = <hir::UnlinkedPass as HirPass>::LabelRef;

    type Literal = <hir::UnlinkedPass as HirPass>::Literal;
}

pub struct DeclModVisitor<'env, 'src, 'err> {
    source: &'src SourceMap,
    env: &'env mut ScopeEnvironment,
    diagnostics: &'err mut Vec<Diagnostic>,
    file: FileId,
    id: ModId,
    mapping: BTreeMap<String, ModId>,
}

impl hir::Visitor<UnlinkedPass> for DeclModVisitor<'_, '_, '_> {
    type Output = PostDeclModPass;
}

impl<'env, 'src, 'err> DeclModVisitor<'env, 'src, 'err> {
    pub const fn new(
        source: &'src SourceMap,
        env: &'env mut ScopeEnvironment,
        diagnostics: &'err mut Vec<Diagnostic>,
        file: FileId,
        id: ModId,
    ) -> Self {
        Self {
            source,
            env,
            diagnostics,
            file,
            id,
            mapping: BTreeMap::new(),
        }
    }

    pub fn mapping(self) -> BTreeMap<String, ModId> {
        self.mapping
    }
}

impl hir::ModDclVisitor<hir::UnlinkedMod> for DeclModVisitor<'_, '_, '_> {
    type Value = hir::LinkedMod;

    fn visit_mod_dcl(&mut self, x: hir::UnlinkedMod) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedMod(decl) = x;
        #[expect(
            clippy::indexing_slicing,
            reason = "AST spans should be valid and file ID should match the AST"
        )]
        let subname = &self.source[self.file][decl.name];
        match self.env.decl_submodule(self.id, subname.to_owned(), decl) {
            Ok(subid) => {
                let existing = self.mapping.insert(subname.to_owned(), subid);
                assert_eq!(
                    existing, None,
                    "mapping should be a subset of env submodules, error should have been produced by decl_submodule"
                );
                return Ok(hir::LinkedMod(subid));
            }
            Err(e) => self.diagnostics.push(Diagnostic::new(
                decl.span().within_file(self.file),
                DiagnosticKind::Other {
                    severity: Severity::Error,
                    category: StaticAnalysis,
                    msg: format!("failed to declare submodule: {e:?}"),
                },
            )),
        }
        Err(CheckDiagnostics)
    }
}

// impl hir::IgnoreModDcl<<UnlinkedPass as HirPass>::ModDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreItemDcl<<UnlinkedPass as HirPass>::ItemDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreItemRef<<UnlinkedPass as HirPass>::ItemRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreVarDcl<<UnlinkedPass as HirPass>::VarDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreVarRef<<UnlinkedPass as HirPass>::VarRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreTypeRef<<UnlinkedPass as HirPass>::TypeRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelDcl<<UnlinkedPass as HirPass>::LabelDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelRef<<UnlinkedPass as HirPass>::LabelRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreLiteral<<UnlinkedPass as HirPass>::Literal> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreScope for DeclModVisitor<'_, '_, '_> {}

pub struct PostDeclPass;

impl HirPass for PostDeclPass {
    type ModDcl = <PostDeclModPass as HirPass>::ModDcl;

    type ItemDcl = hir::LinkedDecl;
    type ItemRef = <PostDeclModPass as HirPass>::ItemRef; // TODO: distinguish from var

    type VarDcl = hir::LinkedDecl; // TODO: distinguish from item
    type VarRef = <PostDeclModPass as HirPass>::VarRef;

    type TypeRef = <PostDeclModPass as HirPass>::TypeRef;

    type LabelDcl = <PostDeclModPass as HirPass>::LabelDcl; // TODO: actually implement this
    type LabelRef = <PostDeclModPass as HirPass>::LabelRef;

    type Literal = <PostDeclModPass as HirPass>::Literal;
}

pub struct DeclVisitor<'env, 'src, 'err> {
    source: &'src SourceMap,
    env: &'env mut ScopeEnvironment,
    diagnostics: &'err mut Vec<Diagnostic>,
    mod_id: ModId,
    file: FileId,
}

impl<'env, 'src, 'err> DeclVisitor<'env, 'src, 'err> {
    pub const fn new(
        source: &'src SourceMap,
        env: &'env mut ScopeEnvironment,
        diagnostics: &'err mut Vec<Diagnostic>,
        mod_id: ModId,
        file: FileId,
    ) -> Self {
        Self {
            source,
            env,
            diagnostics,
            mod_id,
            file,
        }
    }
}

impl hir::Visitor<PostDeclModPass> for DeclVisitor<'_, '_, '_> {
    type Output = PostDeclPass;
}

impl hir::ScopeVisitor for DeclVisitor<'_, '_, '_> {
    fn push_layer(&mut self) {
        self.env[self.mod_id].push_layer();
    }

    fn pop_layer(&mut self) {
        self.env[self.mod_id].pop_layer();
    }

    fn push_scope(&mut self) {
        self.env[self.mod_id].push_scope();
    }

    fn pop_scope(&mut self) {
        self.env[self.mod_id].pop_scope();
    }

    fn clear_scopes(&mut self) {
        self.env[self.mod_id].clear_scopes();
    }
}

impl hir::ItemDclVisitor<hir::UnlinkedDecl> for DeclVisitor<'_, '_, '_> {
    type Value = hir::LinkedDecl;

    fn visit_item_dcl(&mut self, x: hir::UnlinkedDecl) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedDecl { kind, ident } = x;
        let ident = ident.within_file(self.file);
        match self
            .env
            .decl_item(self.mod_id, self.source[ident].to_owned(), kind, ident)
        {
            Ok(id) => Ok(hir::LinkedDecl(id)),
            Err(e) => {
                self.diagnostics.push(Diagnostic::new(
                    ident,
                    DiagnosticKind::Other {
                        severity: Severity::Error,
                        category: StaticAnalysis,
                        msg: format!(
                            "duplicate item: first declaration at {}",
                            self.env
                                .get_decl(e.og)
                                .expect("duplicate item error og should exist")
                                .decl
                                .display_loc(self.source)
                        ),
                    },
                ));
                Err(CheckDiagnostics)
            }
        }
    }
}

impl hir::VarDclVisitor<hir::UnlinkedDecl> for DeclVisitor<'_, '_, '_> {
    type Value = hir::LinkedDecl;

    fn visit_var_dcl(&mut self, x: hir::UnlinkedDecl) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedDecl { kind, ident } = x;
        let ident = ident.within_file(self.file);
        Ok(hir::LinkedDecl(self.env.decl_var(
            self.mod_id,
            self.source[ident].to_owned(),
            kind,
            ident,
        )))
    }
}

impl hir::IgnoreModDcl<<PostDeclModPass as HirPass>::ModDcl> for DeclVisitor<'_, '_, '_> {}
// impl hir::IgnoreItemDcl<<PostDeclModPass as HirPass>::ItemDcl> for DeclVisitor<'_, '_, '_, > {}
impl hir::IgnoreItemRef<<PostDeclModPass as HirPass>::ItemRef> for DeclVisitor<'_, '_, '_> {}
// impl hir::IgnoreVarDcl<<PostDeclModPass as HirPass>::VarDcl> for DeclVisitor<'_, '_, '_, > {}
impl hir::IgnoreVarRef<<PostDeclModPass as HirPass>::VarRef> for DeclVisitor<'_, '_, '_> {}
impl hir::IgnoreTypeRef<<PostDeclModPass as HirPass>::TypeRef> for DeclVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelDcl<<PostDeclModPass as HirPass>::LabelDcl> for DeclVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelRef<<PostDeclModPass as HirPass>::LabelRef> for DeclVisitor<'_, '_, '_> {}
impl hir::IgnoreLiteral<<PostDeclModPass as HirPass>::Literal> for DeclVisitor<'_, '_, '_> {}
// impl hir::IgnoreScope for DeclVisitor<'_, '_, '_> {}

pub struct PostVarRefPass;

impl HirPass for PostVarRefPass {
    type ModDcl = <PostDeclPass as HirPass>::ModDcl;

    type ItemDcl = <PostDeclPass as HirPass>::ItemDcl;
    type ItemRef = hir::LinkedVar; // TODO: distinguish from var

    type VarDcl = <PostDeclPass as HirPass>::VarDcl; // TODO: distinguish from item
    type VarRef = hir::LinkedVar;

    type TypeRef = hir::LinkedTypename;

    type LabelDcl = <PostDeclPass as HirPass>::LabelDcl; // TODO: actually implement this
    type LabelRef = <PostDeclPass as HirPass>::LabelRef;

    type Literal = <PostDeclPass as HirPass>::Literal;
}

pub struct VarRefVisitor<'env, 'src, 'err> {
    source: &'src SourceMap,
    env: &'env mut ScopeEnvironment,
    diagnostics: &'err mut Vec<Diagnostic>,
    mod_id: ModId,
    file: FileId,
}

impl<'env, 'src, 'err> VarRefVisitor<'env, 'src, 'err> {
    pub const fn new(
        source: &'src SourceMap,
        env: &'env mut ScopeEnvironment,
        diagnostics: &'err mut Vec<Diagnostic>,
        mod_id: ModId,
        file: FileId,
    ) -> Self {
        Self {
            source,
            env,
            diagnostics,
            mod_id,
            file,
        }
    }

    fn lookup_mod_path<I: IntoIterator<Item = RelSpan>>(
        &mut self,
        path: I,
    ) -> Result<ModId, CheckDiagnostics> {
        #[cfg(all(debug_assertions, false))]
        self.env.debug_exports();

        let mut path_iter = path.into_iter().map(|span| span.within_file(self.file));
        Ok(if let Some(root) = path_iter.next() {
            let name = &self.source[root];
            #[cfg(all(debug_assertions, false))]
            println!("  looking for library {name}");
            let mut id = self.env.find_library(name).ok_or_else(|| {
                self.diagnostics.push(Diagnostic::new(
                    root,
                    DiagnosticKind::Other {
                        severity: Severity::Error,
                        category: StaticAnalysis,
                        msg: "no library with this name exists in the project".to_owned(),
                    },
                ));
                CheckDiagnostics
            })?;
            #[cfg(all(debug_assertions, false))]
            println!("  found {id} exporting: {:?}", self.env[id]);
            let mut prev_name = name;
            for submod in path_iter {
                let name = &self.source[submod];
                #[cfg(all(debug_assertions, false))]
                println!("  looking for submodule {name}");
                id = self.env[id]
                    .find_submodule(&self.source[submod])
                    .ok_or_else(|| {
                        self.diagnostics.push(Diagnostic::new(
                            root,
                            DiagnosticKind::Other {
                                severity: Severity::Error,
                                category: StaticAnalysis,
                                msg: format!("no submodule with this name exists in {prev_name}"),
                            },
                        ));
                        CheckDiagnostics
                    })?;
                #[cfg(all(debug_assertions, false))]
                println!("  found {id} exporting: {:?}", self.env[id]);
                prev_name = name;
            }
            #[cfg(all(debug_assertions, false))]
            println!("  final module: {id}");
            id
        } else {
            self.mod_id
        })
    }
}

impl hir::Visitor<PostDeclPass> for VarRefVisitor<'_, '_, '_> {
    type Output = PostVarRefPass;
}

// rebuild scope
impl hir::ScopeVisitor for VarRefVisitor<'_, '_, '_> {
    fn push_layer(&mut self) {
        self.env[self.mod_id].push_layer();
    }

    fn pop_layer(&mut self) {
        self.env[self.mod_id].pop_layer();
    }

    fn push_scope(&mut self) {
        self.env[self.mod_id].push_scope();
    }

    fn pop_scope(&mut self) {
        self.env[self.mod_id].pop_scope();
    }

    fn clear_scopes(&mut self) {
        self.env[self.mod_id].clear_scopes();
    }
}

// needed to rebuild scope visibility, does not actually modify the tree
impl hir::ItemDclVisitor<hir::LinkedDecl> for VarRefVisitor<'_, '_, '_> {
    type Value = hir::LinkedDecl;

    fn visit_item_dcl(&mut self, x: hir::LinkedDecl) -> Result<Self::Value, CheckDiagnostics> {
        let hir::LinkedDecl(id) = x;
        let ident = self.env.get_decl(id).expect(
            "all IDs in the HIR should be valid if there were no errors during the declaration pass",
        ).decl;
        let name = self.source[ident].to_owned();
        // important: `env.decl_*()` actually produces a new item ID, `env[self.mod_id].decl_*()` only pushes an ID to the scope!
        let res = self.env[self.mod_id].decl_item(name, id);
        debug_assert_eq!(
            res,
            Ok(()),
            "should have errored in declaration pass\n attempted: {id}"
        );
        Ok(x)
    }
}

// needed to rebuild scope visibility, does not actually modify the tree
impl hir::VarDclVisitor<hir::LinkedDecl> for VarRefVisitor<'_, '_, '_> {
    type Value = hir::LinkedDecl;

    fn visit_var_dcl(&mut self, x: hir::LinkedDecl) -> Result<Self::Value, CheckDiagnostics> {
        let hir::LinkedDecl(id) = x;
        let ident = self.env.get_decl(id).expect(
            "all IDs in the HIR should be valid if there were no errors during the declaration pass",
        ).decl;
        let name = self.source[ident].to_owned();
        // important: `env.decl_*()` actually produces a new item ID, `env[self.mod_id].decl_*()` only pushes an ID to the scope!
        self.env[self.mod_id].decl_var(name, id);
        Ok(x)
    }
}

// TODO: items can be things other than variables!!
impl hir::ItemRefVisitor<hir::UnlinkedVar> for VarRefVisitor<'_, '_, '_> {
    type Value = hir::LinkedVar;

    fn visit_item_ref(&mut self, x: hir::UnlinkedVar) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedVar(ast::Variable { path, ident }) = x;
        let mod_id = self.lookup_mod_path(
            path.into_iter()
                .flatten()
                .map(|ast::ModPathPart { ident, sep: _ }| ident),
        )?;
        let ident = ident.within_file(self.file);
        let name = &self.source[ident];
        self.env[mod_id]
            .lookup_item(name)
            .map(hir::LinkedVar)
            .ok_or_else(|| {
                self.diagnostics
                    .push(Diagnostic::new(ident, DiagnosticKind::UnknownVar));
                CheckDiagnostics
            })
    }
}

impl hir::VarRefVisitor<hir::UnlinkedVar> for VarRefVisitor<'_, '_, '_> {
    type Value = hir::LinkedVar;

    fn visit_var_ref(&mut self, x: hir::UnlinkedVar) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedVar(ast::Variable { path, ident }) = x;
        let mod_id = self.lookup_mod_path(
            path.into_iter()
                .flatten()
                .map(|ast::ModPathPart { ident, sep: _ }| ident),
        )?;
        let ident = ident.within_file(self.file);
        let name = &self.source[ident];
        self.env[mod_id]
            .lookup_var(name)
            // might be a function
            .or_else(|| self.env[mod_id].lookup_item(name))
            .map(hir::LinkedVar)
            .ok_or_else(|| {
                self.diagnostics
                    .push(Diagnostic::new(ident, DiagnosticKind::UnknownVar));
                CheckDiagnostics
            })
    }
}

impl hir::TypeRefVisitor<hir::UnlinkedTypename> for VarRefVisitor<'_, '_, '_> {
    type Value = hir::LinkedTypename;

    fn visit_type_ref(
        &mut self,
        x: hir::UnlinkedTypename,
    ) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedTypename(ast::Typename { path, name }) = x;
        if path.is_empty() {
            let ident = name.within_file(self.file);
            let name = &self.source[ident];
            self.env[self.mod_id]
                .lookup_item(name)
                .map(hir::LinkedTypename)
                .ok_or_else(|| {
                    self.diagnostics
                        .push(Diagnostic::new(ident, DiagnosticKind::UnknownVar));
                    CheckDiagnostics
                })
        } else {
            Ok(todo!("type path"))
        }
    }
}

impl hir::IgnoreModDcl<<PostDeclPass as HirPass>::ModDcl> for VarRefVisitor<'_, '_, '_> {}
// impl hir::IgnoreItemDcl<<PostDeclPass as HirPass>::ItemDcl> for VarRefVisitor<'_, '_, '_, > {}
// impl hir::IgnoreItemRef<<PostDeclPass as HirPass>::ItemRef> for VarRefVisitor<'_, '_, '_, > {}
// impl hir::IgnoreVarDcl<<PostDeclPass as HirPass>::VarDcl> for VarRefVisitor<'_, '_, '_, > {}
// impl hir::IgnoreVarRef<<PostDeclPass as HirPass>::VarRef> for VarRefVisitor<'_, '_, '_, > {}
// impl hir::IgnoreTypeRef<<PostDeclPass as HirPass>::TypeRef> for VarRefVisitor<'_, '_, '_, > {}
impl hir::IgnoreLabelDcl<<PostDeclPass as HirPass>::LabelDcl> for VarRefVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelRef<<PostDeclPass as HirPass>::LabelRef> for VarRefVisitor<'_, '_, '_> {}
impl hir::IgnoreLiteral<<PostDeclPass as HirPass>::Literal> for VarRefVisitor<'_, '_, '_> {}
// impl hir::IgnoreScope for VarRefVisitor<'_, '_, '_> {}

pub struct LitVisitor<'env, 'src, 'err> {
    source: &'src SourceMap,
    env: &'env mut ScopeEnvironment,
    diagnostics: &'err mut Vec<Diagnostic>,
    mod_id: ModId,
    file: FileId,
}

impl<'env, 'src, 'err> LitVisitor<'env, 'src, 'err> {
    pub const fn new(
        source: &'src SourceMap,
        env: &'env mut ScopeEnvironment,
        diagnostics: &'err mut Vec<Diagnostic>,
        mod_id: ModId,
        file: FileId,
    ) -> Self {
        Self {
            source,
            env,
            diagnostics,
            mod_id,
            file,
        }
    }
}

impl hir::Visitor<PostVarRefPass> for LitVisitor<'_, '_, '_> {
    type Output = hir::FinalPass;
}

impl hir::LiteralVisitor<hir::UnlinkedLiteral> for LitVisitor<'_, '_, '_> {
    type Value = hir::LinkedLiteral;

    fn visit_literal(&mut self, x: hir::UnlinkedLiteral) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedLiteral(ast::Literal { kind, span }) = x;
        let span = span.within_file(self.file);
        let lex = &self.source[span];
        Ok(match kind {
            ast::LiteralKind::None => hir::LinkedLiteral::None,

            ast::LiteralKind::Bool(x) => hir::LinkedLiteral::Bool(x),

            ast::LiteralKind::Number => match lex.parse::<NumberLiteral>().map_err(|e| {
                self.diagnostics.push(Diagnostic::new(span, e.into()));
                CheckDiagnostics
            })? {
                NumberLiteral::Byte(x) => hir::LinkedLiteral::Byte(x),
                NumberLiteral::Uint(x) => hir::LinkedLiteral::Uint(x),
                NumberLiteral::Sint(x) => hir::LinkedLiteral::Sint(x),
                NumberLiteral::Flpt(x) => hir::LinkedLiteral::Flpt(x),
            },

            ast::LiteralKind::Char => hir::LinkedLiteral::Char(
                lex.parse::<CharLiteral>()
                    .map_err(|e| {
                        self.diagnostics.push(Diagnostic::new(
                            span,
                            e.try_into().expect("should include delimiters"),
                        ));
                        CheckDiagnostics
                    })?
                    .0,
            ),

            ast::LiteralKind::Text => hir::LinkedLiteral::Text(
                lex.parse::<TextLiteral>()
                    .map_err(|e| {
                        self.diagnostics.push(Diagnostic::new(
                            span,
                            e.try_into().expect("should include delimiters"),
                        ));
                        CheckDiagnostics
                    })?
                    .0,
            ),

            ast::LiteralKind::Macro => todo!("error: should not have any macros at this point"),
        })
    }
}

impl hir::IgnoreModDcl<<PostVarRefPass as HirPass>::ModDcl> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreItemDcl<<PostVarRefPass as HirPass>::ItemDcl> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreItemRef<<PostVarRefPass as HirPass>::ItemRef> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreVarDcl<<PostVarRefPass as HirPass>::VarDcl> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreVarRef<<PostVarRefPass as HirPass>::VarRef> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreTypeRef<<PostVarRefPass as HirPass>::TypeRef> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelDcl<<PostVarRefPass as HirPass>::LabelDcl> for LitVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelRef<<PostVarRefPass as HirPass>::LabelRef> for LitVisitor<'_, '_, '_> {}
// impl hir::IgnoreLiteral<<PostVarRefPass as HirPass>::Literal> for LiteralVisitor<'_, '_, '_> {}
impl hir::IgnoreScope for LitVisitor<'_, '_, '_> {}
