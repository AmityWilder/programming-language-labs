#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    reason = "need to be aware of when the structure changes so its formatting can update"
)]

use crate::{
    analyzer::static_env::{
        ItemId, ModId,
        scope::{
            env::ScopeEnvironment,
            hir::{self, HirUnit},
        },
    },
    lex::token::{
        keyword::Keyword,
        punc::{OpSide, Punctuation},
    },
    src_code::{map::SourceMap, span::FileId},
};
use std::fmt;

mod helpers {
    use super::HirDisplay;
    use crate::{
        analyzer::static_env::scope::env::ScopeEnvironment,
        src_code::{map::SourceMap, span::FileId},
    };
    use std::fmt::{self, Write};

    /// [`std::fmt::PadAdapter`] is private :c
    pub struct Indented<'buf> {
        buf: &'buf mut (dyn 'buf + fmt::Write),
        on_newline: bool,
    }

    impl<'buf> Indented<'buf> {
        const INDENT: &str = "  ";

        fn wrap<'slot, 'fmt>(
            f: &'fmt mut fmt::Formatter<'_>,
            slot: &'slot mut Option<Self>,
        ) -> fmt::Formatter<'slot>
        where
            'fmt: 'buf + 'slot,
        {
            fmt::FormattingOptions::new()
                .alternate(f.alternate()) // <- the only option that's important in this situation
                .create_formatter(slot.insert(Self {
                    buf: f,
                    on_newline: true,
                }))
        }
    }

    impl Write for Indented<'_> {
        // while this function looks exactly identical to the one in the standard library,
        // I actually wrote it BEFORE checking the implementation and I just happened to "convergent evolution" the same definition.
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for line in s.split_inclusive('\n') {
                if self.on_newline {
                    self.buf.write_str(Self::INDENT)?;
                }
                self.on_newline = line.ends_with('\n');
                self.buf.write_str(line)?;
            }
            Ok(())
        }

        // this one I wrote after finding the implementation, and it's basically copy-pasted
        fn write_char(&mut self, c: char) -> fmt::Result {
            if self.on_newline {
                self.buf.write_str(Self::INDENT)?;
            }
            self.on_newline = c == '\n';
            self.buf.write_char(c)
        }
    }

    pub struct HirBlock<'fmt, 'buf> {
        f: &'fmt mut fmt::Formatter<'buf>,
        res: fmt::Result,
        has_prev: bool,
    }

    impl<'fmt, 'buf> HirBlock<'fmt, 'buf> {
        pub fn new(f: &'fmt mut fmt::Formatter<'buf>) -> Self {
            Self {
                res: f.write_str("{"),
                f,
                has_prev: false,
            }
        }

        pub fn finish(&mut self) -> fmt::Result {
            self.res?;
            self.f.write_str("}")
        }

        pub fn entry_with<F>(
            &mut self,
            fun: F,
            source: &SourceMap,
            file: FileId,
            env: &ScopeEnvironment,
        ) -> &mut Self
        where
            F: FnOnce(
                &SourceMap,
                FileId,
                &ScopeEnvironment,
                &mut fmt::Formatter<'_>,
            ) -> fmt::Result,
        {
            self.res = self.res.and_then(|()| {
                if self.f.alternate() {
                    if !self.has_prev {
                        self.f.write_str("\n")?;
                    }
                    let mut slot = None;
                    let mut writer = Indented::wrap(self.f, &mut slot);
                    fun(source, file, env, &mut writer)?;
                    self.f.write_str("\n")
                } else {
                    if self.has_prev {
                        self.f.write_str(" ")?;
                    }
                    fun(source, file, env, &mut *self.f)
                }
            });
            self.has_prev = true;
            self
        }

        pub fn entry<D>(
            &mut self,
            item: &D,
            source: &SourceMap,
            file: FileId,
            env: &ScopeEnvironment,
        ) -> &mut Self
        where
            D: ?Sized + HirDisplay,
        {
            self.entry_with(
                move |source, file, env, f| item.fmt(source, file, env, f),
                source,
                file,
                env,
            )
        }

        pub fn entries<D, I>(
            &mut self,
            items: I,
            source: &SourceMap,
            file: FileId,
            env: &ScopeEnvironment,
        ) -> &mut Self
        where
            D: HirDisplay,
            I: IntoIterator<Item = D>,
        {
            for item in items {
                self.entry(&item, source, file, env);
            }
            self
        }
    }
}
use helpers::*;

pub trait HirDisplay {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result;
}

