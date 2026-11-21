use crate::{
    analyzer::static_env::{
        ModId,
        scope::{
            env::{ModScopeEnvironment, ScopeEnvironment},
            hir::{self, HirPass, UnlinkedPass},
        },
    },
    error::{
        CheckDiagnostics, Diagnostic, DiagnosticCategory::StaticAnalysis, DiagnosticKind, Severity,
    },
    parse::ast::AstNode as _,
    src_code::{map::SourceMap, span::FileId},
};
use std::collections::BTreeMap;

pub struct PostDeclModPass;

impl HirPass for PostDeclModPass {
    type ModDcl = hir::LinkedMod;
    type ModRef = <hir::UnlinkedPass as HirPass>::ModRef;

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

impl hir::Visitor<UnlinkedPass> for DeclModVisitor<'_, '_, '_> {
    type Output = PostDeclModPass;
}

// impl hir::IgnoreModDcl<<UnlinkedPass as HirPass>::ModDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreModRef<<UnlinkedPass as HirPass>::ModRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreItemDcl<<UnlinkedPass as HirPass>::ItemDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreItemRef<<UnlinkedPass as HirPass>::ItemRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreVarDcl<<UnlinkedPass as HirPass>::VarDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreVarRef<<UnlinkedPass as HirPass>::VarRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreTypeRef<<UnlinkedPass as HirPass>::TypeRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelDcl<<UnlinkedPass as HirPass>::LabelDcl> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreLabelRef<<UnlinkedPass as HirPass>::LabelRef> for DeclModVisitor<'_, '_, '_> {}
impl hir::IgnoreLiteral<<UnlinkedPass as HirPass>::Literal> for DeclModVisitor<'_, '_, '_> {}

pub struct PostDeclPass;

impl HirPass for PostDeclPass {
    type ModDcl = <PostDeclModPass as HirPass>::ModDcl;
    type ModRef = <PostDeclModPass as HirPass>::ModRef;

    type ItemDcl = hir::LinkedDecl;
    type ItemRef = <PostDeclModPass as HirPass>::ItemRef; // TODO: distinguish from var

    type VarDcl = hir::LinkedDecl; // TODO: distinguish from item
    type VarRef = <PostDeclModPass as HirPass>::VarRef;

    type TypeRef = <PostDeclModPass as HirPass>::TypeRef;

    type LabelDcl = <PostDeclModPass as HirPass>::LabelDcl; // TODO: actually implement this
    type LabelRef = <PostDeclModPass as HirPass>::LabelRef;

    type Literal = <PostDeclModPass as HirPass>::Literal;
}

pub struct DeclVisitor<'env, 'm_env, 'src, 'err> {
    source: &'src SourceMap,
    env: &'m_env mut ModScopeEnvironment<'env>,
    diagnostics: &'err mut Vec<Diagnostic>,
    file: FileId,
}

impl<'env, 'm_env, 'src, 'err> DeclVisitor<'env, 'm_env, 'src, 'err> {
    pub const fn new(
        source: &'src SourceMap,
        env: &'m_env mut ModScopeEnvironment<'env>,
        diagnostics: &'err mut Vec<Diagnostic>,
        file: FileId,
    ) -> Self {
        Self {
            source,
            env,
            diagnostics,
            file,
        }
    }
}

impl hir::ItemDclVisitor<hir::UnlinkedDecl> for DeclVisitor<'_, '_, '_, '_> {
    type Value = hir::LinkedDecl;

    fn visit_item_dcl(&mut self, x: hir::UnlinkedDecl) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedDecl(name) = x;
        let ident = name.within_file(self.file);
        match self.env.decl_item(self.source[ident].to_owned(), ident) {
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

impl hir::VarDclVisitor<hir::UnlinkedDecl> for DeclVisitor<'_, '_, '_, '_> {
    type Value = hir::LinkedDecl;

    fn visit_var_dcl(&mut self, x: hir::UnlinkedDecl) -> Result<Self::Value, CheckDiagnostics> {
        let hir::UnlinkedDecl(name) = x;
        let ident = name.within_file(self.file);
        let id = self.env.decl_var(self.source[ident].to_owned(), ident);
        Ok(hir::LinkedDecl(id))
    }
}

impl hir::Visitor<PostDeclModPass> for DeclVisitor<'_, '_, '_, '_> {
    type Output = PostDeclPass;
}

impl hir::IgnoreModDcl<<PostDeclModPass as HirPass>::ModDcl> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreModRef<<PostDeclModPass as HirPass>::ModRef> for DeclVisitor<'_, '_, '_, '_> {}
// impl hir::IgnoreItemDcl<<PostDeclModPass as HirPass>::ItemDcl> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreItemRef<<PostDeclModPass as HirPass>::ItemRef> for DeclVisitor<'_, '_, '_, '_> {}
// impl hir::IgnoreVarDcl<<PostDeclModPass as HirPass>::VarDcl> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreVarRef<<PostDeclModPass as HirPass>::VarRef> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreTypeRef<<PostDeclModPass as HirPass>::TypeRef> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreLabelDcl<<PostDeclModPass as HirPass>::LabelDcl> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreLabelRef<<PostDeclModPass as HirPass>::LabelRef> for DeclVisitor<'_, '_, '_, '_> {}
impl hir::IgnoreLiteral<<PostDeclModPass as HirPass>::Literal> for DeclVisitor<'_, '_, '_, '_> {}
