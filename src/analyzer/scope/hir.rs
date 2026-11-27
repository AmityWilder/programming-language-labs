//! Type Intermediate Representation

#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    reason = "need to be aware of when the structure changes"
)]

use crate::{
    analyzer::{
        static_env::{ItemId, LabelId, ModId, TypeId, VarId, scope::ItemKind},
        types::bounds::Combinator,
    },
    error::CheckDiagnostics,
    lex::token::{Operator, punc::OpSide},
    parse::ast,
    src_code::span::{FileId, RelSpan},
    types::{byte, flpt, sint, text, uint},
    util::OrAllErrsExt as _,
};
use std::collections::VecDeque;

fn convert_iter<U, A: IntoIterator<Item: Into<U>>, B: FromIterator<U>>(vec: A) -> B {
    vec.into_iter().map(Into::into).collect::<B>()
}

pub trait HirPass {
    type ModDcl: super::fmt::HirDisplay;

    type ItemDcl: super::fmt::HirDisplay;
    type ItemRef: super::fmt::HirDisplay;

    type VarDcl: super::fmt::HirDisplay;
    type VarRef: super::fmt::HirDisplay;

    type TypeRef: super::fmt::HirDisplay;

    type LabelDcl: super::fmt::HirDisplay;
    type LabelRef: super::fmt::HirDisplay;

    type Literal: super::fmt::HirDisplay;
}

pub struct UnlinkedPass;

impl HirPass for UnlinkedPass {
    type ModDcl = UnlinkedMod;

    type ItemDcl = UnlinkedDecl;
    type ItemRef = UnlinkedVar;

    type VarDcl = UnlinkedDecl;
    type VarRef = UnlinkedVar;

    type TypeRef = UnlinkedTypename;

    type LabelDcl = UnlinkedLabel;
    type LabelRef = UnlinkedLabel;

    type Literal = UnlinkedLiteral;
}

pub struct FinalPass;

impl HirPass for FinalPass {
    type ModDcl = LinkedMod;

    type ItemDcl = LinkedDecl;
    type ItemRef = LinkedVar;

    type VarDcl = LinkedDecl;
    type VarRef = LinkedVar;

    type TypeRef = LinkedTypename;

    type LabelDcl = LinkedLabel;
    type LabelRef = LinkedLabel;

    type Literal = LinkedLiteral;
}

pub trait ModDclVisitor<T> {
    type Value;