impl<T: ?Sized + HirDisplay> HirDisplay for &T {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        (*self).fmt(source, file, env, f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayHir<'env, 'hir, 'src> {
    pub source: &'src SourceMap,
    pub env: &'env ScopeEnvironment,
    pub hir: &'hir HirUnit,
}

impl fmt::Display for DisplayHir<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.hir.fmt(self.source, self.hir.file, self.env, f)
    }
}

impl HirDisplay for ItemId {
    #[expect(
        clippy::only_used_in_recursion,
        reason = "the parameter is required in the trait definition, so it can't be removed. \
            and because it IS used in recursion, it can't be marked with `_` either."
    )]
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let item = env.get_decl(*self).expect("HIR IDs should be valid");
        write!(f, "{}\x1b[94m{{", source.span_str(item.decl))?;
        if let Some(parent) = item.parent {
            f.write_str("\x1b[0m")?;
            parent.fmt(source, file, env, f)?;
            f.write_str("\x1b[90m::\x1b[94m")?;
        }
        write!(f, "{self}}}\x1b[0m")
    }
}

impl HirDisplay for ModId {
    fn fmt(
        &self,
        _source: &SourceMap,
        _file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let module = env.get_module(*self).expect("HIR IDs should be valid");
        write!(f, "{}\x1b[92m{{{}}}\x1b[0m", module.name, self)
    }
}

impl HirDisplay for hir::TypeExpr {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::TypeExpr::Primitive(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::Instance(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::Fallible(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::Ref(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::View(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::Assoc(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::FunPtr(inner) => inner.fmt(source, file, env, f),
            hir::TypeExpr::Opaque(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::PrimitiveType {
    fn fmt(
        &self,
        _source: &SourceMap,
        _file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Nevr => "nevr",
            Self::Fail => "fail",
            Self::Bool => "bool",
            Self::Byte => "byte",
            Self::Uint => "uint",
            Self::Sint => "sint",
            Self::Flpt => "flpt",
            Self::Char => "char",
            Self::Text => "text",
        })
    }
}

impl HirDisplay for hir::Typename {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { path, name } = self;
        for part in path {
            write!(f, "{}::", &source[file][part.name])?;
        }
        f.write_str(&source[file][*name])
    }
}

impl HirDisplay for hir::GenericGroup {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(params) = self;
        f.write_str("<")?;
        let mut has_prev = false;
        for param in params {
            if has_prev {
                f.write_str(", ")?;
            } else {
                has_prev = true;
            }
            param.fmt(source, file, env, f)?;
        }
        f.write_str(">")
    }
}

impl HirDisplay for hir::TypeInstance {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { name, args } = self;
        name.fmt(source, file, env, f)?;
        if let Some(args) = &args {
            args.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::FallType {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { then_ty, ty } = self;
        then_ty.fmt(source, file, env, f)?;
        write!(f, " | ")?;
        f.write_str(match ty {
            hir::OrType::None => "none",
            hir::OrType::Fail => "fail",
            hir::OrType::Nevr => "nevr",
        })
    }
}

impl HirDisplay for hir::RefType {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { ty } = self;
        f.write_str("@")?;
        ty.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::ViewType {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { item, count } = self;
        f.write_str("[")?;
        item.fmt(source, file, env, f)?;
        if let Some(_count) = &count {
            f.write_str("; ")?;
            todo!("expr display")
        }
        f.write_str("]")
    }
}

impl HirDisplay for hir::AssocType {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { base, assoc } = self;
        base.fmt(source, file, env, f)?;
        f.write_str(Punctuation::PathSep.as_str())?;
        assoc.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::AssocTypeSource {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Simple(inner) => inner.fmt(source, file, env, f),

            Self::Grouped(inner) => {
                f.write_str("<")?;
                inner.fmt(source, file, env, f)?;
                f.write_str(">")
            }
        }
    }
}

impl HirDisplay for hir::HirUnit {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { file: _, decls } = self;
        for decl in decls {
            decl.fmt(source, file, env, f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::SelfParam {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id } = self;
        id.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::FunParam {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, ty } = self;
        id.fmt(source, file, env, f)?;
        f.write_str(": ")?;
        ty.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::FunType {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { params, ret } = self;
        write!(f, "{}(", Keyword::Fun)?;
        let mut has_prev = false;
        for arg in params {
            if has_prev {
                f.write_str(", ")?;
            } else {
                has_prev = true;
            }
            arg.fmt(source, file, env, f)?;
        }
        f.write_str(")")?;
        if let Some(ret) = ret {
            write!(f, " {} ", Punctuation::Arrow)?;
            ret.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::OpaqueType {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(bound) = self;
        write!(f, ":( ")?;
        bound.fmt(source, file, env, f)?;
        f.write_str(" )")
    }
}

impl HirDisplay for hir::ConjunctionBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(bounds) = self;
        let mut has_prev = false;
        f.write_str("(")?;
        for bound in bounds {
            if has_prev {
                f.write_str(") + (")?;
            } else {
                has_prev = true;
            }
            bound.fmt(source, file, env, f)?;
        }
        f.write_str(")")
    }
}

impl HirDisplay for hir::DisjunctionBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(bounds) = self;
        let mut has_prev = false;
        f.write_str("(")?;
        for bound in bounds {
            if has_prev {
                f.write_str(") | (")?;
            } else {
                has_prev = true;
            }
            bound.fmt(source, file, env, f)?;
        }
        f.write_str(")")
    }
}

impl HirDisplay for hir::SelectionBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(bounds) = self;
        let mut has_prev = false;
        f.write_str("(")?;
        for bound in bounds {
            if has_prev {
                f.write_str(") / (")?;
            } else {
                has_prev = true;
            }
            bound.fmt(source, file, env, f)?;
        }
        f.write_str(")")
    }
}

impl HirDisplay for hir::NegativeBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(bound) = self;
        f.write_str("!(")?;
        bound.fmt(source, file, env, f)?;
        f.write_str(")")
    }
}

