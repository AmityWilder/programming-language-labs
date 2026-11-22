#![deny(
    clippy::rest_pat_in_fully_bound_structs,
    clippy::rest_pattern_accessible_field,
    unused_variables,
    clippy::todo,
    reason = "need to be aware of when the structure changes so its formatting can update"
)]

use crate::{
    analyzer::static_env::{
        ItemId, ModId,
        scope::{
            env::{Item, ScopeEnvironment, ScopeId},
            hir::{self, HirUnit},
        },
    },
    lex::token::{
        Operator,
        keyword::Keyword,
        punc::{OpSide, Punctuation},
    },
    parse::ast,
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
        pub const INDENT: &str = "  ";

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

#[derive(Clone, Copy)]
pub struct DisplayHir<'env, 'hir, 'src, S: hir::HirPass> {
    pub source: &'src SourceMap,
    pub env: &'env ScopeEnvironment,
    pub hir: &'hir HirUnit<S>,
}

impl<S: hir::HirPass> fmt::Display for DisplayHir<'_, '_, '_, S> {
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
        let Item {
            decl,
            kind,
            parent,
            children: _,
            scope,
        } = *item;
        write!(f, "{}\x1b[94m{{", &source[decl])?;
        if let Some(parent) = parent {
            f.write_str("\x1b[0m")?;
            parent.fmt(source, file, env, f)?;
            f.write_str("\x1b[90m::\x1b[94m")?;
        }
        {
            f.write_str("\x1b[95m")?;
            match scope {
                ScopeId::Global => f.write_str("global")?,
                ScopeId::Local { layer, scope } => {
                    write!(f, "{layer}.")?;
                    if scope == 0 {
                        f.write_str("root")?;
                    } else {
                        write!(f, "{scope}")?;
                    }
                }
            }
            f.write_str(":\x1b[94m")?;
        }
        write!(f, "{self}\x1b[96m#{kind:?}\x1b[94m}}\x1b[0m")
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

impl<S: hir::HirPass> HirDisplay for hir::TypeExpr<S> {
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

impl HirDisplay for hir::UnlinkedTypename {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(x) = self;
        let crate::parse::ast::Typename { path, name } = x;
        for part in path {
            write!(f, "{}::", &source[file][part.name])?;
        }
        f.write_str(&source[file][*name])
    }
}

impl<S: hir::HirPass> HirDisplay for hir::GenericParam<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(inner) = self;
        inner.fmt(source, file, env, f)
    }
}

impl<S: hir::HirPass> HirDisplay for hir::GenericParams<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::GenericArgs<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::TypeInstance<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::FallType<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::RefType<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::ViewType<S> {
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
        if let Some(count) = &count {
            f.write_str("; ")?;
            count.fmt(source, file, env, f)?;
        }
        f.write_str("]")
    }
}

impl<S: hir::HirPass> HirDisplay for hir::AssocType<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::AssocTypeSource<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::HirUnit<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::SelfParam<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::FunParam<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::FunType<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::OpaqueType<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::ComboBound<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { cmb, bounds } = self;
        let mut has_prev = false;
        f.write_str("(")?;
        for bound in bounds {
            if has_prev {
                write!(f, ") {cmb} (")?;
            } else {
                has_prev = true;
            }
            bound.fmt(source, file, env, f)?;
        }
        f.write_str(")")
    }
}

impl<S: hir::HirPass> HirDisplay for hir::NegativeBound<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::ExactTypeBound<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::MemberBound<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::ShapeBound<S> {
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
        Ok(())
    }
}

impl<S: hir::HirPass> HirDisplay for hir::ExprBound<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::TypeBound<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::Combo(inner) => inner.fmt(source, file, env, f),
            Self::Not(inner) => inner.fmt(source, file, env, f),
            Self::Exact(inner) => inner.fmt(source, file, env, f),
            Self::Has(inner) => inner.fmt(source, file, env, f),
            Self::Shape(inner) => inner.fmt(source, file, env, f),
            Self::Supports(inner) => inner.fmt(source, file, env, f),
            Self::Satisfies(inner) => inner.fmt(source, file, env, f),
        }
    }
}

impl<S: hir::HirPass> HirDisplay for hir::TypeBoundClause<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::IffClause<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::Decl<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::DeclMod<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::DeclOn<S> {
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
        write!(f, "{} ", Keyword::On)?;
        if let Some(generics) = generics {
            generics.fmt(source, file, env, f)?;
        }
        target.fmt(source, file, env, f)?;
        if let Some(iff) = iff {
            write!(f, " {} ", Punctuation::Iff)?;
            let mut has_prev = false;
            for clause in &iff.0 {
                if has_prev {
                    // TODO: multiline
                    f.write_str(", ")?;
                } else {
                    has_prev = true;
                }
                clause.fmt(source, file, env, f)?;
            }
        }
        f.write_str(" ")?;
        HirBlock::new(f)
            .entries(members, source, file, env)
            .finish()
    }
}