    fn visit_mod_dcl(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreModDcl<T> {}

impl<T: IgnoreModDcl<U>, U> ModDclVisitor<U> for T {
    type Value = U;

    fn visit_mod_dcl(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait ModRefVisitor<T> {
    type Value;

    fn visit_mod_ref(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreModRef<T> {}

impl<T: IgnoreModRef<U>, U> ModRefVisitor<U> for T {
    type Value = U;

    fn visit_mod_ref(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait ItemDclVisitor<T> {
    type Value;

    fn visit_item_dcl(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreItemDcl<T> {}

impl<T: IgnoreItemDcl<U>, U> ItemDclVisitor<U> for T {
    type Value = U;

    fn visit_item_dcl(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait ItemRefVisitor<T> {
    type Value;

    fn visit_item_ref(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreItemRef<T> {}

impl<T: IgnoreItemRef<U>, U> ItemRefVisitor<U> for T {
    type Value = U;

    fn visit_item_ref(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait VarDclVisitor<T> {
    type Value;

    fn visit_var_dcl(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreVarDcl<T> {}

impl<T: IgnoreVarDcl<U>, U> VarDclVisitor<U> for T {
    type Value = U;

    fn visit_var_dcl(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait VarRefVisitor<T> {
    type Value;

    fn visit_var_ref(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreVarRef<T> {}

impl<T: IgnoreVarRef<U>, U> VarRefVisitor<U> for T {
    type Value = U;

    fn visit_var_ref(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait TypeRefVisitor<T> {
    type Value;

    fn visit_type_ref(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreTypeRef<T> {}

impl<T: IgnoreTypeRef<U>, U> TypeRefVisitor<U> for T {
    type Value = U;

    fn visit_type_ref(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait LabelDclVisitor<T> {
    type Value;

    fn visit_label_dcl(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreLabelDcl<T> {}

impl<T: IgnoreLabelDcl<U>, U> LabelDclVisitor<U> for T {
    type Value = U;

    fn visit_label_dcl(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait LabelRefVisitor<T> {
    type Value;

    fn visit_label_ref(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreLabelRef<T> {}

impl<T: IgnoreLabelRef<U>, U> LabelRefVisitor<U> for T {
    type Value = U;

    fn visit_label_ref(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait LiteralVisitor<T> {
    type Value;

    fn visit_literal(&mut self, x: T) -> Result<Self::Value, CheckDiagnostics>;
}

/// Marker trait
pub trait IgnoreLiteral<T> {}

impl<T: IgnoreLiteral<U>, U> LiteralVisitor<U> for T {
    type Value = U;

    fn visit_literal(&mut self, x: U) -> Result<Self::Value, CheckDiagnostics> {
        Ok(x)
    }
}

pub trait ScopeVisitor {
    fn push_layer(&mut self);

    /// Popping a layer should also pop all scopes within that layer, so calling `pop_scope` is unnecessary.
    fn pop_layer(&mut self);

    fn push_scope(&mut self);

    fn pop_scope(&mut self);

    /// Clears the scope tree in the current module, for a fresh rebuild.
    /// Called before the first node is walked.
    fn clear_scopes(&mut self);
}

pub trait IgnoreScope {}

impl<T: IgnoreScope> ScopeVisitor for T {
    fn push_layer(&mut self) {}
    fn pop_layer(&mut self) {}
    fn push_scope(&mut self) {}
    fn pop_scope(&mut self) {}
    fn clear_scopes(&mut self) {}
}

pub trait Visitor<S: HirPass>:
    ScopeVisitor
    + ModDclVisitor<S::ModDcl>
    + ItemDclVisitor<S::ItemDcl>
    + ItemRefVisitor<S::ItemRef>
    + VarDclVisitor<S::VarDcl>
    + VarRefVisitor<S::VarRef>
    + TypeRefVisitor<S::TypeRef>
    + LabelDclVisitor<S::LabelDcl>
    + LabelRefVisitor<S::LabelRef>
    + LiteralVisitor<S::Literal>
{
    type Output: HirPass<
            ModDcl = <Self as ModDclVisitor<S::ModDcl>>::Value,
            ItemDcl = <Self as ItemDclVisitor<S::ItemDcl>>::Value,
            ItemRef = <Self as ItemRefVisitor<S::ItemRef>>::Value,
            VarDcl = <Self as VarDclVisitor<S::VarDcl>>::Value,
            VarRef = <Self as VarRefVisitor<S::VarRef>>::Value,
            TypeRef = <Self as TypeRefVisitor<S::TypeRef>>::Value,
            LabelDcl = <Self as LabelDclVisitor<S::LabelDcl>>::Value,
            LabelRef = <Self as LabelRefVisitor<S::LabelRef>>::Value,
            Literal = <Self as LiteralVisitor<S::Literal>>::Value,
        >;
}

pub trait TreeWalk<V, S>
where
    S: HirPass,
    V: Visitor<S>,
{
    type Output;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics>;
}

impl<V, S: HirPass, T> TreeWalk<V, S> for Box<T>
where
    V: Visitor<S>,
    T: TreeWalk<V, S>,
{
    type Output = Box<T::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        (*self).walk(visitor).map(Box::new)
    }
}

impl<V, S: HirPass, T> TreeWalk<V, S> for Option<T>
where
    V: Visitor<S>,
    T: TreeWalk<V, S>,
{
    type Output = Option<T::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        self.map(|x| x.walk(visitor)).transpose()
    }
}

impl<V, S: HirPass, T> TreeWalk<V, S> for Vec<T>
where
    V: Visitor<S>,
    T: TreeWalk<V, S>,
{
    type Output = Vec<T::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        self.into_iter()
            .map(|x| x.walk(visitor))
            .collect_or_acc_errs()
    }
}

impl<V, S: HirPass, T> TreeWalk<V, S> for VecDeque<T>
where
    V: Visitor<S>,
    T: TreeWalk<V, S>,
{
    type Output = VecDeque<T::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        self.into_iter()
            .map(|x| x.walk(visitor))
            .collect_or_acc_errs()
    }
}

impl<V, S: HirPass, T> TreeWalk<V, S> for Box<[T]>
where
    V: Visitor<S>,
    T: TreeWalk<V, S>,
{
    type Output = Box<[T::Output]>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        self.into_iter()
            .map(|x| x.walk(visitor))
            .collect_or_acc_errs()
            .map(Vec::into_boxed_slice)
    }
}

macro_rules! Walkable {
    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident<$S:ident: HirPass>(
            $(#[$fm:meta])*
            $fvis:vis $Field:ty
        $(,)?);
    } => {
        impl<V, S: HirPass> TreeWalk<V, S> for $Type<S>
        where
            V: Visitor<S>,
        {
            type Output = $Type<V::Output>;

            fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
                self.0.walk(visitor).map($Type)
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident<$S:ident: HirPass> {$(
            $(#[$fm:meta])*
            $fvis:vis $field:ident: $Field:ty
        ),* $(,)?}
    } => {
        impl<V, S: HirPass> TreeWalk<V, S> for $Type<S>
        where
            V: Visitor<S>,
        {
            type Output = $Type<V::Output>;

            fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
                let Self { $($field),* } = self;
                $(let $field = $field.walk(visitor);)*
                Ok($Type { $($field: $field?),* })
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis enum $Type:ident<$S:ident: HirPass> {$(
            $(#[$vm:meta])*
            $Variant:ident(
                $(#[$fm:meta])*
                $Field:ty
            $(,)?)
        ),* $(,)?}
    }=> {
        impl<V, S: HirPass> TreeWalk<V, S> for $Type<S>
        where
            V: Visitor<S>,
        {
            type Output = $Type<V::Output>;

            fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
                match self {
                    $(Self::$Variant(x) => x.walk(visitor).map($Type::$Variant)),*
                }
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident($($rest:tt)*);
    } => {
        impl<V, S: HirPass> TreeWalk<V, S> for $Type
        where
            V: Visitor<S>,
        {
            type Output = Self;

            fn walk(self, _: &mut V) -> Result<Self::Output, CheckDiagnostics> {
                Ok(self)
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident {$($rest:tt)*}
    } => {
        impl<V, S: HirPass> TreeWalk<V, S> for $Type
        where
            V: Visitor<S>,
        {
            type Output = Self;

            fn walk(self, _: &mut V) -> Result<Self::Output, CheckDiagnostics> {
                Ok(self)
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis enum $Type:ident {$($rest:tt)*}
    } => {
        impl<V, S: HirPass> TreeWalk<V, S> for $Type
        where
            V: Visitor<S>,
        {
            type Output = Self;

            fn walk(self, _: &mut V) -> Result<Self::Output, CheckDiagnostics> {
                Ok(self)
            }
        }
    };
}

pub struct HirUnit<S: HirPass> {
    pub file: FileId,
    pub decls: Box<[Decl<S>]>,
}

impl<V, S: HirPass> TreeWalk<V, S> for HirUnit<S>
where
    V: Visitor<S>,
{
    type Output = HirUnit<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { file, decls } = self;
        visitor.clear_scopes();
        let decls = decls.walk(visitor);
        Ok(HirUnit {
            file,
            decls: decls?,
        })
    }
}

impl From<ast::ProgramUnit> for HirUnit<UnlinkedPass> {
    fn from(ast::ProgramUnit { file, decls }: ast::ProgramUnit) -> Self {
        Self {
            file,
            decls: convert_iter(decls),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Walkable)]
pub enum PrimitiveType {
    #[default]
    None,
    Nevr,
    Fail,
    Bool,
    Byte,
    Uint,
    Sint,
    Flpt,
    Char,
    Text,
}

impl From<ast::TypenameKeyword> for PrimitiveType {
    fn from(value: ast::TypenameKeyword) -> Self {
        match value {
            ast::TypenameKeyword::None => Self::None,
            ast::TypenameKeyword::Nevr => Self::Nevr,
            ast::TypenameKeyword::Fail => Self::Fail,
            ast::TypenameKeyword::Bool => Self::Bool,
            ast::TypenameKeyword::Byte => Self::Byte,
            ast::TypenameKeyword::Uint => Self::Uint,
            ast::TypenameKeyword::Sint => Self::Sint,
            ast::TypenameKeyword::Flpt => Self::Flpt,
            ast::TypenameKeyword::Char => Self::Char,
            ast::TypenameKeyword::Text => Self::Text,
        }
    }
}

impl From<ast::PrimitiveType> for PrimitiveType {
    fn from(ast::PrimitiveType { ty, span: _ }: ast::PrimitiveType) -> Self {
        ty.into()
    }
}

#[derive(Walkable)]
pub struct NamespacePathPart {
    /// Identifier
    pub name: RelSpan, // TODO
}

impl From<ast::NamespacePathPart> for NamespacePathPart {
    fn from(ast::NamespacePathPart { name, sep: _ }: ast::NamespacePathPart) -> Self {
        Self { name }
    }
}

pub struct UnlinkedTypename(pub ast::Typename);

impl From<ast::Typename> for UnlinkedTypename {
    fn from(value: ast::Typename) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedTypename(pub TypeId);

pub struct GenericParam<S: HirPass>(pub S::ItemDcl);

impl<V, S: HirPass> TreeWalk<V, S> for GenericParam<S>
where
    V: Visitor<S>,
{
    type Output = GenericParam<V::Output>;
    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        visitor.visit_item_dcl(self.0).map(GenericParam)
    }
}

impl From<RelSpan> for GenericParam<UnlinkedPass> {
    fn from(ident: RelSpan) -> Self {
        Self(UnlinkedDecl {
            kind: ItemKind::TypeParam,
            ident,
        })
    }
}

#[derive(Walkable)]
pub struct GenericParams<S: HirPass>(pub Box<[GenericParam<S>]>);

impl From<ast::GenericParams> for GenericParams<UnlinkedPass> {
    fn from(value: ast::GenericParams) -> Self {
        Self(convert_iter(value.list))
    }
}

#[derive(Walkable)]
pub struct GenericArgs<S: HirPass>(pub Box<[TypeExpr<S>]>);

impl From<ast::GenericArgs> for GenericArgs<UnlinkedPass> {
    fn from(value: ast::GenericArgs) -> Self {
        Self(convert_iter(value.list))
    }
}

pub struct TypeInstance<S: HirPass> {
    pub name: S::TypeRef,
    pub args: Option<GenericArgs<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for TypeInstance<S>
where
    V: Visitor<S>,
{
    type Output = TypeInstance<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { name, args } = self;
        let name = visitor.visit_type_ref(name);
        let args = args.walk(visitor);
        Ok(TypeInstance {
            name: name?,
            args: args?,
        })
    }
}

impl From<ast::TypeInstance> for TypeInstance<UnlinkedPass> {
    fn from(ast::TypeInstance { name, args }: ast::TypeInstance) -> Self {
        Self {
            name: name.into(),
            args: args.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Walkable)]
pub enum OrType {
    #[default]
    None,
    Fail,
    Nevr,
}

impl From<ast::OrTypeKeyword> for OrType {
    fn from(value: ast::OrTypeKeyword) -> Self {
        match value {
            ast::OrTypeKeyword::None => Self::None,
            ast::OrTypeKeyword::Nevr => Self::Nevr,
            ast::OrTypeKeyword::Fail => Self::Fail,
        }
    }
}

/// Fallible Type
#[derive(Walkable)]
pub struct FallType<S: HirPass> {
    pub then_ty: TypeExpr<S>,
    pub ty: OrType,
}

impl From<ast::FallType> for FallType<UnlinkedPass> {
    fn from(
        ast::FallType {
            then_ty,
            pipe: _,
            ty,
            ty_span: _,
        }: ast::FallType,
    ) -> Self {
        Self {
            then_ty: (*then_ty).into(),
            ty: ty.into(),
        }
    }
}

#[derive(Walkable)]
pub struct RefType<S: HirPass> {
    // TBD: reference mutability?
    pub ty: TypeExpr<S>,
}

impl From<ast::RefType> for RefType<UnlinkedPass> {
    fn from(ast::RefType { sym: _, ty }: ast::RefType) -> Self {
        Self { ty: (*ty).into() }
    }
}

#[derive(Walkable)]
pub struct ViewType<S: HirPass> {
    pub item: TypeExpr<S>,
    pub count: Option<Expr<S>>,
}

impl From<ast::ViewType> for ViewType<UnlinkedPass> {
    fn from(
        ast::ViewType {
            open: _,
            item,
            count,
            close: _,
        }: ast::ViewType,
    ) -> Self {
        Self {
            item: (*item).into(),
            count: count.map(|ast::ArrCount { semi: _, count }| count.into()),
        }
    }
}

pub enum AssocTypeSource<S: HirPass> {
    Simple(S::TypeRef),
    Grouped(TypeExpr<S>),
}

impl<V, S: HirPass> TreeWalk<V, S> for AssocTypeSource<S>
where
    V: Visitor<S>,
{
    type Output = AssocTypeSource<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Simple(x) => visitor.visit_type_ref(x).map(AssocTypeSource::Simple),
            Self::Grouped(x) => x.walk(visitor).map(AssocTypeSource::Grouped),
        }
    }
}

impl From<ast::AssocTypeSource> for AssocTypeSource<UnlinkedPass> {
    fn from(value: ast::AssocTypeSource) -> Self {
        match value {
            ast::AssocTypeSource::Simple(inner) => Self::Simple(inner.into()),
            ast::AssocTypeSource::Grouped(inner) => Self::Grouped(inner.inner.into()),
        }
    }
}

#[derive(Walkable)]
pub struct AssocType<S: HirPass> {
    pub base: AssocTypeSource<S>,
    pub assoc: TypeInstance<S>,
}

impl From<ast::AssocType> for AssocType<UnlinkedPass> {
    fn from(
        ast::AssocType {
            base,
            sep: _,
            assoc,
        }: ast::AssocType,
    ) -> Self {
        Self {
            base: base.into(),
            assoc: assoc.into(),
        }
    }
}

#[derive(Walkable)]
pub struct FunType<S: HirPass> {
    pub params: Box<[TypeExpr<S>]>,
    pub ret: Option<Box<TypeExpr<S>>>,
}

impl From<ast::FunType> for FunType<UnlinkedPass> {
    fn from(
        ast::FunType {
            kw: _,
            open: _,
            params,
            close: _,
            ret,
        }: ast::FunType,
    ) -> Self {
        Self {
            params: convert_iter(params),
            ret: ret.map(|ast::FunRet { arrow: _, ty }| Box::new(ty.into())),
        }
    }
}

#[derive(Walkable)]
pub struct OpaqueType<S: HirPass>(pub TypeBound<S>);

impl From<ast::OpaqueType> for OpaqueType<UnlinkedPass> {
    fn from(ast::OpaqueType { colon: _, bound }: ast::OpaqueType) -> Self {
        Self(bound.into())
    }
}

/// All/Any/One
pub struct ComboBound<S: HirPass> {
    pub cmb: Combinator,
    pub bounds: Box<[TypeBound<S>]>,
}

impl<V, S: HirPass> TreeWalk<V, S> for ComboBound<S>
where
    V: Visitor<S>,
{
    type Output = ComboBound<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { cmb, bounds } = self;
        let bounds = bounds.walk(visitor);
        Ok(ComboBound {
            cmb,
            bounds: bounds?,
        })
    }
}

impl From<ast::ComboBound> for ComboBound<UnlinkedPass> {
    fn from(
        ast::ComboBound {
            lhs,
            sep,
            sep_span: _,
            rhs,
        }: ast::ComboBound,
    ) -> Self {
        // TODO: chain?
        Self {
            cmb: sep,
            bounds: Box::from([lhs, rhs].map(Into::into)),
        }
    }
}

/// Not
#[derive(Walkable)]
pub struct NegativeBound<S: HirPass>(pub TypeBound<S>);

impl From<ast::NegativeBound> for NegativeBound<UnlinkedPass> {
    fn from(ast::NegativeBound { not: _, rhs }: ast::NegativeBound) -> Self {
        Self(rhs.into())
    }
}

/// Only
#[derive(Walkable)]
pub struct ExactTypeBound<S: HirPass>(pub TypeExpr<S>);

impl From<ast::ExactTypeBound> for ExactTypeBound<UnlinkedPass> {
    fn from(ast::ExactTypeBound { kw: _, ty }: ast::ExactTypeBound) -> Self {
        Self(ty.into())
    }
}

/// Has
pub struct MemberBound<S: HirPass> {
    /// Identifier
    // TBD: should this be an ID at any point before monomorphization?
    pub member: RelSpan,
    pub req: Option<TypeBound<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for MemberBound<S>
where
    V: Visitor<S>,
{
    type Output = MemberBound<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { member, req } = self;
        let req = req.walk(visitor);
        Ok(MemberBound { member, req: req? })
    }
}

impl From<ast::MemberBound> for MemberBound<UnlinkedPass> {
    fn from(ast::MemberBound { kw: _, member, req }: ast::MemberBound) -> Self {
        Self {
            member,
            req: req.map(|ast::ColonBound { colon: _, bound }| bound.into()),
        }
    }
}

/// Shape
#[derive(Walkable)]
pub struct ShapeBound<S: HirPass>(pub FunType<S>);

impl From<ast::FunType> for ShapeBound<UnlinkedPass> {
    fn from(value: ast::FunType) -> Self {
        Self(value.into())
    }
}

/// Supports
#[derive(Walkable)]
pub struct OverloadBound {
    // TODO
}

impl From<ast::OverloadBound> for OverloadBound {
    fn from(ast::OverloadBound {}: ast::OverloadBound) -> Self {
        Self {}
    }
}

/// Satisfies
pub struct ExprBound<S: HirPass> {
    // TBD: should this be a special ID since it never actually appears in the program?
    pub param: S::VarDcl,
    pub expr: Expr<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for ExprBound<S>
where
    V: Visitor<S>,
{
    type Output = ExprBound<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { param, expr } = self;
        visitor.push_layer();
        let param = visitor.visit_var_dcl(param);
        let expr = expr.walk(visitor);
        visitor.pop_layer();
        Ok(ExprBound {
            param: param?,
            expr: expr?,
        })
    }
}

impl From<ast::ExprBound> for ExprBound<UnlinkedPass> {
    fn from(
        ast::ExprBound {
            kw: _,
            name,
            colon: _,
            expr,
        }: ast::ExprBound,
    ) -> Self {
        Self {
            param: UnlinkedDecl {
                kind: ItemKind::BoundParam,
                ident: name,
            },
            expr: expr.into(),
        }
    }
}

#[derive(Walkable)]
pub enum TypeBound<S: HirPass> {
    Combo(ComboBound<S>),
    Not(Box<NegativeBound<S>>),
    Exact(ExactTypeBound<S>),
    Has(Box<MemberBound<S>>),
    Shape(ShapeBound<S>),
    Supports(Box<OverloadBound>),
    Satisfies(Box<ExprBound<S>>),
}

impl From<ast::TypeBound> for TypeBound<UnlinkedPass> {
    fn from(value: ast::TypeBound) -> Self {
        match value {
            ast::TypeBound::Combo(inner) => Self::Combo((*inner).into()),
            ast::TypeBound::Not(inner) => Self::Not(Box::new((*inner).into())),
            ast::TypeBound::Exact(inner) => Self::Exact((*inner).into()),
            ast::TypeBound::Has(inner) => Self::Has(Box::new((*inner).into())),
            ast::TypeBound::Shape(inner) => Self::Shape((*inner).into()),
            ast::TypeBound::Supports(inner) => Self::Supports(Box::new((*inner).into())),
            ast::TypeBound::Satisfies(inner) => Self::Satisfies(Box::new((*inner).into())),
            ast::TypeBound::Group(ast::GroupedBound {
                open: _,
                inner,
                close: _,
            }) => inner.into(),
        }
    }
}

#[derive(Walkable)]
pub enum TypeExpr<S: HirPass> {
    Primitive(PrimitiveType),
    Instance(TypeInstance<S>),
    Fallible(Box<FallType<S>>),
    Ref(Box<RefType<S>>),
    View(Box<ViewType<S>>),
    Assoc(Box<AssocType<S>>),
    FunPtr(FunType<S>),
    Opaque(Box<OpaqueType<S>>),
}

impl From<ast::TypeExpr> for TypeExpr<UnlinkedPass> {
    fn from(value: ast::TypeExpr) -> Self {
        match value {
            ast::TypeExpr::Primitive(inner) => Self::Primitive(inner.into()),
            ast::TypeExpr::Instance(inner) => Self::Instance(inner.into()),
            ast::TypeExpr::Fallible(inner) => Self::Fallible(Box::new(inner.into())),
            ast::TypeExpr::Ref(inner) => Self::Ref(Box::new(inner.into())),
            ast::TypeExpr::View(inner) => Self::View(Box::new(inner.into())),
            ast::TypeExpr::Assoc(inner) => Self::Assoc(Box::new((*inner).into())),
            ast::TypeExpr::FunPtr(inner) => Self::FunPtr(inner.into()),
            ast::TypeExpr::Opaque(inner) => Self::Opaque(Box::new(inner.into())),
        }
    }
}

impl From<ast::TypeAnnotation> for TypeExpr<UnlinkedPass> {
    fn from(ast::TypeAnnotation { colon: _, ty }: ast::TypeAnnotation) -> Self {
        ty.into()
    }
}

pub struct BindingPattern<S: HirPass> {
    pub var: S::VarDcl,
}

impl<V, S: HirPass> TreeWalk<V, S> for BindingPattern<S>
where
    V: Visitor<S>,
{
    type Output = BindingPattern<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { var } = self;
        let var = visitor.visit_var_dcl(var);
        Ok(BindingPattern { var: var? })
    }
}

#[derive(Walkable)]
pub struct PickPattern {
    // TODO
}

impl From<ast::PickPattern> for PickPattern {
    fn from(ast::PickPattern {}: ast::PickPattern) -> Self {
        Self {}
    }
}

#[derive(Walkable)]
pub enum Decl<S: HirPass> {
    Mod(DeclMod<S>),
    On(DeclOn<S>),
    Fun(DeclFun<S>),
    Rec(DeclRec<S>),
    Cat(DeclCat<S>),
    Uni(DeclUni<S>),
    Let(DeclLet<S>),
    Pvt(DeclPvt<S>),
    Dfer(DeclDfer<S>),
    Stmt(Stmt<S>),
}

impl From<ast::Decl> for Decl<UnlinkedPass> {
    fn from(value: ast::Decl) -> Self {
        match value {
            ast::Decl::Mod(inner) => Self::Mod(inner.into()),
            ast::Decl::On(inner) => Self::On(inner.into()),
            ast::Decl::Var(ast::DeclVar {
                kw,
                kw_span: _,
                binding,
                ty,
                assignment,
                semi: _,
            }) => {
                let ast::BindingPattern { ident } = binding;
                let ty = ty.map(Into::into);
                let init = assignment.map(Into::into);
                match kw {
                    ast::VarKeyword::Let => Self::Let(DeclLet {
                        id: UnlinkedDecl {
                            kind: ItemKind::Var,
                            ident,
                        },
                        ty,
                        init,
                    }),
                    ast::VarKeyword::Uni => Self::Uni(DeclUni {
                        id: UnlinkedDecl {
                            kind: ItemKind::Uni,
                            ident,
                        },
                        ty,
                        init,
                    }),
                    ast::VarKeyword::Pvt => Self::Pvt(DeclPvt {
                        id: UnlinkedDecl {
                            kind: ItemKind::Pvt,
                            ident,
                        },
                        ty,
                        // TODO: make this the parser's job
                        init: init.expect("pvt should have initializer"),
                    }),
                }
            }
            ast::Decl::Fun(inner) => Self::Fun(inner.into()),
            ast::Decl::Type(ast::DeclType::Rec(inner)) => Self::Rec(inner.into()),
            ast::Decl::Type(ast::DeclType::Cat(inner)) => Self::Cat(inner.into()),
            ast::Decl::Dfer(inner) => Self::Dfer(inner.into()),
            ast::Decl::Stmt(inner) => Self::Stmt(inner.into()),
        }
    }
}

pub struct DeclMod<S: HirPass> {
    pub id: S::ModDcl,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclMod<S>
where
    V: Visitor<S>,
{
    type Output = DeclMod<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id } = self;
        let id = visitor.visit_mod_dcl(id);
        Ok(DeclMod { id: id? })
    }
}

impl From<ast::DeclMod> for DeclMod<UnlinkedPass> {
    fn from(decl: ast::DeclMod) -> Self {
        Self { id: decl.into() }
    }
}

#[derive(Walkable)]
pub struct TypeBoundClause<S: HirPass> {
    pub target: TypeExpr<S>,
    pub bound: TypeBound<S>,
}

impl From<ast::TypeBoundClause> for TypeBoundClause<UnlinkedPass> {
    fn from(
        ast::TypeBoundClause {
            ty,
            colon: _,
            bound,
        }: ast::TypeBoundClause,
    ) -> Self {
        Self {
            target: ty.into(),
            bound: bound.into(),
        }
    }
}

#[derive(Walkable)]
pub struct IffClause<S: HirPass>(pub Box<[TypeBoundClause<S>]>);

impl From<ast::IffClause> for IffClause<UnlinkedPass> {
    fn from(ast::IffClause { iff: _, bounds }: ast::IffClause) -> Self {
        Self(convert_iter(bounds))
    }
}

pub struct DeclOn<S: HirPass> {
    pub generics: Option<GenericParams<S>>,
    pub target: TypeExpr<S>,
    pub iff: Option<IffClause<S>>,
    pub members: Box<[Decl<S>]>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclOn<S>
where
    V: Visitor<S>,
{
    type Output = DeclOn<V::Output>;
    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self {
            generics,
            target,
            iff,
            members,
        } = self;
        let generics = generics.walk(visitor);
        let target = target.walk(visitor);
        let iff = iff.walk(visitor);

        visitor.push_layer();
        let members = members.walk(visitor);
        visitor.pop_layer();

        Ok(DeclOn {
            generics: generics?,
            target: target?,
            iff: iff?,
            members: members?,
        })
    }
}

impl From<ast::DeclOn> for DeclOn<UnlinkedPass> {
    fn from(
        ast::DeclOn {
            kw: _,
            params,
            ty,
            iff,
            body:
                ast::BlockStmt {
                    open: _,
                    body,
                    close: _,
                },
        }: ast::DeclOn,
    ) -> Self {
        Self {
            generics: params.map(Into::into),
            target: ty.into(),
            iff: iff.map(Into::into),
            members: convert_iter(body),
        }
    }
}

pub struct SelfParam<S: HirPass> {
    pub id: S::VarDcl,
}

impl<V, S: HirPass> TreeWalk<V, S> for SelfParam<S>
where
    V: Visitor<S>,
{
    type Output = SelfParam<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id } = self;
        let id = visitor.visit_var_dcl(id);
        Ok(SelfParam { id: id? })
    }
}

impl From<ast::SelfParam> for SelfParam<UnlinkedPass> {
    fn from(ast::SelfParam { span }: ast::SelfParam) -> Self {
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Me,
                ident: span,
            },
        }
    }
}

pub struct DeclFun<S: HirPass> {
    pub id: S::ItemDcl,
    pub generics: Option<GenericParams<S>>,
    pub self_param: Option<SelfParam<S>>,
    pub params: Box<[FunParam<S>]>,
    pub ret: Option<TypeExpr<S>>,
    pub iff: Option<IffClause<S>>,
    pub body: Option<BlockStmt<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclFun<S>
where
    V: Visitor<S>,
{
    type Output = DeclFun<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self {
            id,
            generics,
            self_param,
            params,
            ret,
            iff,
            body,
        } = self;
        let id = visitor.visit_item_dcl(id);
        let generics = generics.walk(visitor);
        let ret = ret.walk(visitor);
        let iff = iff.walk(visitor);

        visitor.push_layer(); // Don't need to push a scope because layers start with a root scope
        let self_param = self_param.walk(visitor);
        let params = params.walk(visitor);
        let body = body.walk(visitor);
        visitor.pop_layer();

        Ok(DeclFun {
            id: id?,
            generics: generics?,
            self_param: self_param?,
            params: params?,
            ret: ret?,
            iff: iff?,
            body: body?,
        })
    }
}

impl From<ast::DeclFun> for DeclFun<UnlinkedPass> {
    fn from(value: ast::DeclFun) -> Self {
        let (
            ast::FnSignature {
                kw: _,
                name,
                generics,
                open: _,
                self_param,
                params,
                close: _,
                ret,
                iff,
            },
            body,
        ) = match value {
            ast::DeclFun::Declare(ast::AnnounceFun { head, semi: _ }) => (head, None),
            ast::DeclFun::Define(ast::DefFun { head, body }) => (head, Some(body)),
        };
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Fun,
                ident: name,
            },
            generics: generics.map(Into::into),
            self_param: self_param.map(Into::into),
            params: convert_iter(params),
            ret: ret.map(|ast::FunRet { arrow: _, ty }| ty.into()),
            iff: iff.map(Into::into),
            body: body.map(Into::into),
        }
    }
}

pub struct FunParam<S: HirPass> {
    /// Treated as a member of the function
    pub id: S::VarDcl,
    pub ty: TypeExpr<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for FunParam<S>
where
    V: Visitor<S>,
{
    type Output = FunParam<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty } = self;
        let id = visitor.visit_var_dcl(id);
        let ty = ty.walk(visitor);
        Ok(FunParam { id: id?, ty: ty? })
    }
}

impl From<ast::FnParam> for FunParam<UnlinkedPass> {
    fn from(ast::FnParam { name, ty }: ast::FnParam) -> Self {
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Param,
                ident: name,
            },
            ty: ty.into(),
        }
    }
}

pub struct DeclRec<S: HirPass> {
    pub id: S::ItemDcl,
    pub generics: Option<GenericParams<S>>,
    pub fields: Box<[Field<S>]>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclRec<S>
where
    V: Visitor<S>,
{
    type Output = DeclRec<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self {
            id,
            generics,
            fields,
        } = self;
        let id = visitor.visit_item_dcl(id);
        let generics = generics.walk(visitor);

        visitor.push_layer();
        // TODO: need to associate the fields with the parent
        let fields = fields.walk(visitor);
        visitor.pop_layer();

        Ok(DeclRec {
            id: id?,
            generics: generics?,
            fields: fields?,
        })
    }
}

impl From<ast::DeclRec> for DeclRec<UnlinkedPass> {
    fn from(
        ast::DeclRec {
            kw: _,
            name,
            generics,
            body:
                ast::FieldSet {
                    open: _,
                    fields,
                    close: _,
                },
        }: ast::DeclRec,
    ) -> Self {
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Rec,
                ident: name,
            },
            generics: generics.map(Into::into),
            fields: convert_iter(fields),
        }
    }
}

pub struct Field<S: HirPass> {
    pub id: S::ItemDcl, // TODO: these should be treated like unique items
    pub ty: TypeExpr<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for Field<S>
where
    V: Visitor<S>,
{
    type Output = Field<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty } = self;
        let id = visitor.visit_item_dcl(id);
        let ty = ty.walk(visitor);
        Ok(Field { id: id?, ty: ty? })
    }
}

impl From<ast::Field> for Field<UnlinkedPass> {
    fn from(ast::Field { name, ty }: ast::Field) -> Self {
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Field,
                ident: name,
            },
            ty: ty.into(),
        }
    }
}

pub struct DeclCat<S: HirPass> {
    pub id: S::ItemDcl, // TODO: these should be treated like unique items
    pub discriminants: Box<[Discriminant<S>]>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclCat<S>
where
    V: Visitor<S>,
{
    type Output = DeclCat<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, discriminants } = self;
        let id = visitor.visit_item_dcl(id);
        let discriminants = discriminants.walk(visitor);
        Ok(DeclCat {
            id: id?,
            discriminants: discriminants?,
        })
    }
}

impl From<ast::DeclCat> for DeclCat<UnlinkedPass> {
    fn from(
        ast::DeclCat {
            kw: _,
            name,
            discriminants:
                ast::CatDiscSet {
                    open: _,
                    items,
                    close: _,
                },
        }: ast::DeclCat,
    ) -> Self {
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Cat,
                ident: name,
            },
            discriminants: convert_iter(items),
        }
    }
}

pub struct Discriminant<S: HirPass> {
    pub id: S::ItemDcl,
    pub val: Option<Expr<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for Discriminant<S>
where
    V: Visitor<S>,
{
    type Output = Discriminant<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, val } = self;
        let id = visitor.visit_item_dcl(id);
        let val = val.walk(visitor);
        Ok(Discriminant { id: id?, val: val? })
    }
}

impl From<ast::CatDisc> for Discriminant<UnlinkedPass> {
    fn from(ast::CatDisc { name, val }: ast::CatDisc) -> Self {
        Self {
            id: UnlinkedDecl {
                kind: ItemKind::Discrim,
                ident: name,
            },
            val: val.map(Into::into),
        }
    }
}

pub struct DeclUni<S: HirPass> {
    pub id: S::ItemDcl,
    pub ty: Option<TypeExpr<S>>,
    pub init: Option<Expr<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclUni<S>
where
    V: Visitor<S>,
{
    type Output = DeclUni<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty, init } = self;
        let ty = ty.walk(visitor);
        let init = init.walk(visitor);
        // ID must be declared AFTER initializer and type
        let id = visitor.visit_item_dcl(id);
        Ok(DeclUni {
            id: id?,
            ty: ty?,
            init: init?,
        })
    }
}

pub enum Expr<S: HirPass> {
    Block(BlockExpr<S>),
    Binary(Box<Binary<S>>),
    Unary(Box<Unary<S>>),
    Index(Box<Subscript<S>>),
    Call(Box<FnCall<S>>),
    Literal(S::Literal),
    Directive(Directive),
    Variable(S::VarRef),
}

impl<V, S: HirPass> TreeWalk<V, S> for Expr<S>
where
    V: Visitor<S>,
{
    type Output = Expr<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        match self {
            Self::Block(x) => x.walk(visitor).map(Expr::Block),
            Self::Binary(x) => x.walk(visitor).map(Expr::Binary),
            Self::Unary(x) => x.walk(visitor).map(Expr::Unary),
            Self::Index(x) => x.walk(visitor).map(Expr::Index),
            Self::Call(x) => x.walk(visitor).map(Expr::Call),
            Self::Literal(x) => visitor.visit_literal(x).map(Expr::Literal),
            Self::Directive(x) => x.walk(visitor).map(Expr::Directive),
            Self::Variable(x) => visitor.visit_var_ref(x).map(Expr::Variable),
        }
    }
}

impl From<ast::Expr> for Expr<UnlinkedPass> {
    fn from(value: ast::Expr) -> Self {
        match value {
            ast::Expr::Block(inner) => Self::Block(inner.into()),
            ast::Expr::Binary(inner) => Self::Binary(Box::new((*inner).into())),
            ast::Expr::Unary(inner) => Self::Unary(Box::new((*inner).into())),
            ast::Expr::Index(inner) => Self::Index(Box::new((*inner).into())),
            ast::Expr::Call(inner) => Self::Call(Box::new((*inner).into())),
            ast::Expr::Literal(inner) => Self::Literal(inner.into()),
            ast::Expr::Directive(inner) => Self::Directive(inner.into()),
            ast::Expr::Member(_) => todo!("members"),
            ast::Expr::Variable(inner) => Self::Variable(inner.into()),
            ast::Expr::Grouping(inner) => (*inner).into(),
            ast::Expr::Type(_) => todo!("type in expression"),
        }
    }
}

impl From<ast::Assignment> for Expr<UnlinkedPass> {
    fn from(ast::Assignment { op: _, expr }: ast::Assignment) -> Self {
        expr.into()
    }
}

impl From<ast::Grouping> for Expr<UnlinkedPass> {
    fn from(
        ast::Grouping {
            open: _,
            expr,
            close: _,
        }: ast::Grouping,
    ) -> Self {
        expr.into()
    }
}

pub struct Binary<S: HirPass> {
    pub op: Operator,
    pub lhs: Expr<S>,
    pub rhs: Expr<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for Binary<S>
where
    V: Visitor<S>,
{
    type Output = Binary<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { op, lhs, rhs } = self;
        let lhs = lhs.walk(visitor);
        let rhs = rhs.walk(visitor);
        Ok(Binary {
            op,
            lhs: lhs?,
            rhs: rhs?,
        })
    }
}

impl From<ast::Binary> for Binary<UnlinkedPass> {
    fn from(
        ast::Binary {
            lhs,
            op,
            op_span: _,
            rhs,
        }: ast::Binary,
    ) -> Self {
        Self {
            op,
            lhs: lhs.into(),
            rhs: rhs.into(),
        }
    }
}

pub struct Unary<S: HirPass> {
    pub op: Operator,
    pub operand: Expr<S>,
    pub side: OpSide,
}

impl<V, S: HirPass> TreeWalk<V, S> for Unary<S>
where
    V: Visitor<S>,
{
    type Output = Unary<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { op, operand, side } = self;
        let operand = operand.walk(visitor);
        Ok(Unary {
            op,
            operand: operand?,
            side,
        })
    }
}

impl From<ast::Unary> for Unary<UnlinkedPass> {
    fn from(
        ast::Unary {
            op,
            op_span: _,
            operand,
            side,
        }: ast::Unary,
    ) -> Self {
        Self {
            op,
            operand: operand.into(),
            side,
        }
    }
}

#[derive(Walkable)]
pub struct Subscript<S: HirPass> {
    pub arr: Expr<S>,
    pub idx: Expr<S>,
}

impl From<ast::Subscript> for Subscript<UnlinkedPass> {
    fn from(
        ast::Subscript {
            arr,
            open: _,
            index,
            close: _,
        }: ast::Subscript,
    ) -> Self {
        Self {
            arr: arr.into(),
            idx: index.into(),
        }
    }
}

#[derive(Walkable)]
pub struct FnCall<S: HirPass> {
    pub fun: Expr<S>,
    pub args: Box<[Expr<S>]>,
}

impl From<ast::FnCall> for FnCall<UnlinkedPass> {
    fn from(
        ast::FnCall {
            fun,
            open: _,
            args,
            close: _,
        }: ast::FnCall,
    ) -> Self {
        Self {
            fun: fun.into(),
            args: convert_iter(args),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnlinkedLiteral(pub ast::Literal);

impl From<ast::Literal> for UnlinkedLiteral {
    fn from(value: ast::Literal) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum LinkedLiteral {
    #[default]
    None,
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Flpt(flpt),
    Char(char),
    Text(text),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Walkable)]
pub enum DirectiveKind {
    Inpt,
    Prnt,
}

impl From<ast::DirectiveKw> for DirectiveKind {
    fn from(kw: ast::DirectiveKw) -> Self {
        match kw {
            ast::DirectiveKw::Inpt => DirectiveKind::Inpt,
            ast::DirectiveKw::Prnt => DirectiveKind::Prnt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Walkable)]
pub struct Directive {
    pub span: RelSpan,
    pub kind: DirectiveKind,
}

impl From<ast::Directive> for Directive {
    fn from(ast::Directive { kw, kw_span }: ast::Directive) -> Self {
        Self {
            span: kw_span,
            kind: kw.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct UnlinkedDecl {
    pub kind: ItemKind,
    pub ident: RelSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedDecl(pub ItemId);

pub struct UnlinkedMod(pub ast::DeclMod);

impl From<ast::DeclMod> for UnlinkedMod {
    fn from(value: ast::DeclMod) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedMod(pub ModId);

pub struct UnlinkedVar(pub ast::Variable);

impl From<ast::Variable> for UnlinkedVar {
    fn from(value: ast::Variable) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedVar(pub VarId);

#[derive(Walkable)]
pub enum BlockExpr<S: HirPass> {
    If(Box<IfStmt<S>>),
    Loop(Box<LoopStmt<S>>),
    Cord(Box<CordStmt<S>>),
    Block(BlockStmt<S>),
}

impl From<ast::BlockExpr> for BlockExpr<UnlinkedPass> {
    fn from(value: ast::BlockExpr) -> Self {
        match value {
            ast::BlockExpr::If(inner) => Self::If(Box::new((*inner).into())),
            ast::BlockExpr::Loop(ast::LoopBlock {
                label,
                kw: _,
                kind,
                block,
            }) => {
                let label = label.map(Into::into);
                let body = block.into();
                match kind {
                    ast::LoopKind::Loop => Self::Loop(Box::new(LoopStmt { label, body })),
                    ast::LoopKind::Cord => Self::Cord(Box::new(CordStmt { label, body })),
                    _ => panic!("should only be loop or cord"),
                }
            }
            ast::BlockExpr::Block(inner) => Self::Block(inner.into()),
        }
    }
}

pub struct DeclPvt<S: HirPass> {
    pub id: S::ItemDcl,
    pub ty: Option<TypeExpr<S>>,
    pub init: Expr<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclPvt<S>
where
    V: Visitor<S>,
{
    type Output = DeclPvt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty, init } = self;
        let ty = ty.walk(visitor);
        let init = init.walk(visitor);
        // ID must be declared AFTER initializer and type
        let id = visitor.visit_item_dcl(id);
        Ok(DeclPvt {
            id: id?,
            ty: ty?,
            init: init?,
        })
    }
}

pub struct DeclLet<S: HirPass> {
    // TODO: complex bindings
    pub id: S::VarDcl,
    pub ty: Option<TypeExpr<S>>,
    pub init: Option<Expr<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DeclLet<S>
where
    V: Visitor<S>,
{
    type Output = DeclLet<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { id, ty, init } = self;
        let ty = ty.walk(visitor);
        let init = init.walk(visitor);
        // ID must be declared AFTER initializer and type
        let id = visitor.visit_var_dcl(id);
        Ok(DeclLet {
            id: id?,
            ty: ty?,
            init: init?,
        })
    }
}

#[derive(Walkable)]
pub struct DeclDfer<S: HirPass> {
    pub param: Option<BindingPattern<S>>,
    pub on_pop: BlockStmt<S>,
}

impl From<ast::DeclDfer> for DeclDfer<UnlinkedPass> {
    fn from(
        ast::DeclDfer {
            kw: _,
            param,
            on_pop,
        }: ast::DeclDfer,
    ) -> Self {
        Self {
            param: param.map(
                |ast::DferParam {
                     open: _,
                     param: ast::BindingPattern { ident },
                     close: _,
                 }| BindingPattern {
                    var: UnlinkedDecl {
                        kind: ItemKind::DferParam,
                        ident,
                    },
                },
            ),
            on_pop: on_pop.into(),
        }
    }
}

#[derive(Walkable)]
pub enum Stmt<S: HirPass> {
    Expr(ExprStmt<S>),
    If(IfStmt<S>),
    Pick(PickExpr<S>),
    Loop(LoopStmt<S>),
    Cord(CordStmt<S>),
    Rep(RepStmt<S>),
    For(ForStmt<S>),
    Give(GiveStmt<S>),
    Fail(FailStmt<S>),
    Done(DoneStmt<S>),
    Skip(SkipStmt<S>),
    Block(BlockStmt<S>),
}

impl From<ast::Stmt> for Stmt<UnlinkedPass> {
    fn from(value: ast::Stmt) -> Self {
        match value {
            ast::Stmt::Expr(inner) => Self::Expr(inner.into()),
            ast::Stmt::If(inner) => Self::If(inner.into()),
            ast::Stmt::Loop(ast::LoopBlock {
                label,
                kw: _,
                kind,
                block,
            }) => {
                let label = label.map(Into::into);
                let body = block.into();
                match kind {
                    ast::LoopKind::Loop => Self::Loop(LoopStmt { label, body }),
                    ast::LoopKind::Cord => Self::Cord(CordStmt { label, body }),
                    ast::LoopKind::Rep { pred } => Self::Rep(RepStmt {
                        label,
                        pred: pred.into(),
                        body,
                    }),
                    ast::LoopKind::For {
                        bind: ast::BindingPattern { ident },
                        in_kw: _,
                        iter,
                    } => Self::For(ForStmt {
                        label,
                        var: BindingPattern {
                            var: UnlinkedDecl {
                                kind: ItemKind::Var,
                                ident,
                            },
                        },
                        iter: iter.into(),
                        body,
                    }),
                }
            }
            ast::Stmt::Fail(inner) => Self::Fail(inner.into()),
            ast::Stmt::Give(inner) => Self::Give(inner.into()),
            ast::Stmt::Emit(_) => todo!("emit"),
            ast::Stmt::Done(inner) => Self::Done(inner.into()),
            ast::Stmt::Skip(inner) => Self::Skip(inner.into()),
            ast::Stmt::Block(inner) => Self::Block(inner.into()),
        }
    }
}

#[derive(Walkable)]
pub struct ExprStmt<S: HirPass>(pub Expr<S>);

impl From<ast::ExprStmt> for ExprStmt<UnlinkedPass> {
    fn from(ast::ExprStmt { expr, semi: _ }: ast::ExprStmt) -> Self {
        Self(expr.into())
    }
}

#[derive(Walkable)]
pub struct IfStmt<S: HirPass> {
    pub pred: Expr<S>,
    pub then: BlockStmt<S>,
    pub or: Option<Box<OrStmt<S>>>,
}

impl From<ast::IfStmt> for IfStmt<UnlinkedPass> {
    fn from(
        ast::IfStmt {
            kw: _,
            pred,
            then,
            or,
        }: ast::IfStmt,
    ) -> Self {
        Self {
            pred: pred.into(),
            then: then.into(),
            or: or.map(|x| Box::new((*x).into())),
        }
    }
}

#[derive(Walkable)]
pub enum OrStmt<S: HirPass> {
    More(IfStmt<S>),
    Last(BlockStmt<S>),
}

impl From<ast::OrBody> for OrStmt<UnlinkedPass> {
    fn from(value: ast::OrBody) -> Self {
        match value {
            ast::OrBody::More(inner) => Self::More(inner.into()),
            ast::OrBody::Last(inner) => Self::Last(inner.into()),
        }
    }
}

impl From<ast::OrStmt> for OrStmt<UnlinkedPass> {
    fn from(ast::OrStmt { kw: _, body }: ast::OrStmt) -> Self {
        body.into()
    }
}

#[derive(Walkable)]
pub struct PickExpr<S: HirPass> {
    pub expr: Expr<S>,
    pub arms: Box<[PickArm<S>]>,
}

impl From<ast::PickStmt> for PickExpr<UnlinkedPass> {
    fn from(
        ast::PickStmt {
            kw: _,
            expr,
            open: _,
            arms,
            close: _,
        }: ast::PickStmt,
    ) -> Self {
        Self {
            expr: expr.into(),
            arms: convert_iter(arms),
        }
    }
}

#[derive(Walkable)]
pub struct PickArm<S: HirPass> {
    pub pat: PickPattern,
    pub expr: Expr<S>,
}

impl From<ast::PickArm> for PickArm<UnlinkedPass> {
    fn from(
        ast::PickArm {
            pattern,
            arrow: _,
            then,
        }: ast::PickArm,
    ) -> Self {
        Self {
            pat: pattern.into(),
            expr: then.into(),
        }
    }
}

pub struct UnlinkedLabel(pub RelSpan);

impl From<ast::Label> for UnlinkedLabel {
    fn from(ast::Label { dot: _, name }: ast::Label) -> Self {
        Self(name)
    }
}

impl From<ast::ScopeLabel> for UnlinkedLabel {
    fn from(ast::ScopeLabel { label, colon: _ }: ast::ScopeLabel) -> Self {
        label.into()
    }
}

pub struct LinkedLabel(pub LabelId);

pub struct LoopStmt<S: HirPass> {
    pub label: Option<S::LabelDcl>,
    pub body: BlockStmt<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for LoopStmt<S>
where
    V: Visitor<S>,
{
    type Output = LoopStmt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, body } = self;
        // HACK: inlined from BlockStmt so that `label` can be within the scope of the block
        let (label, body) = {
            let BlockStmt { decls } = body;
            visitor.push_scope();
            let label = label.map(|x| visitor.visit_label_dcl(x)).transpose();
            let body = decls.walk(visitor).map(|decls| BlockStmt { decls });
            visitor.pop_scope();
            (label, body)
        };
        Ok(LoopStmt {
            label: label?,
            body: body?,
        })
    }
}

pub struct CordStmt<S: HirPass> {
    pub label: Option<S::LabelDcl>,
    pub body: BlockStmt<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for CordStmt<S>
where
    V: Visitor<S>,
{
    type Output = CordStmt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, body } = self;
        // HACK: inlined from BlockStmt so that `label` can be within the scope of the block
        let (label, body) = {
            let BlockStmt { decls } = body;
            visitor.push_scope();
            let label = label.map(|x| visitor.visit_label_dcl(x)).transpose();
            let body = decls.walk(visitor).map(|decls| BlockStmt { decls });
            visitor.pop_scope();
            (label, body)
        };
        Ok(CordStmt {
            label: label?,
            body: body?,
        })
    }
}

pub struct RepStmt<S: HirPass> {
    pub label: Option<S::LabelDcl>,
    pub pred: Expr<S>,
    pub body: BlockStmt<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for RepStmt<S>
where
    V: Visitor<S>,
{
    type Output = RepStmt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, pred, body } = self;
        let label = label.map(|x| visitor.visit_label_dcl(x)).transpose();
        let pred = pred.walk(visitor);
        let body = body.walk(visitor);
        Ok(RepStmt {
            label: label?,
            pred: pred?,
            body: body?,
        })
    }
}

pub struct ForStmt<S: HirPass> {
    pub label: Option<S::LabelDcl>,
    pub var: BindingPattern<S>,
    pub iter: Expr<S>,
    pub body: BlockStmt<S>,
}

impl<V, S: HirPass> TreeWalk<V, S> for ForStmt<S>
where
    V: Visitor<S>,
{
    type Output = ForStmt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self {
            label,
            var,
            iter,
            body,
        } = self;
        let iter = iter.walk(visitor);
        // HACK: inlined from BlockStmt so that `label` can be within the scope of the block
        let (label, var, body) = {
            let BlockStmt { decls } = body;
            visitor.push_scope();
            let label = label.map(|x| visitor.visit_label_dcl(x)).transpose();
            let var = var.walk(visitor);
            let body = decls.walk(visitor).map(|decls| BlockStmt { decls });
            visitor.pop_scope();
            (label, var, body)
        };
        Ok(ForStmt {
            label: label?,
            var: var?,
            iter: iter?,
            body: body?,
        })
    }
}

#[derive(Walkable)]
pub struct GiveStmt<S: HirPass> {
    pub expr: Option<Expr<S>>,
}

impl From<ast::GiveStmt> for GiveStmt<UnlinkedPass> {
    fn from(
        ast::GiveStmt {
            kw: _,
            value,
            semi: _,
        }: ast::GiveStmt,
    ) -> Self {
        Self {
            expr: value.map(Into::into),
        }
    }
}

#[derive(Walkable)]
pub struct FailStmt<S: HirPass> {
    pub expr: Option<Expr<S>>,
}

impl From<ast::FailStmt> for FailStmt<UnlinkedPass> {
    fn from(
        ast::FailStmt {
            kw: _,
            value,
            semi: _,
        }: ast::FailStmt,
    ) -> Self {
        Self {
            expr: value.map(Into::into),
        }
    }
}

pub struct DoneStmt<S: HirPass> {
    pub label: Option<S::LabelRef>,
    pub expr: Option<Expr<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for DoneStmt<S>
where
    V: Visitor<S>,
{
    type Output = DoneStmt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label, expr } = self;
        let label = label.map(|x| visitor.visit_label_ref(x)).transpose();
        let expr = expr.walk(visitor);
        Ok(DoneStmt {
            label: label?,
            expr: expr?,
        })
    }
}

impl From<ast::DoneStmt> for DoneStmt<UnlinkedPass> {
    fn from(
        ast::DoneStmt {
            kw: _,
            label,
            value,
            semi: _,
        }: ast::DoneStmt,
    ) -> Self {
        Self {
            label: label.map(Into::into),
            expr: value.map(Into::into),
        }
    }
}

pub struct SkipStmt<S: HirPass> {
    pub label: Option<S::LabelRef>,
}

impl<V, S: HirPass> TreeWalk<V, S> for SkipStmt<S>
where
    V: Visitor<S>,
{
    type Output = SkipStmt<V::Output>;

    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { label } = self;
        let label = label.map(|x| visitor.visit_label_ref(x)).transpose();
        Ok(SkipStmt { label: label? })
    }
}

impl From<ast::SkipStmt> for SkipStmt<UnlinkedPass> {
    fn from(
        ast::SkipStmt {
            kw: _,
            label,
            semi: _,
        }: ast::SkipStmt,
    ) -> Self {
        Self {
            label: label.map(Into::into),
        }
    }
}

pub struct BlockStmt<S: HirPass> {
    pub decls: VecDeque<Decl<S>>,
}

impl<V, S: HirPass> TreeWalk<V, S> for BlockStmt<S>
where
    V: Visitor<S>,
{
    type Output = BlockStmt<V::Output>;
    fn walk(self, visitor: &mut V) -> Result<Self::Output, CheckDiagnostics> {
        let Self { decls } = self;
        visitor.push_scope();
        let res = decls.walk(visitor).map(|decls| BlockStmt { decls });
        visitor.pop_scope();
        res
    }
}

impl From<ast::BlockStmt> for BlockStmt<UnlinkedPass> {
    fn from(
        ast::BlockStmt {
            open: _,
            body,
            close: _,
        }: ast::BlockStmt,
    ) -> Self {
        Self {
            decls: convert_iter(body),
        }
    }
}
