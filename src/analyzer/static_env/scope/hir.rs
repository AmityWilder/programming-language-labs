//! Type Intermediate Representation

#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    reason = "need to be aware of when the structure changes"
)]

use crate::{
    analyzer::{
        static_env::{ItemId, ModId, TypeId, VarId},
        types::bounds::Combinator,
    },
    lex::token::{Operator, punc::OpSide},
    parse::ast,
    src_code::span::{FileId, RelSpan},
    todo_types::Todo,
    util::{DropAll, OrAllErrsExt},
};

#[inline]
pub const fn no_change<T>(x: T) -> Option<T> {
    Some(x)
}

pub trait TreeMap<Mod1, Dcl1, Var1, Tyn1, Lbl1, Lit1> {
    type Output<Mod2, Dcl2, Var2, Tyn2, Lbl2, Lit2>;

    fn tree_map<Ctx, Mod2, Dcl2, Var2, Tyn2, Lbl2, Lit2>(
        self,
        ctx: &mut Ctx,
        fmod: &mut impl FnMut(Mod1, &mut Ctx) -> Option<Mod2>,
        fdcl: &mut impl FnMut(Dcl1, &mut Ctx) -> Option<Dcl2>,
        fvar: &mut impl FnMut(Var1, &mut Ctx) -> Option<Var2>,
        ftyn: &mut impl FnMut(Tyn1, &mut Ctx) -> Option<Tyn2>,
        flbl: &mut impl FnMut(Lbl1, &mut Ctx) -> Option<Lbl2>,
        flit: &mut impl FnMut(Lit1, &mut Ctx) -> Option<Lit2>,
    ) -> Option<Self::Output<Mod2, Dcl2, Var2, Tyn2, Lbl2, Lit2>>;
}

macro_rules! impl_tree_map {
    (for $Type:ident fn($self:ident, $ctx:ident, $fm:ident, $fd:ident, $fv:ident, $ft:ident, $fl:ident) $def:block) => {
        impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for $Type<M1, D1, V1, T1, L1> {
            type Output<M2, D2, V2, T2, L2> = $Type<M2, D2, V2, T2, L2>;

            fn tree_map<Ctx, M2, D2, V2, T2, L2>(
                $self,
                $ctx: &mut Ctx,
                $fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
                $fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
                $fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
                $ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
                $fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
            ) -> Option<Self::Output<M2, D2, V2, T2, L2>>
                $def
        }
    };

    (for $Type:ident) => {
        impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for $Type {
            type Output<M2, D2, V2, T2, L2> = Self;

            fn tree_map<Ctx, M2, D2, V2, T2, L2>(
                self,
                _: &mut Ctx,
                _: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
                _: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
                _: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
                _: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
                _: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
            ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
                Some(self)
            }
        }
    };
}