impl<S: hir::HirPass> HirDisplay for hir::DeclFun<S> {
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
        f.write_str("(")?;
        if let Some(self_param) = self_param {
            self_param.fmt(source, file, env, f)?;
        }
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
            writeln!(f)?;
            iff.fmt(source, file, env, f)?;
        }
        if let Some(body) = body {
            f.write_str(if iff.is_some() { "\n" } else { " " })?;
            body.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl<S: hir::HirPass> HirDisplay for hir::DeclRec<S> {
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
            fields,
        } = self;
        write!(f, "{} ", Keyword::Rec)?;
        id.fmt(source, file, env, f)?;
        if let Some(generics) = generics {
            generics.fmt(source, file, env, f)?;
        }
        f.write_str(" ")?;
        HirBlock::new(f).entries(fields, source, file, env).finish()
    }
}

impl<S: hir::HirPass> HirDisplay for hir::Field<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::DeclCat<S> {
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
        f.write_str(" ")?;
        HirBlock::new(f)
            .entries(discriminants, source, file, env)
            .finish()
    }
}

impl<S: hir::HirPass> HirDisplay for hir::Discriminant<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, val } = self;
        id.fmt(source, file, env, f)?;
        if let Some(val) = val {
            val.fmt(source, file, env, f)?;
        }
        Ok(())
    }
}

impl<S: hir::HirPass> HirDisplay for hir::DeclUni<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { id, ty, init } = self;
        write!(f, "{} ", Keyword::Uni)?;
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

impl<S: hir::HirPass> HirDisplay for hir::DeclLet<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::DeclPvt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::DeclDfer<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::Stmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::ExprStmt<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(expr) = self;
        expr.fmt(source, file, env, f)?;
        f.write_str(";")
    }
}

impl<S: hir::HirPass> HirDisplay for hir::Expr<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::BlockExpr<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::Binary<S> {
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
        let is_unspaced = matches!(op, Operator::Dot);
        write!(f, "{0}{op}{0}", if is_unspaced { "" } else { " " })?;
        rhs.fmt(source, file, env, f)?;
        f.write_str(")")
    }
}

impl<S: hir::HirPass> HirDisplay for hir::Unary<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::Subscript<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::FnCall<S> {
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

impl HirDisplay for hir::UnlinkedDecl {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { kind: _, ident } = self;
        f.write_str(&source[file][*ident])
    }
}

impl HirDisplay for hir::LinkedDecl {
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

impl HirDisplay for hir::UnlinkedMod {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(decl) = self;
        // TBD: are kw and semi needed?
        let ast::DeclMod {
            kw: _,
            name,
            semi: _,
        } = decl;
        f.write_str(&source[file][*name])
    }
}

impl HirDisplay for hir::LinkedMod {
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

impl HirDisplay for hir::UnlinkedLiteral {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(inner) = self;
        let ast::Literal { kind: _, span } = inner;
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

impl HirDisplay for hir::UnlinkedVar {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(var) = self;
        let crate::parse::ast::Variable { path, ident } = var;
        if let Some(path) = path {
            for part in path {
                write!(f, "{}::", &source[file][part.ident])?;
            }
        }
        write!(f, "{}", &source[file][*ident])
    }
}

impl HirDisplay for hir::LinkedVar {
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

impl HirDisplay for hir::LinkedTypename {
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

impl<S: hir::HirPass> HirDisplay for hir::PrintStmt<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(expr) = self;
        write!(f, "{}(", Keyword::Prnt)?;
        expr.fmt(source, file, env, f)?;
        f.write_str(");")
    }
}

impl<S: hir::HirPass> HirDisplay for hir::IfStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::OrStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::PickExpr<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { expr, arms } = self;
        write!(f, "{} ", Keyword::Pick)?;
        expr.fmt(source, file, env, f)?;
        f.write_str(" ")?;
        HirBlock::new(f).entries(arms, source, file, env).finish()
    }
}

impl<S: hir::HirPass> HirDisplay for hir::PickArm<S> {
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
        Ok(())
    }
}

impl<S: hir::HirPass> HirDisplay for hir::LoopStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::CordStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::RepStmt<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { label, pred, body } = self;
        if let Some(label) = label {
            label.fmt(source, file, env, f)?;
            f.write_str(": ")?;
        }
        write!(f, "{} ", Keyword::Rep)?;
        pred.fmt(source, file, env, f)?;
        body.fmt(source, file, env, f)
    }
}

impl<S: hir::HirPass> HirDisplay for hir::BindingPattern<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::ForStmt<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self {
            label,
            var,
            iter,
            body,
        } = self;
        if let Some(label) = label {
            label.fmt(source, file, env, f)?;
            f.write_str(": ")?;
        }
        write!(f, "{} ", Keyword::For)?;
        var.fmt(source, file, env, f)?;
        write!(f, " {} ", Keyword::In)?;
        iter.fmt(source, file, env, f)?;
        body.fmt(source, file, env, f)
    }
}

impl HirDisplay for hir::UnlinkedLabel {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        _env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self(name) = self;
        write!(f, ".{}", &source[file][*name])
    }
}

impl<S: hir::HirPass> HirDisplay for hir::GiveStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::FailStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::DoneStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::SkipStmt<S> {
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

impl<S: hir::HirPass> HirDisplay for hir::BlockStmt<S> {
    fn fmt(
        &self,
        source: &SourceMap,
        file: FileId,
        env: &ScopeEnvironment,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let Self { open: _, decls } = self;
        HirBlock::new(f).entries(decls, source, file, env).finish()
    }
}