impl HirDisplay for hir::ExactTypeBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(ty) = self;
        write!(f, "{} ", Keyword::Only)?;
        ty.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::MemberBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { member, req } = self;
        write!(f, "{} ", Keyword::Has)?;
        f.write_str(&source[file][*member])?;
        if let Some(req) = req {
            f.write_str(": ")?;
            req.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::ShapeBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(shape) = self;
        shape.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::OverloadBound {
    fn fmt(
        &self,
        _source: &SourceMap,
        _file: FileId,
        _env: &ScopeEnvironment,
        _f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self {} = self;
        todo!()
    }
}

impl HirDisplay for hir::ExprBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { param: name, expr } = self;
        write!(f, "{} ", Keyword::For)?;
        name.fmt(source, file, env, f)?;
        f.write_str(": ")?;
        expr.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::TypeBound {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Conjunction(inner) => inner.fmt(source, file, env, f),
            Self::Disjunction(inner) => inner.fmt(source, file, env, f),
            Self::Selection(inner) => inner.fmt(source, file, env, f),
            Self::Not(inner) => inner.fmt(source, file, env, f),
            Self::Exact(inner) => inner.fmt(source, file, env, f),
            Self::Has(inner) => inner.fmt(source, file, env, f),
            Self::Shape(inner) => inner.fmt(source, file, env, f),
            Self::Supports(inner) => inner.fmt(source, file, env, f),
            Self::Satisfies(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::TypeBoundClause {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { target, bound } = self;
        target.fmt(source, file, env, f)?;
        f.write_str(": ")?;
        bound.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::IffClause {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(bounds) = self;
        write!(f, "{} ", Punctuation::Iff)?;
        for bound in bounds {
            bound.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::Decl {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Decl::Mod(inner) => inner.fmt(source, file, env, f),
            hir::Decl::On(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Fun(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Rec(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Cat(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Uni(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Let(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Pvt(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Dfer(inner) => inner.fmt(source, file, env, f),
            hir::Decl::Stmt(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::DeclMod {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id } = self;
        write!(f, "{} ", Keyword::Mod)?;
        id.fmt(source, file, env, f)?;
        f.write_str(";")
    }
}

impl HirDisplay for hir::DeclOn {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self {
            generics,
            target,
            iff,
            members,
        } = self;
        write!(f, "{}", Keyword::On)?;
        if let Some(generics) = generics {
            generics.fmt(source, file, env, f)?;
        }
        target.fmt(source, file, env, f)?;
        todo!("display for {iff:?} {members:?}")
    }
}

impl HirDisplay for hir::DeclFun {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self {
            id,
            generics,
            self_param,
            params,
            ret,
            iff,
            body,
        } = self;
        write!(f, "{} ", Keyword::Fun)?;
        id.fmt(source, file, env, f)?;
        if let Some(generics) = generics {
            generics.fmt(source, file, env, f)?;
        }
        if let Some(self_param) = self_param {
            self_param.fmt(source, file, env, f)?;
        }
        f.write_str("(")?;
        let mut has_prev = false;
        for param in params {
            if has_prev {
                f.write_str(", ")?;
            } else {
                has_prev = true;
            }
            param.fmt(source, file, env, f)?;
        }
        f.write_str(")")?;
        if let Some(ret) = ret {
            f.write_str(" -> ")?;
            ret.fmt(source, file, env, f)?;
        }
        if let Some(iff) = iff {
            iff.fmt(source, file, env, f)?;
        }
        if let Some(body) = body {
            body.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::DeclRec {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, fields } = self;
        write!(f, "{} ", Keyword::Rec)?;
        id.fmt(source, file, env, f)?;
        f.write_str(" ")?;
        HirBlock::new(f).entries(fields, source, file, env).finish()
    }
}

impl HirDisplay for hir::Field {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, ty } = self;
        id.fmt(source, file, env, f)?;
        f.write_str(": ")?;
        ty.fmt(source, file, env, f)?;
        f.write_str(",")
    }
}

impl HirDisplay for hir::DeclCat {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, discriminants } = self;
        write!(f, "{} ", Keyword::Cat)?;
        id.fmt(source, file, env, f)?;
        HirBlock::new(f)
            .entries(discriminants, source, file, env)
            .finish()
    }
}

impl HirDisplay for hir::Discriminant {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, val } = self;
        id.fmt(source, file, env, f)?;
        val.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::DeclUni {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, ty, init } = self;
        write!(f, "{} ", Keyword::Uni)?;
        if let Some(ty) = ty {
            f.write_str(": ")?;
            ty.fmt(source, file, env, f)?;
        }
        if let Some(init) = init {
            f.write_str(" = ")?;
            init.fmt(source, file, env, f)?;
        }
        id.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::DeclLet {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, ty, init } = self;
        write!(f, "{} ", Keyword::Let)?;
        id.fmt(source, file, env, f)?;
        if let Some(ty) = ty {
            f.write_str(": ")?;
            ty.fmt(source, file, env, f)?;
        }
        if let Some(init) = init {
            f.write_str(" = ")?;
            init.fmt(source, file, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::DeclPvt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, ty, init } = self;
        write!(f, "{} ", Keyword::Pvt)?;
        id.fmt(source, file, env, f)?;
        if let Some(ty) = ty {
            f.write_str(": ")?;
            ty.fmt(source, file, env, f)?;
        }
        f.write_str(" = ")?;
        init.fmt(source, file, env, f)?;
        f.write_str(";")
    }
}

impl HirDisplay for hir::DeclDfer {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { param, on_pop } = self;
        write!(f, "{}", Keyword::Dfer)?;
        if let Some(param) = param {
            f.write_str("(")?;
            param.fmt(source, file, env, f)?;
            f.write_str(")")?;
        }
        f.write_str(" ")?;
        on_pop.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::Stmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Stmt::Expr(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Print(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::If(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Pick(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Loop(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Cord(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Rep(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::For(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Give(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Fail(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Done(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Skip(inner) => inner.fmt(source, file, env, f),
            hir::Stmt::Block(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::ExprStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { expr } = self;
        expr.fmt(source, file, env, f)?;
        f.write_str(";")
    }
}

impl HirDisplay for hir::Expr {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Expr::Block(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Binary(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Unary(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Index(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Call(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Literal(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Directive(inner) => inner.fmt(source, file, env, f),
            hir::Expr::Variable(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::BlockExpr {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::BlockExpr::If(inner) => inner.fmt(source, file, env, f),
            hir::BlockExpr::Loop(inner) => inner.fmt(source, file, env, f),
            hir::BlockExpr::Cord(inner) => inner.fmt(source, file, env, f),
            hir::BlockExpr::Block(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::Binary {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { op, lhs, rhs } = self;
        f.write_str("(")?;
        lhs.fmt(source, file, env, f)?;
        write!(f, " {op} ")?;
        rhs.fmt(source, file, env, f)?;
        f.write_str(")")
    }
}

impl HirDisplay for hir::Unary {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { op, operand, side } = self;
        f.write_str("(")?;
        match side {
            OpSide::Postfix => {
                operand.fmt(source, file, env, f)?;
                fmt::Display::fmt(op.as_str(), f)?;
            }
            OpSide::Prefix => {
                fmt::Display::fmt(op.as_str(), f)?;
                operand.fmt(source, file, env, f)?;
            }
        }
        f.write_str(")")
    }
}

impl HirDisplay for hir::Subscript {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { arr, idx } = self;
        arr.fmt(source, file, env, f)?;
        f.write_str("[")?;
        idx.fmt(source, file, env, f)?;
        f.write_str("]")
    }
}

impl HirDisplay for hir::FnCall {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { fun, args } = self;
        fun.fmt(source, file, env, f)?;
        f.write_str("(")?;
        let mut has_prev = false;
        for arg in args {
            if has_prev {
                f.write_str(", ")?;
            }
            has_prev = true;
            arg.fmt(source, file, env, f)?;
        }
        f.write_str(")")
    }
}

impl HirDisplay for hir::Literal {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { kind: _, span } = self;
        write!(f, "{}", &source[file][*span])
    }
}

impl HirDisplay for hir::Directive {
    fn fmt(
        &self,
        _source: &SourceMap,
        _file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::Directive::Inpt => f.write_str(Keyword::Inpt.as_str()),
        }
    }
}

impl HirDisplay for hir::Variable {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(id) = self;
        id.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::PrintStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { expr } = self;
        write!(f, "{}(", Keyword::Prnt)?;
        expr.fmt(source, file, env, f)?;
        f.write_str(");")
    }
}

impl HirDisplay for hir::IfStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { pred, then, or } = self;
        write!(f, "{} ", Keyword::If)?;
        pred.fmt(source, file, env, f)?;
        f.write_str(" ")?;
        then.fmt(source, file, env, f)?;
        if let Some(or) = or.as_deref() {
            write!(f, " {} ", Keyword::Or)?;
            or.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl HirDisplay for hir::OrStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            hir::OrStmt::More(inner) => inner.fmt(source, file, env, f),
            hir::OrStmt::Last(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl HirDisplay for hir::PickExpr {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { arms } = self;
        write!(f, "{} ", Keyword::Pick)?;
        HirBlock::new(f).entries(arms, source, file, env).finish()
    }
}

impl HirDisplay for hir::PickArm {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { pat, expr } = self;
        pat.fmt(source, file, env, f)?;
        write!(f, " {} ", Punctuation::FatArrow)?;
        expr.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::PickPattern {
    fn fmt(
        &self,
        _source: &SourceMap,
        _file: FileId,
        _env: &ScopeEnvironment,
        _f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self {} = self;
        todo!("pick pattern")
    }
}

impl HirDisplay for hir::LoopStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { label, body } = self;
        if let Some(label) = label {
            label.fmt(source, file, env, f)?;
            f.write_str(": ")?;
        }
        write!(f, "{} ", Keyword::Loop)?;
        body.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::CordStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { label, body } = self;
        if let Some(label) = label {
            label.fmt(source, file, env, f)?;
            f.write_str(": ")?;
        }
        write!(f, "{} ", Keyword::Cord)?;
        body.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::RepStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { pred, body } = self;
        write!(f, "{} ", Keyword::Rep)?;
        pred.fmt(source, file, env, f)?;
        body.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::BindingPattern {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { var } = self;
        var.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::ForStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { var, iter, body } = self;
        write!(f, "{} ", Keyword::For)?;
        var.fmt(source, file, env, f)?;
        write!(f, " {} ", Keyword::In)?;
        iter.fmt(source, file, env, f)?;
        body.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::Label {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { name } = self;
        write!(f, ".{}", &source[file][*name])
    }
}

impl HirDisplay for hir::GiveStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { expr } = self;
        write!(f, "{}", Keyword::Give)?;
        if let Some(expr) = expr {
            f.write_str(" ")?;
            expr.fmt(source, file, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::FailStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { expr } = self;
        write!(f, "{}", Keyword::Fail)?;
        if let Some(expr) = expr {
            f.write_str(" ")?;
            expr.fmt(source, file, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::DoneStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { label, expr } = self;
        write!(f, "{}", Keyword::Done)?;
        if let Some(label) = label {
            f.write_str(" ")?;
            label.fmt(source, file, env, f)?;
        }
        if let Some(expr) = expr {
            f.write_str(" ")?;
            expr.fmt(source, file, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::SkipStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { label } = self;
        write!(f, "{}", Keyword::Skip)?;
        if let Some(label) = label {
            f.write_str(" ")?;
            label.fmt(source, file, env, f)?;
        }
        f.write_str(";")
    }
}

impl HirDisplay for hir::BlockStmt {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(decls) = self;
        HirBlock::new(f).entries(decls, source, file, env).finish()
    }
}