/// Passthrough
macro_rules! TreeMapPass {
    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident<$M:ident, $D:ident, $V:ident, $T:ident, $L:ident, $Lit:ident>(
            $(#[$fm:meta])*
            $fvis:vis $Field:ty $(,)?
        );
    } => {
        impl_tree_map! {
            for $Type fn(self, ctx, fm, fd, fv, ft, fl) {
                self.0.tree_map(ctx, fm, fd, fv, ft, fl).map($Type)
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident<$M:ident, $D:ident, $V:ident, $T:ident, $L:ident, $Lit:ident> {$(
            $(#[$fm:meta])*
            $fvis:vis $field:ident: $Field:ty
        ),* $(,)?}
    } => {
        impl_tree_map! {
            for $Type fn(self, ctx, fm, fd, fv, ft, fl) {
                let Self { $($field),* } = self;
                $(let $field = $field.tree_map(ctx, fm, fd, fv, ft, fl);)*
                Some($Type { $($field: $field?),* })
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis enum $Type:ident<$M:ident, $D:ident, $V:ident, $T:ident, $L:ident, $Lit:ident> {$(
            $(#[$vm:meta])*
            $Variant:ident(
                $(#[$fm:meta])*
                $Field:ty $(,)?
            )
        ),* $(,)?}
    } => {
        impl_tree_map! {
            for $Type fn(self, ctx, fm, fd, fv, ft, fl) {
                match self {
                    $(Self::$Variant(x) => x.tree_map(ctx, fm, fd, fv, ft, fl).map($Type::$Variant),)*
                }
            }
        }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident($($body:tt)*);
    } => {
        impl_tree_map! { for $Type }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis struct $Type:ident { $($body:tt)* }
    } => {
        impl_tree_map! { for $Type }
    };

    derive() {
        $(#[$m:meta])*
        $vis:vis enum $Type:ident { $($body:tt)* }
    } => {
        impl_tree_map! { for $Type }
    };
}

impl<M1, D1, V1, T1, L1, T: TreeMap<M1, D1, V1, T1, L1>> TreeMap<M1, D1, V1, T1, L1> for Option<T> {
    type Output<M2, D2, V2, T2, L2> = Option<T::Output<M2, D2, V2, T2, L2>>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        self.map(|x| x.tree_map(ctx, fm, fd, fv, ft, fl))
    }
}

impl<M1, D1, V1, T1, L1, T: TreeMap<M1, D1, V1, T1, L1>> TreeMap<M1, D1, V1, T1, L1> for Box<[T]> {
    type Output<M2, D2, V2, T2, L2> = Box<[T::Output<M2, D2, V2, T2, L2>]>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        self.into_iter()
            .map(|x| x.tree_map(ctx, fm, fd, fv, ft, fl).ok_or(()))
            .collect_or_acc_errs::<_, DropAll>()
            .map(Vec::into_boxed_slice)
            .ok()
    }
}

impl<M1, D1, V1, T1, L1, T: TreeMap<M1, D1, V1, T1, L1>> TreeMap<M1, D1, V1, T1, L1> for Box<T> {
    type Output<M2, D2, V2, T2, L2> = Box<T::Output<M2, D2, V2, T2, L2>>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        (*self).tree_map(ctx, fm, fd, fv, ft, fl).map(Box::new)
    }
}

fn convert_iter<U, A: IntoIterator<Item: Into<U>>, B: FromIterator<U>>(vec: A) -> B {
    vec.into_iter().map(Into::into).collect::<B>()
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirUnit<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub file: FileId,
    pub decls: Box<[Decl<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::ProgramUnit>
    for HirUnit<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::ProgramUnit { file, decls }: ast::ProgramUnit) -> Self {
        Self {
            file,
            decls: convert_iter(decls),
        }
    }
}

impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for HirUnit<M1, D1, V1, T1, L1> {
    type Output<M2, D2, V2, T2, L2> = HirUnit<M2, D2, V2, T2, L2>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        let Self { file, decls } = self;
        let decls = decls.tree_map(ctx, fm, fd, fv, ft, fl);
        Some(HirUnit {
            file,
            decls: decls?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, TreeMapPass)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct NamespacePathPart {
    /// Identifier
    pub name: RelSpan, // TODO
}

impl From<ast::NamespacePathPart> for NamespacePathPart {
    fn from(ast::NamespacePathPart { name, sep: _ }: ast::NamespacePathPart) -> Self {
        Self { name }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkedTypename(pub ast::Typename);

impl From<ast::Typename> for UnlinkedTypename {
    fn from(value: ast::Typename) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedTypename(pub TypeId);

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct GenericGroup<Mod, Decl, Var, Ty, Lbl, Lit>(
    pub Box<[TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>]>,
);

impl From<ast::GenericGroup>
    for GenericGroup<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(value: ast::GenericGroup) -> Self {
        Self(convert_iter(value.list))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInstance<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub name: T,
    pub args: Option<GenericGroup<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::TypeInstance>
    for TypeInstance<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::TypeInstance { name, args }: ast::TypeInstance) -> Self {
        Self {
            name: name.into(),
            args: args.map(Into::into),
        }
    }
}

impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for TypeInstance<M1, D1, V1, T1, L1> {
    type Output<M2, D2, V2, T2, L2> = TypeInstance<M2, D2, V2, T2, L2>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        let Self { name, args } = self;
        let name = ft(name, ctx);
        let args = args.tree_map(ctx, fm, fd, fv, ft, fl);
        Some(TypeInstance {
            name: name?,
            args: args?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct FallType<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub then_ty: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub ty: OrType,
}

impl From<ast::FallType>
    for FallType<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for FallType<M1, D1, V1, T1, L1> {
    type Output<M2, D2, V2, T2, L2> = FallType<M2, D2, V2, T2, L2>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        let Self { then_ty, ty } = self;
        let then_ty = then_ty.tree_map(ctx, fm, fd, fv, ft, fl);
        Some(FallType {
            then_ty: then_ty?,
            ty,
        })
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct RefType<Mod, Decl, Var, Ty, Lbl, Lit> {
    // TBD: reference mutability?
    pub ty: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::RefType>
    for RefType<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::RefType { sym: _, ty }: ast::RefType) -> Self {
        Self { ty: (*ty).into() }
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct ViewType<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub item: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub count: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::ViewType>
    for ViewType<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub enum AssocTypeSource<Mod, Decl, Var, Ty, Lbl, Lit> {
    Simple(T),
    Grouped(TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>),
}

impl From<ast::AssocTypeSource>
    for AssocTypeSource<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(value: ast::AssocTypeSource) -> Self {
        match value {
            ast::AssocTypeSource::Simple(inner) => Self::Simple(inner.into()),
            ast::AssocTypeSource::Grouped(inner) => Self::Grouped(inner.inner.into()),
        }
    }
}

impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for AssocTypeSource<M1, D1, V1, T1, L1> {
    type Output<M2, D2, V2, T2, L2> = AssocTypeSource<M2, D2, V2, T2, L2>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        match self {
            Self::Simple(x) => ft(x, ctx).map(AssocTypeSource::Simple),
            Self::Grouped(x) => x
                .tree_map(ctx, fm, fd, fv, ft, fl)
                .map(AssocTypeSource::Grouped),
        }
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct AssocType<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub base: AssocTypeSource<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub assoc: TypeInstance<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::AssocType>
    for AssocType<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct FunType<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub params: Box<[TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>]>,
    pub ret: Option<Box<TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>>>,
}

impl From<ast::FunType>
    for FunType<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct OpaqueType<Mod, Decl, Var, Ty, Lbl, Lit>(pub TypeBound<Mod, Decl, Var, Ty, Lbl, Lit>);

impl From<ast::OpaqueType>
    for OpaqueType<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::OpaqueType { colon: _, bound }: ast::OpaqueType) -> Self {
        Self(bound.into())
    }
}

/// All/Any/One
#[derive(Debug, Clone, PartialEq)]
pub struct ComboBound<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub cmb: Combinator,
    pub bounds: Box<[TypeBound<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::ComboBound>
    for ComboBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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
#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct NegativeBound<Mod, Decl, Var, Ty, Lbl, Lit>(pub TypeBound<Mod, Decl, Var, Ty, Lbl, Lit>);

impl From<ast::NegativeBound>
    for NegativeBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::NegativeBound { not: _, rhs }: ast::NegativeBound) -> Self {
        Self(rhs.into())
    }
}

/// Only
#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct ExactTypeBound<Mod, Decl, Var, Ty, Lbl, Lit>(pub TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>);

impl From<ast::ExactTypeBound>
    for ExactTypeBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::ExactTypeBound { kw: _, ty }: ast::ExactTypeBound) -> Self {
        Self(ty.into())
    }
}

/// Has
#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct MemberBound<Mod, Decl, Var, Ty, Lbl, Lit> {
    /// Identifier
    // TBD: should this be an ID at any point?
    pub member: RelSpan,
    pub req: Option<TypeBound<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::MemberBound>
    for MemberBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::MemberBound { kw: _, member, req }: ast::MemberBound) -> Self {
        Self {
            member,
            req: req.map(|ast::ColonBound { colon: _, bound }| bound.into()),
        }
    }
}

/// Shape
#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct ShapeBound<Mod, Decl, Var, Ty, Lbl, Lit>(pub FunType<Mod, Decl, Var, Ty, Lbl, Lit>);

impl From<ast::FunType>
    for ShapeBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(value: ast::FunType) -> Self {
        Self(value.into())
    }
}

/// Supports
#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct OverloadBound {
    // TODO
}

impl From<ast::OverloadBound> for OverloadBound {
    fn from(ast::OverloadBound {}: ast::OverloadBound) -> Self {
        Self {}
    }
}

/// Satisfies
#[derive(Debug, Clone, PartialEq)]
pub struct ExprBound<Mod, Decl, Var, Ty, Lbl, Lit> {
    // TBD: should this be a special ID since it never actually appears in the program?
    pub param: D,
    pub expr: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::ExprBound>
    for ExprBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(
        ast::ExprBound {
            kw: _,
            name,
            colon: _,
            expr,
        }: ast::ExprBound,
    ) -> Self {
        Self {
            param: name.into(),
            expr: expr.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum TypeBound<Mod, Decl, Var, Ty, Lbl, Lit> {
    Combo(ComboBound<Mod, Decl, Var, Ty, Lbl, Lit>),
    Not(Box<NegativeBound<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Exact(ExactTypeBound<Mod, Decl, Var, Ty, Lbl, Lit>),
    Has(Box<MemberBound<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Shape(ShapeBound<Mod, Decl, Var, Ty, Lbl, Lit>),
    Supports(Box<OverloadBound>),
    Satisfies(Box<ExprBound<Mod, Decl, Var, Ty, Lbl, Lit>>),
}

impl From<ast::TypeBound>
    for TypeBound<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit> {
    Primitive(PrimitiveType),
    Instance(TypeInstance<Mod, Decl, Var, Ty, Lbl, Lit>),
    Fallible(Box<FallType<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Ref(Box<RefType<Mod, Decl, Var, Ty, Lbl, Lit>>),
    View(Box<ViewType<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Assoc(Box<AssocType<Mod, Decl, Var, Ty, Lbl, Lit>>),
    FunPtr(FunType<Mod, Decl, Var, Ty, Lbl, Lit>),
    Opaque(Box<OpaqueType<Mod, Decl, Var, Ty, Lbl, Lit>>),
}

impl From<ast::TypeExpr>
    for TypeExpr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

impl From<ast::TypeAnnotation>
    for TypeExpr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::TypeAnnotation { colon: _, ty }: ast::TypeAnnotation) -> Self {
        ty.into()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingPattern<D> {
    pub var: D,
}

impl From<ast::BindingPattern> for BindingPattern<UnlinkedDecl> {
    fn from(ast::BindingPattern { ident }: ast::BindingPattern) -> Self {
        Self { var: ident.into() }
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct PickPattern {
    // TODO
}

impl From<ast::PickPattern> for PickPattern {
    fn from(ast::PickPattern {}: ast::PickPattern) -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum Decl<Mod, Decl, Var, Ty, Lbl, Lit> {
    Mod(DeclMod<M>),
    On(DeclOn<Mod, Decl, Var, Ty, Lbl, Lit>),
    Fun(DeclFun<Mod, Decl, Var, Ty, Lbl, Lit>),
    Rec(DeclRec<Mod, Decl, Var, Ty, Lbl, Lit>),
    Cat(DeclCat<Mod, Decl, Var, Ty, Lbl, Lit>),
    Uni(DeclUni<Mod, Decl, Var, Ty, Lbl, Lit>),
    Let(DeclLet<Mod, Decl, Var, Ty, Lbl, Lit>),
    Pvt(DeclPvt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Dfer(DeclDfer<Mod, Decl, Var, Ty, Lbl, Lit>),
    Stmt(Stmt<Mod, Decl, Var, Ty, Lbl, Lit>),
}

impl From<ast::Decl>
    for Decl<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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
                let id = ident.into();
                let ty = ty.map(Into::into);
                let init = assignment.map(Into::into);
                match kw {
                    ast::VarKeyword::Let => Self::Let(DeclLet { id, ty, init }),
                    ast::VarKeyword::Uni => Self::Uni(DeclUni { id, ty, init }),
                    ast::VarKeyword::Pvt => Self::Pvt(DeclPvt {
                        id,
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

#[derive(Debug, Clone, PartialEq)]
pub struct DeclMod<M> {
    pub id: M,
}

impl From<ast::DeclMod> for DeclMod<UnlinkedMod> {
    fn from(
        ast::DeclMod {
            kw: _,
            name,
            semi: _,
        }: ast::DeclMod,
    ) -> Self {
        Self { id: name.into() }
    }
}

impl<M1, D1, V1, T1, L1> TreeMap<M1, D1, V1, T1, L1> for DeclMod<M1> {
    type Output<M2, D2, V2, T2, L2> = DeclMod<M2>;

    fn tree_map<Ctx, M2, D2, V2, T2, L2>(
        self,
        ctx: &mut Ctx,
        fm: &mut impl FnMut(M1, &mut Ctx) -> Option<M2>,
        _fd: &mut impl FnMut(D1, &mut Ctx) -> Option<D2>,
        _fv: &mut impl FnMut(V1, &mut Ctx) -> Option<V2>,
        _ft: &mut impl FnMut(T1, &mut Ctx) -> Option<T2>,
        _fl: &mut impl FnMut(L1, &mut Ctx) -> Option<L2>,
    ) -> Option<Self::Output<M2, D2, V2, T2, L2>> {
        let Self { id } = self;
        let id = fm(id, ctx);
        Some(DeclMod { id: id? })
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct TypeBoundClause<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub target: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub bound: TypeBound<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::TypeBoundClause>
    for TypeBoundClause<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct IffClause<Mod, Decl, Var, Ty, Lbl, Lit>(
    pub Box<[TypeBoundClause<Mod, Decl, Var, Ty, Lbl, Lit>]>,
);

impl From<ast::IffClause>
    for IffClause<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::IffClause { iff: _, bounds }: ast::IffClause) -> Self {
        Self(convert_iter(bounds))
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct DeclOn<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub generics: Option<GenericGroup<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub target: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub iff: Option<IffClause<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub members: Box<[Decl<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::DeclOn>
    for DeclOn<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct SelfParam<D> {
    pub id: D,
}

impl From<ast::SelfParam> for SelfParam<UnlinkedDecl> {
    fn from(ast::SelfParam { span }: ast::SelfParam) -> Self {
        Self { id: span.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclFun<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D,
    pub generics: Option<GenericGroup<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub self_param: Option<SelfParam<D>>,
    pub params: Box<[FunParam<Mod, Decl, Var, Ty, Lbl, Lit>]>,
    pub ret: Option<TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub iff: Option<IffClause<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub body: Option<BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::DeclFun>
    for DeclFun<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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
            id: name.into(),
            generics: generics.map(Into::into),
            self_param: self_param.map(Into::into),
            params: convert_iter(params),
            ret: ret.map(|ast::FunRet { arrow: _, ty }| ty.into()),
            iff: iff.map(Into::into),
            body: body.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunParam<Mod, Decl, Var, Ty, Lbl, Lit> {
    /// Treated as a member of the function
    pub id: D,
    pub ty: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::FnParam>
    for FunParam<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::FnParam { name, ty }: ast::FnParam) -> Self {
        Self {
            id: name.into(),
            ty: ty.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclRec<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D,
    pub generics: Option<GenericGroup<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub fields: Box<[Field<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::DeclRec>
    for DeclRec<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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
            id: name.into(),
            generics: generics.map(Into::into),
            fields: convert_iter(fields),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D, // TODO: these should be treated like unique items
    pub ty: TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::Field>
    for Field<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::Field { name, ty }: ast::Field) -> Self {
        Self {
            id: name.into(),
            ty: ty.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclCat<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D, // TODO: these should be treated like unique items
    pub discriminants: Box<[Discriminant<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::DeclCat>
    for DeclCat<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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
            id: name.into(),
            discriminants: convert_iter(items),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Discriminant<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D,
    pub val: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::CatDisc>
    for Discriminant<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::CatDisc { name, val }: ast::CatDisc) -> Self {
        Self {
            id: name.into(),
            val: val.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclUni<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D,
    pub ty: Option<TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub init: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<Mod, Decl, Var, Ty, Lbl, Lit> {
    Block(BlockExpr<Mod, Decl, Var, Ty, Lbl, Lit>),
    Binary(Box<Binary<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Unary(Box<Unary<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Index(Box<Subscript<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Call(Box<FnCall<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Literal(Lit),
    Directive(Directive),
    Variable(V),
}

impl From<ast::Expr>
    for Expr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

impl From<ast::Assignment>
    for Expr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::Assignment { op: _, expr }: ast::Assignment) -> Self {
        expr.into()
    }
}

impl From<ast::Grouping>
    for Expr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct Binary<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub op: Operator,
    pub lhs: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub rhs: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::Binary>
    for Binary<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct Unary<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub op: Operator,
    pub operand: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub side: OpSide,
}

impl From<ast::Unary>
    for Unary<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct Subscript<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub arr: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub idx: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::Subscript>
    for Subscript<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct FnCall<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub fun: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub args: Box<[Expr<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::FnCall>
    for FnCall<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkedLiteral(pub ast::Literal);

impl From<ast::Literal> for UnlinkedLiteral {
    fn from(value: ast::Literal) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkedLiteral(pub Todo);

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum Directive {
    Inpt,
}

impl From<ast::Directive> for Directive {
    fn from(ast::Directive { kw, kw_span: _ }: ast::Directive) -> Self {
        match kw {
            ast::DirectiveKw::Inpt => Directive::Inpt,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkedDecl(pub RelSpan);

impl From<RelSpan> for UnlinkedDecl {
    fn from(value: RelSpan) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedDecl(pub ItemId);

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkedMod(pub RelSpan);

impl From<RelSpan> for UnlinkedMod {
    fn from(value: RelSpan) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedMod(pub ModId);

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkedVar(pub ast::Variable);

impl From<ast::Variable> for UnlinkedVar {
    fn from(value: ast::Variable) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedVar(pub VarId);

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum BlockExpr<Mod, Decl, Var, Ty, Lbl, Lit> {
    If(Box<IfStmt<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Loop(Box<LoopStmt<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Cord(Box<CordStmt<Mod, Decl, Var, Ty, Lbl, Lit>>),
    Block(BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
}

impl From<ast::BlockExpr>
    for BlockExpr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct DeclPvt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub id: D,
    pub ty: Option<TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub init: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclLet<Mod, Decl, Var, Ty, Lbl, Lit> {
    // TODO: complex bindings
    pub id: D,
    pub ty: Option<TypeExpr<Mod, Decl, Var, Ty, Lbl, Lit>>,
    pub init: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct DeclDfer<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub param: Option<BindingPattern<D>>,
    pub on_pop: BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::DeclDfer>
    for DeclDfer<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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
                     param,
                     close: _,
                 }| param.into(),
            ),
            on_pop: on_pop.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum Stmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    Expr(ExprStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Print(PrintStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    If(IfStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Pick(PickExpr<Mod, Decl, Var, Ty, Lbl, Lit>),
    Loop(LoopStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Cord(CordStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Rep(RepStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    For(ForStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Give(GiveStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Fail(FailStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Done(DoneStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Skip(SkipStmt<L>),
    Block(BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
}

impl From<ast::Stmt>
    for Stmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(value: ast::Stmt) -> Self {
        match value {
            ast::Stmt::Expr(inner) => Self::Expr(inner.into()),
            ast::Stmt::Print(inner) => Self::Print(inner.into()),
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
                        bind,
                        in_kw: _,
                        iter,
                    } => Self::For(ForStmt {
                        label,
                        var: bind.into(),
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct ExprStmt<Mod, Decl, Var, Ty, Lbl, Lit>(pub Expr<Mod, Decl, Var, Ty, Lbl, Lit>);

impl From<ast::ExprStmt>
    for ExprStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::ExprStmt { expr, semi: _ }: ast::ExprStmt) -> Self {
        Self(expr.into())
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct PrintStmt<Mod, Decl, Var, Ty, Lbl, Lit>(pub Expr<Mod, Decl, Var, Ty, Lbl, Lit>);

impl From<ast::PrintStmt>
    for PrintStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(
        ast::PrintStmt {
            kw: _,
            group,
            semi: _,
        }: ast::PrintStmt,
    ) -> Self {
        Self(group.into())
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct IfStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub pred: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub then: BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub or: Option<Box<OrStmt<Mod, Decl, Var, Ty, Lbl, Lit>>>,
}

impl From<ast::IfStmt>
    for IfStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub enum OrStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    More(IfStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
    Last(BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>),
}

impl From<ast::OrBody>
    for OrStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(value: ast::OrBody) -> Self {
        match value {
            ast::OrBody::More(inner) => Self::More(inner.into()),
            ast::OrBody::Last(inner) => Self::Last(inner.into()),
        }
    }
}

impl From<ast::OrStmt>
    for OrStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(ast::OrStmt { kw: _, body }: ast::OrStmt) -> Self {
        body.into()
    }
}

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct PickExpr<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub expr: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub arms: Box<[PickArm<Mod, Decl, Var, Ty, Lbl, Lit>]>,
}

impl From<ast::PickStmt>
    for PickExpr<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct PickArm<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub pat: PickPattern,
    pub expr: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
}

impl From<ast::PickArm>
    for PickArm<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct UnlinkedLabel(pub RelSpan);

#[derive(Debug, Clone, PartialEq)]
pub struct LinkedLabel(pub Todo);

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

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub label: Option<L>,
    pub body: BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CordStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub label: Option<L>,
    pub body: BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub label: Option<L>,
    pub pred: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub body: BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub label: Option<L>,
    pub var: BindingPattern<D>,
    pub iter: Expr<Mod, Decl, Var, Ty, Lbl, Lit>,
    pub body: BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GiveStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub expr: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::GiveStmt>
    for GiveStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct FailStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub expr: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::FailStmt>
    for FailStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct DoneStmt<Mod, Decl, Var, Ty, Lbl, Lit> {
    pub label: Option<L>,
    pub expr: Option<Expr<Mod, Decl, Var, Ty, Lbl, Lit>>,
}

impl From<ast::DoneStmt>
    for DoneStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
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

#[derive(Debug, Clone, PartialEq)]
pub struct SkipStmt<L> {
    pub label: Option<L>,
}

impl From<ast::SkipStmt> for SkipStmt<UnlinkedLabel> {
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

#[derive(Debug, Clone, PartialEq, TreeMapPass)]
pub struct BlockStmt<Mod, Decl, Var, Ty, Lbl, Lit>(pub Box<[Decl<Mod, Decl, Var, Ty, Lbl, Lit>]>);

impl From<ast::BlockStmt>
    for BlockStmt<UnlinkedMod, UnlinkedDecl, UnlinkedVar, UnlinkedTypename, UnlinkedLabel>
{
    fn from(
        ast::BlockStmt {
            open: _,
            body,
            close: _,
        }: ast::BlockStmt,
    ) -> Self {
        Self(convert_iter(body))
    }
}
