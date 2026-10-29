#![allow(dead_code, reason = "under construction")]

use crate::{
    SYNTAX_STYLE_ANSI, Todo,
    analyzer::types::{
        AbstractType, FunShape,
        bounds::{Combinator, TypeBound},
    },
    highlight::style::StyleWrapper,
};
use std::{
    assert_matches, borrow::Cow, collections::BTreeMap, fmt::Display, iter::once,
    range::RangeInclusive,
};

/// WARNING: this method may cause a memory leak unless the number of names written are tracked.
/// It will allocate strings until the spare capacity without pushing them, so the vec won't know to drop them.
fn silly_names() -> impl Iterator<Item = Cow<'static, str>> {
    const EZ: [&str; 4] = ["Foo", "Bar", "Baz", "Bat"];
    EZ.into_iter().map(Cow::Borrowed).chain(
        // after we run out of hard-coded names, start naming them
        // `Foo1`, `Bar1`, `Baz1`, `Foo2`, `Bar2`, `Baz2`, etc.
        (1..)
            .flat_map(|i| std::iter::repeat(i).zip(EZ))
            .map(|(i, name)| Cow::Owned(format!("{name}{i}"))),
    )
}

/// WARNING: this method may cause a memory leak unless the number of names written are tracked.
/// It will allocate strings until the spare capacity without pushing them, so the vec won't know to drop them.
fn _silly_names(buf: &mut Vec<String>) -> impl Iterator<Item = &str> {
    const EZ: [&str; 4] = ["Foo", "Bar", "Baz", "Bat"];
    std::iter::chain(
        EZ,
        // after we run out of hard-coded names, start naming them
        // `Foo1`, `Bar1`, `Baz1`, `Foo2`, `Bar2`, `Baz2`, etc.
        (1..)
            .flat_map(|i| std::iter::repeat(i).zip(EZ))
            .zip(buf.spare_capacity_mut())
            .map(|((i, name), slot)| slot.write(format!("{name}{i}")).as_str()),
    )
}

/// A wrapper for [`silly_names`] that automatically tracks the added names so they can be safely dropped.
#[derive(Debug)]
struct NameGenerator {
    buf: Vec<String>,
    names_used: usize,
}

impl Drop for NameGenerator {
    fn drop(&mut self) {
        // SAFETY: `names_used` is equal to `buf.len()` + the number of elements that were written to it
        unsafe {
            self.buf.set_len(self.names_used);
        }
    }
}

impl NameGenerator {
    pub const fn new(buf: Vec<String>) -> Self {
        Self {
            names_used: buf.len(),
            buf,
        }
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &str> {
        _silly_names(&mut self.buf)
            .inspect(|_| self.names_used = self.names_used.strict_add(self.names_used))
    }
}

impl TypeBound<'_> {
    fn let_statements<T>(
        &self,
        reason: &T,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<bool, std::fmt::Error>
    where
        T: Reason,
    {
        match self {
            &TypeBound::Combo(kind, ref all_bounds) => let_if_new(
                once((self, None)).chain({
                    reason
                        .aliases_needed()
                        .into_iter()
                        .map(|x| {
                            let i = all_bounds
                                .iter()
                                .position(|b| b == x) // remember that combo bounds aren't necessarily ordered
                                .expect("every item output by aliases_needed on a Combo should be an element of the Combo vec");
                            (x, Some((Some((i, kind)), self)))
                        })
                }),
                f,
            ),

            TypeBound::Exactly(x) => {
                let items: [(&dyn AliasPrefix, _); 2] = [(self, None), (x, Some((None, self)))];
                let_if_new(items, f)
            }

            TypeBound::Not(x)
            | TypeBound::Has {
                member: _,
                req: Some(x),
            } => let_if_new([(self, None), (x.as_ref(), Some((None, self)))], f),

            TypeBound::Has {
                member: _,
                req: None,
            }
            | TypeBound::Shape(_)
            | TypeBound::Supports {
                op: _,
                operands: _,
                ret: _,
            }
            | TypeBound::Satisfies { ..} => let_if_new(once((self, None)), f),
        }
    }

    fn requirement(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the bound `{:#}` requires ", Aliased::new(self))?;
        match self {
            Self::Combo(kind, _) => {
                f.write_str(match kind {
                    Combinator::All => "every",
                    Combinator::Any => "at least",
                    Combinator::One => "exactly",
                })?;
                f.write_str(" one of its bounds to be fulfilled")
            }

            Self::Not(inner) => {
                write!(
                    f,
                    "its bound `{:#}` to be unfulfilled",
                    Aliased::new(inner.as_ref())
                )
            }

            Self::Exactly(ty) => write!(f, "exactly `{:#}`", Aliased::new(ty)),

            Self::Has { member, req } => {
                write!(
                    f,
                    "a member `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )?;
                if let Some(req) = req {
                    write!(f, " satisfying `{:#}`", Aliased::new(req.as_ref()))?;
                }
                f.write_str(" to exist")
            }

            Self::Shape(FunShape { params, ret }) => {
                write!(f, "callability with arguments ")?;
                let mut has_prev = false;
                for param in params {
                    if has_prev {
                        f.write_str(", ")?;
                    }
                    has_prev = true;
                    write!(f, "`{param:#}`")?;
                }
                write!(f, " for an output of `{ret:#}`")
            }

            Self::Supports { .. } => todo!(),

            Self::Satisfies { .. } => todo!(),
        }
    }
}

mod local_depth {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    // HACK: using global state may have side-effects!
    thread_local! {
        static BOUNDS_DIAGNOSIS_DEPTH: Cell<usize> = const { Cell::new(0) };
    }

    pub struct LocalDepth(());

    impl LocalDepth {
        pub fn new() -> Self {
            BOUNDS_DIAGNOSIS_DEPTH.update(|n| {
                n.checked_add(1)
                    .expect("encountered stack overflow while evaluating bounds error")
            });
            Self(())
        }
    }

    impl Drop for LocalDepth {
        fn drop(&mut self) {
            BOUNDS_DIAGNOSIS_DEPTH.update(|n| {
                n.checked_sub(1)
                    .expect("popped more bounds than were pushed")
            });
        }
    }

    pub struct Indent;

    impl std::fmt::Display for Indent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let n = BOUNDS_DIAGNOSIS_DEPTH.get().saturating_mul(4);
            write!(f, "{:n$}", "")
        }
    }

    // HACK: using global state may have side-effects!
    thread_local! {
        static MAX_ID: Cell<usize> = const { Cell::new(0) };
        /// Maps a stringified [`TypeBound`] to a name
        static DEFS: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
    }

    /// Defines a new variable without an actual definition. It will not appear in the table.
    pub fn create_var() -> usize {
        MAX_ID.replace(
            MAX_ID
                .get()
                .checked_add(1)
                .expect("stack overflow while naming bounds"),
        )
    }

    /// Returns [`None`] if the name is too short to require an alias.
    /// Boolean field represents whether the name is new.
    pub fn define<T: ?Sized + ToString>(bound: &T) -> Option<(bool, usize)> {
        const MAX_INLINE_BOUND_CHARS: usize = 4;
        DEFS.with_borrow_mut(|table| {
            let key = bound.to_string();
            (key.chars().count() > MAX_INLINE_BOUND_CHARS).then(|| {
                let entry = table.entry(key);
                let is_new = matches!(entry, std::collections::hash_map::Entry::Vacant(_));
                let id = entry.or_insert_with(create_var);
                (is_new, *id)
            })
        })
    }

    /// Always call this when finished displaying an error!
    pub fn clear_defs() {
        MAX_ID.set(0);
        DEFS.with_borrow_mut(|table| {
            table.clear();
        });
    }
}
use local_depth::{Indent, LocalDepth, clear_defs, create_var, define};

trait AliasPrefix: Display {
    fn kind_name(&self) -> &'static str;
}

impl<T: ?Sized + AliasPrefix> AliasPrefix for &T {
    fn kind_name(&self) -> &'static str {
        (*self).kind_name()
    }
}

impl AliasPrefix for AbstractType<'_> {
    fn kind_name(&self) -> &'static str {
        "type"
    }
}

impl AliasPrefix for TypeBound<'_> {
    fn kind_name(&self) -> &'static str {
        "bound"
    }
}

impl AliasPrefix for CheckedType<'_> {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Abstract(t) => t.kind_name(),
            Self::Bound(b) => b.kind_name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct MetaType(usize);

impl Display for MetaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let style = SYNTAX_STYLE_ANSI.typename;
        if f.alternate() {
            style.begin().fmt(f)?;
        }
        // TODO: what if there's already has a type parameter with that name present in the bounds?
        let mut n = self.0;
        loop {
            let r;
            (n, r) = (n / 26, n % 26);
            #[expect(
                clippy::arithmetic_side_effects,
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                reason = "0-26 and b'A' + 26 are guaranteed to fit in u8 without truncation"
            )]
            write!(f, "{}", char::from(b'A' + r as u8))?;
            if n < 26 {
                break;
            }
        }
        if f.alternate() {
            style.end().fmt(f)?;
        }
        Ok(())
    }
}

#[repr(transparent)]
struct Aliased<T: ?Sized>(T);

impl<T: ?Sized> Aliased<T> {
    const fn new(value: &T) -> &Self {
        // SAFETY: `Aliased` is a transparent wrapper for `T`
        unsafe { std::mem::transmute(value) }
    }
}

impl<T: ?Sized + Display + AliasPrefix> Display for Aliased<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((is_new, id)) = define(&self.0) {
            assert!(!is_new, "should define an alias before referencing it");
            MetaType(id).fmt(f)
        } else {
            self.0.fmt(f)
        }
    }
}

// TODO: what if we KNOW it's something new? how would we handle name collisions??
/// Does not add any trailing punctuation
fn let_if_new<'a, T, I>(iter: I, f: &mut std::fmt::Formatter<'_>) -> Result<bool, std::fmt::Error>
where
    T: 'a + ?Sized + Display + AliasPrefix,
    I: 'a
        + IntoIterator<
            Item = (
                &'a T,
                Option<(Option<(usize, Combinator)>, &'a TypeBound<'a>)>,
            ),
        >,
{
    const OVERSIZE_DEF_LEN: usize = 16;
    let _overflow = LocalDepth::new();
    let mut has_prev = false;
    let mut line_def_chars = 0;
    let iter = iter.into_iter();
    let (lo, hi) = iter.size_hint();
    let len = lo;
    debug_assert_eq!(
        Some(len),
        hi,
        "cannot require ExactSizeIterator because of Chain, but should still be exact size in practice"
    );
    for (i, (x, b)) in iter.enumerate() {
        if matches!(define(x), Some((true, _))) {
            if has_prev {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "has_prev implies `i` is at least 1"
                )]
                f.write_str(if i == len - 1 { " and" } else { "," })?;
                if line_def_chars > OVERSIZE_DEF_LEN {
                    writeln!(f)?;
                    Indent.fmt(f)?; // HACK: this is a sneaky lil mf
                } else {
                    f.write_str(" ")?;
                }
            } else {
                has_prev = true;
                f.write_str("let ")?;
            }
            line_def_chars = line_def_chars.saturating_add(x.to_string().chars().count());
            write!(
                f,
                "`{:#}` be the {} `{x:#}`",
                Aliased::new(x),
                x.kind_name()
            )?;
            if let Some((index, b)) = b {
                f.write_str(" from the ")?;
                if let Some((index, kind)) = index {
                    let style = SYNTAX_STYLE_ANSI.number_literal;
                    style.begin().fmt(f)?;
                    if let Some(n) = index.checked_add(1) {
                        let th = match n % 10 {
                            1 if n % 100 != 11 => "st",
                            2 if n % 100 != 12 => "nd",
                            3 if n % 100 != 13 => "rd",
                            _ => "th",
                        };
                        write!(f, "{n}{th}")
                    } else {
                        // an index exists, but overflows usize.
                        // no worries, instead of using a number, we can say the position!
                        f.write_str("last")
                    }?;
                    style.end().fmt(f)?;
                    // TODO: keep this up to date with the actual syntax!!
                    write!(
                        f,
                        " `{:#}`-separated sub-bound of ",
                        SYNTAX_STYLE_ANSI.keyword.alt_style(match kind {
                            Combinator::All => "+",
                            Combinator::Any => "|",
                            Combinator::One => "/",
                        })
                    )?;
                }
                write!(f, "bound `{:#}`", Aliased::new(b))?;
            }
        }
    }
    Ok(has_prev)
}

fn let_local(
    bound: &TypeBound<'_>,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<MetaType, std::fmt::Error> {
    let t = MetaType(create_var());
    write!(
        f,
        "let `{t:#}` be a type satisfying the bound `{:#}`",
        Aliased::new(bound)
    )
    .map(|()| t)
}

/// Assumes every local is an [`AbstractType`].
fn let_locals<'a, I>(
    iter: I,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<Vec<Option<RangeInclusive<usize>>>, std::fmt::Error>
where
    I: IntoIterator<IntoIter: 'a, Item = (&'a TypeBound<'a>, usize)>,
{
    let mut has_prev_group = false;
    f.write_str("let ")?;
    let iter = iter.into_iter();
    let (lo, hi) = iter.size_hint();
    debug_assert_eq!(
        Some(lo),
        hi,
        "cannot require ExactSizeIterator because of Chain, but should still be exact size in practice"
    );
    let num_groups = lo;
    iter.enumerate()
        .map(|(i, (bound, count))| {
            if has_prev_group {
                f.write_str(if i == num_groups {
                    // final group
                    ", and "
                } else {
                    ", "
                })?;
            }
            has_prev_group = true;
            let mut iter = std::iter::repeat_with(create_var).take(count);
            let Some(start) = iter.next() else {
                return Ok(None);
            };
            let last = iter.last().unwrap_or(start);
            let ids = RangeInclusive { start, last };
            let mut has_prev = false;
            for id in ids {
                if has_prev {
                    f.write_str(if id == last {
                        // final item
                        " and "
                    } else {
                        ", "
                    })?;
                }
                has_prev = true;
                write!(f, "`{:#}`", MetaType(id))?;
            }
            write!(
                f,
                " be {} satisfying the bound `{:#}`",
                if start == last { "a type" } else { "types" },
                Aliased::new(bound)
            )?;
            Ok(Some(ids))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExampleTypeRef<'brw, 'nam, 'ty, 'src> {
    Abstract(&'brw AbstractType<'src>),
    Bound(&'brw TypeBound<'src>),
    Rec(&'brw ExampleRec<'nam, 'ty, 'src>),
}

#[derive(Debug, PartialEq)]
pub enum ExampleTypeMut<'brw, 'nam, 'ty, 'src> {
    Abstract(&'brw mut AbstractType<'src>),
    Bound(&'brw mut TypeBound<'src>),
    Rec(&'brw mut ExampleRec<'nam, 'ty, 'src>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExampleType<'nam, 'ty, 'src> {
    Abstract(Cow<'ty, AbstractType<'src>>),
    Bound(Cow<'ty, TypeBound<'src>>),
    Checked(Cow<'ty, CheckedType<'src>>),
    Rec(ExampleRec<'nam, 'ty, 'src>),
}

impl<'nam, 'ty, 'src> ExampleType<'nam, 'ty, 'src> {
    pub const fn as_ref(&self) -> ExampleTypeRef<'_, 'nam, 'ty, 'src> {
        match self {
            Self::Abstract(Cow::Borrowed(x)) => ExampleTypeRef::Abstract(x),

            Self::Abstract(Cow::Owned(x))
            | Self::Checked(
                Cow::Borrowed(CheckedType::Abstract(x)) | Cow::Owned(CheckedType::Abstract(x)),
            ) => ExampleTypeRef::Abstract(x),

            Self::Bound(Cow::Borrowed(x)) => ExampleTypeRef::Bound(x),

            Self::Bound(Cow::Owned(x))
            | Self::Checked(
                Cow::Borrowed(CheckedType::Bound(x)) | Cow::Owned(CheckedType::Bound(x)),
            ) => ExampleTypeRef::Bound(x),

            Self::Rec(x) => ExampleTypeRef::Rec(x),
        }
    }

    pub fn to_mut(&mut self) -> ExampleTypeMut<'_, 'nam, 'ty, 'src> {
        match self {
            Self::Abstract(cow) => ExampleTypeMut::Abstract(cow.to_mut()),
            Self::Bound(cow) => ExampleTypeMut::Bound(cow.to_mut()),
            Self::Checked(cow) => match cow.to_mut() {
                CheckedType::Abstract(t) => ExampleTypeMut::Abstract(t),
                CheckedType::Bound(b) => ExampleTypeMut::Bound(b),
            },
            Self::Rec(x) => ExampleTypeMut::Rec(x),
        }
    }
}

impl Display for ExampleType<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abstract(x) => x.fmt(f),
            Self::Bound(x) => x.fmt(f),
            Self::Checked(x) => x.fmt(f),
            Self::Rec(x) => x.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExampleRec<'nam, 'ty, 'src> {
    pub name: Cow<'nam, str>,
    pub members: BTreeMap<&'src str, Option<ExampleType<'nam, 'ty, 'src>>>,
}

impl Display for ExampleRec<'_, '_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)?;
        f.write_str(" {")?;
        let mut has_prev = false;
        for (key, val) in &self.members {
            if has_prev {
                f.write_str(", ")?;
            } else {
                has_prev = true;
                f.write_str(" ")?;
            }
            SYNTAX_STYLE_ANSI.variable.alt_style(key).fmt(f)?;
            if let Some(val) = val {
                f.write_str(": ")?;
                val.fmt(f)?;
            }
        }
        f.write_str(" }")
    }
}

impl<'src> TypeBound<'src> {
    /// If an example of a type satisfying the bound exists, presents it.
    pub const fn trivial_example(&self) -> Option<&AbstractType<'src>> {
        if let Self::Exactly(t) = self {
            Some(t)
        } else {
            None
        }
    }

    /// Produces an example involving a made-up Foo/Bar type specifically constructed to fulfill the bound, if possible
    ///
    /// Should not consume a name if [`None`] is returned.
    pub fn foobar_example<'nam, 'ty, I>(
        &'ty self,
        names: &mut I,
    ) -> Option<ExampleType<'nam, 'ty, 'src>>
    where
        I: 'nam + Iterator<Item = Cow<'nam, str>>,
    {
        match self {
            Self::Combo(kind, bounds) => match kind {
                Combinator::All => {
                    if let Some(t) = bounds.iter().find_map(|bound| bound.trivial_example()) {
                        // assuming the restraint is legal, this type should satisfy all other bounds on its own.
                        // TODO: what if the example is being used to show why the restraint ISN'T legal?
                        Some(ExampleType::Abstract(Cow::Borrowed(t)))
                    } else {
                        let mut bounds = bounds.iter();
                        let mut base = bounds
                            .find(|bound| matches!(bound, Self::Has { .. }))?
                            .foobar_example(names)
                            .expect("`has` should succeed");
                        {
                            let ExampleType::Rec(base) = &mut base else {
                                unreachable!("filtered to `has`")
                            };
                            for bound in bounds {
                                // TODO: what if `has foo + !has foo`?
                                match bound {
                                    Self::Combo(_kind, _bounds) => todo!(),

                                    Self::Has { member, req } => {
                                        let ty = base.members.entry(member).or_default();
                                        if let Some(req) = req {
                                            if let Some(ty) = ty {
                                                match ty.to_mut() {
                                                    ExampleTypeMut::Abstract(_)
                                                    | ExampleTypeMut::Rec(_) => {
                                                        todo!(
                                                            "additional restraints on explicit type"
                                                        )
                                                    }

                                                    ExampleTypeMut::Bound(b) => {
                                                        b.extend_one(req.as_ref());
                                                        b.simplify();
                                                    }
                                                }
                                            } else {
                                                *ty = Some(req.foobar_example(names).unwrap_or(
                                                    ExampleType::Bound(Cow::Borrowed(req)),
                                                ));
                                            }
                                        }
                                    }

                                    Self::Exactly(_) => {
                                        unreachable!("should be handled by other branch")
                                    }

                                    // TOOD: aren't these bounds important?
                                    Self::Not(_)
                                    | Self::Shape(_)
                                    | Self::Supports { .. }
                                    | Self::Satisfies { .. } => {}
                                }
                            }
                        }
                        Some(base)
                    }
                }

                Combinator::Any | Combinator::One => {
                    bounds.iter().find_map(|bound| bound.foobar_example(names))
                }
            },

            Self::Exactly(t) => Some(ExampleType::Abstract(Cow::Borrowed(t))),

            Self::Has { member, req } => Some(ExampleType::Rec(ExampleRec {
                name: names.next().expect("ran out of silly names"),
                members: BTreeMap::from_iter([(
                    *member,
                    req.as_deref().map(|req| {
                        req.foobar_example(names)
                            .unwrap_or(ExampleType::Bound(Cow::Borrowed(req)))
                    }),
                )]),
            })),

            Self::Not(_) | Self::Shape(_) | Self::Supports { .. } | Self::Satisfies { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckedType<'src> {
    Abstract(AbstractType<'src>),
    Bound(TypeBound<'src>),
}

impl<'src> CheckedType<'src> {
    /// If an example of a type satisfying the bound exists, presents it.
    pub const fn trivial_example(&self) -> Option<&AbstractType<'src>> {
        match self {
            Self::Abstract(t) => Some(t),
            Self::Bound(b) => b.trivial_example(),
        }
    }

    /// Produces an example involving a made-up Foo/Bar type specifically constructed to fulfill the bound, if possible
    pub fn foobar_example<'nam, 'ty, I>(
        &'ty self,
        names: &mut I,
    ) -> Option<ExampleType<'nam, 'ty, 'src>>
    where
        I: 'nam + Iterator<Item = Cow<'nam, str>>,
    {
        match self {
            Self::Abstract(t) => Some(ExampleType::Abstract(Cow::Borrowed(t))),
            Self::Bound(b) => b.foobar_example(names),
        }
    }
}

impl Display for CheckedType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abstract(t) => t.fmt(f),
            Self::Bound(b) => b.fmt(f),
        }
    }
}

trait Reason {
    type AliasesNeeded<'a>: IntoIterator<Item = &'a TypeBound<'a>>
    where
        Self: 'a;

    /// If `self` represents a [`TypeBound::Combo`], returns an iterator over specifically the bounds used in reasoning
    fn aliases_needed(&self) -> Self::AliasesNeeded<'_>;

    /// Indicates that the reason uses the same bound as the outer check (e.g. adapting [`DeniedBound`] -> [`UnsatisfiedBound`])
    fn is_adapter(&self) -> bool {
        false
    }

    /// Writes the adapter requirement. This method is allowed to panic if [`Self::is_adapter`] would return false.
    fn adapter_requirement(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        #![expect(unused_variables, reason = "used in implementors")]
        Ok(())
    }

    /// Comes after the [`TypeBound`] requirement but before [`Self::write_source`].
    /// The start of each line should be indented.
    // WARNING: need to carefully review which cases should use `bound` and which should use `attempt`!
    fn nested(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result;

    fn has_consideration(&self) -> bool;

    /// Describes a hypothetical scenario which would fail to satisfy the bound while satisfying the attempt.
    ///
    /// Should be written so that "consider: the type `{attempt_var} | {attempt}` might `{consideration}`." makes sense.
    /// This method is allowed to panic if [`has_consideration`] would have been false.
    fn consideration(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result;

    /// Should be written so that "since the `{attempt}` `{write_source}`, ..." makes sense.
    fn since(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result;

    /// Should be written so that "the `{attempt}` `{write_effect}` the bound `{bound}`." makes sense.
    fn effect(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundCheck<'src, T> {
    // TODO: having EACH NODE deep-clone the ENTIRE bound below that point could get really expensive
    pub bound: TypeBound<'src>,
    pub attempt: CheckedType<'src>,
    pub reason: T,
}

/// Do not default to this, use [`BoundCheck::fmt_check`] if possible to give the most info
impl<T: Reason> Display for BoundCheck<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_check(None, f)
    }
}

impl<T: Reason> BoundCheck<'_, T> {
    fn fmt_check(
        &self,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let Self {
            bound,
            attempt,
            reason,
        } = self;
        Indent.fmt(f)?;
        // write!(f, "{}: ", reason.name())?; // DEBUG
        if reason.is_adapter() {
            reason.adapter_requirement(bound, attempt, attempt_var, f)?;
        } else {
            match attempt {
                CheckedType::Abstract(_) => {
                    if bound.let_statements(reason, f)? {
                        writeln!(f, ".")?;
                        Indent.fmt(f)?;
                    }
                    bound.requirement(f)?;
                }

                CheckedType::Bound(attempt_bound) => {
                    if attempt_bound.let_statements(reason, f)? {
                        writeln!(f, ".")?;
                        Indent.fmt(f)?;
                    }
                    attempt_bound.requirement(f)?;
                }
            }
        }
        writeln!(f, ".")?;
        reason.nested(bound, attempt, attempt_var, f)?;
        if reason.has_consideration() {
            Indent.fmt(f)?;
            match attempt {
                CheckedType::Abstract(t) => {
                    write!(f, "consider: the type `{:#}` might ", Aliased::new(t))?;
                    reason.consideration(bound, attempt, None, f)?;
                }
                CheckedType::Bound(b) => {
                    let t = let_local(b, f)?;
                    writeln!(f, ".")?;
                    Indent.fmt(f)?;
                    write!(f, "consider: the type `{t:#}` might ")?;
                    reason.consideration(bound, attempt, Some(t), f)?;
                }
            }
            writeln!(f)?;
        }
        Indent.fmt(f)?;
        write!(
            f,
            "since the {} `{:#}` ",
            attempt.kind_name(),
            Aliased::new(attempt)
        )?;
        reason.since(bound, attempt, attempt_var, f)?;
        match attempt {
            CheckedType::Abstract(t) => {
                write!(f, ", the type `{:#}` ", Aliased::new(t))?;
            }

            CheckedType::Bound(b) => {
                f.write_str(", ")?;
                if let Some(t) = attempt_var {
                    write!(f, "any type `{t:#}` defined by ")?;
                }
                write!(f, "the bound `{:#}` ", Aliased::new(b))?;
            }
        }
        reason.effect(f)?;
        write!(f, " the bound `{:#}`.", Aliased::new(bound))
    }
}

pub type SatisfiedBound<'src> = BoundCheck<'src, SatisfiedBoundReason<'src>>;
pub type UnsatisfiedBound<'src> = BoundCheck<'src, UnsatisfiedBoundReason<'src>>;

pub type RequiredBound<'src> = BoundCheck<'src, RequiredBoundReason<'src>>;
pub type AllowedBound<'src> = BoundCheck<'src, AllowedBoundReason<'src>>;
pub type DeniedBound<'src> = BoundCheck<'src, DeniedBoundReason<'src>>;

/// The bound is necessarily satisfied
#[derive(Debug, Clone, PartialEq)]
pub enum RequiredBoundReason<'src> {
    /// A [`TypeBound::Combo`] with [`Combinator::All`] satisfies the bound through one of its requirements
    OneOfAll(Box<RequiredBound<'src>>),
    /// A [`TypeBound::Combo`] with [`Combinator::Any`] satisfies the bound through all of its requirements
    AllOfAny,
    /// A [`TypeBound::Combo`] with [`Combinator::One`] satisfies the bound through all of its requirements
    AllOfOne,
    /// A [`TypeBound::Exactly`] satisfies the bound by requiring a type that satisfies it
    Exactly(Box<SatisfiedBound<'src>>),
    /// A [`TypeBound::Has`] satisfies the bound by requiring the same member
    Has,
    /// A [`TypeBound::Has`] satisfies the bound by requiring the same member with satisfying requirements
    HasMatching(Box<RequiredBound<'src>>),
    /// A [`TypeBound::Shape`] satisfies the bound by requiring the same shape
    Shape,
    /// A [`TypeBound::Satisfies`] satisfies the bound by requiring a subset or equivalent to the requirements
    Expression,
}

impl Reason for RequiredBoundReason<'_> {
    type AliasesNeeded<'a>
        = Option<&'a TypeBound<'a>>
    where
        Self: 'a;

    fn aliases_needed(&self) -> Self::AliasesNeeded<'_> {
        match self {
            Self::OneOfAll(BoundCheck { attempt, .. }) => {
                if let CheckedType::Bound(attempt) = attempt {
                    Some(attempt)
                } else {
                    unimplemented!()
                }
            }

            Self::AllOfAny
            | Self::AllOfOne
            | Self::Exactly(_)
            | Self::Has
            | Self::HasMatching(_)
            | Self::Shape
            | Self::Expression => None,
        }
    }

    fn nested(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::OneOfAll(req) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`.",
                    Aliased::new(&req.attempt)
                )?;
                {
                    let _depth = LocalDepth::new();
                    req.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::Exactly(req) => {
                todo!("this is a default");
                {
                    let _depth = LocalDepth::new();
                    req.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::HasMatching(req) => {
                todo!("this is a default");
                {
                    let _depth = LocalDepth::new();
                    req.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::AllOfAny | Self::AllOfOne | Self::Has | Self::Shape | Self::Expression => Ok(()),
        }
    }

    fn has_consideration(&self) -> bool {
        false
    }

    fn consideration(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        _f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        unimplemented!("requiredness has no hypotheticals")
    }

    fn since(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::OneOfAll(BoundCheck {
                attempt: fulfilled, ..
            }) => {
                write!(
                    f,
                    "must satisfy every one of its bounds, one of which is `{fulfilled:#}` which satisfies the bound"
                )
            }

            Self::AllOfAny | Self::AllOfOne => {
                write!(
                    f,
                    "requires the bound in every branch, leaving no case in which it can be unsatisfied"
                )
            }

            Self::Exactly(BoundCheck { attempt: _req, .. }) => {
                write!(
                    f,
                    "must be a specific type which verifiably satisfies the bound"
                )
            }

            Self::Has => {
                write!(f, "necessarily requires a member by the same name")
            }

            Self::HasMatching(BoundCheck { attempt: req, .. }) => {
                write!(
                    f,
                    "necessarily requires a member by the same name, and requires that member to satisfy `{req:#}`"
                )
            }

            Self::Shape => {
                write!(f, "necessarily requires the described shape")
            }

            Self::Expression => {
                todo!()
            }
        }
    }

    fn effect(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("by definition requires")
    }
}

/// The bound may be satisfied by the final type, but the bounds do not enforce it, so it may also be unsatisfied
#[derive(Debug, Clone, PartialEq)]
pub enum AllowedBoundReason<'src> {
    /// A [`TypeBound::Combo`] with [`Combinator::All`] neither requires nor denies the bound through any bound
    NoneOfAll,
    /// A bounded type does not deny but contains at least one bound which might not satisfy the [`Combinator::All`]
    IncompleteAll(Box<AllowedBound<'src>>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::Any`] neither requires nor denies the bound
    AnyBestAllowed(Box<AllowedBound<'src>>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::One`] neither requires nor denies the bound
    OneBestAllowed(Box<AllowedBound<'src>>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::One`] requires the bound, but another allows it
    OnePassingMultipleAllowed(Box<(SatisfiedBound<'src>, AllowedBound<'src>)>),
    /// Multiple bounds in a [`TypeBound::Combo`] with [`Combinator::One`] allow the bound
    OneAllowsMultiple(Box<(AllowedBound<'src>, AllowedBound<'src>)>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::Any`] only allows the bound, this might be the one matched
    OneOfAny(Box<AllowedBound<'src>>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::One`] only allows the bound, this might be the one matched
    OneOfOne(Box<AllowedBound<'src>>),
    /// A [`TypeBound::Not`] requires that a bound is not denied, but doesn't require it
    NotDenied(Box<DeniedBound<'src>>),
    /// A [`TypeBound::Not`] neither requires nor denies the bound through any bound
    IneffectiveNot(Box<AllowedBound<'src>>),
    /// A [`TypeBound::Not`] requires that a bound is not required, but doesn't deny it
    NotRequired(Box<RequiredBound<'src>>),
    /// A [`TypeBound::Exactly`] denies the bound by requiring a bounded type that neither requires nor denies it
    ExactlyAllowed(Box<AllowedBound<'src>>),
    /// A [`TypeBound`] places no bounds on whether its type [`TypeBound::Has`] the member
    MayHave,
    /// A [`TypeBound::Has`] requires a different member, making no comment on the required member
    MayAlsoHave,
    /// A [`TypeBound::Has`] requires the member, but doesn't place any requirements on it
    HasUndenied,
    /// A [`TypeBound::Has`] requires the member, but doesn't place the same requirements on it
    HasAllowed(Box<AllowedBound<'src>>),
    /// A [`TypeBound`] places no bounds on whether its type has the [`TypeBound::Shape`]
    ShapeUndenied,
    /// A [`TypeBound`] does not restrict the expression, but it could be satisfied at runtime
    RuntimeExpression,
}

impl Reason for AllowedBoundReason<'_> {
    type AliasesNeeded<'a>
        = Option<&'a TypeBound<'a>>
    where
        Self: 'a;

    fn aliases_needed(&self) -> Self::AliasesNeeded<'_> {
        match self {
            Self::IncompleteAll(BoundCheck { attempt, .. })
            | Self::AnyBestAllowed(BoundCheck { attempt, .. })
            | Self::OneBestAllowed(BoundCheck { attempt, .. })
            | Self::OneOfAny(BoundCheck { attempt, .. })
            | Self::OneOfOne(BoundCheck { attempt, .. }) => {
                if let CheckedType::Bound(attempt) = attempt {
                    Some(attempt)
                } else {
                    unimplemented!();
                }
            }

            Self::NoneOfAll
            | Self::OnePassingMultipleAllowed(_)
            | Self::OneAllowsMultiple(_)
            | Self::NotDenied(_)
            | Self::IneffectiveNot(_)
            | Self::NotRequired(_)
            | Self::ExactlyAllowed(_)
            | Self::MayHave
            | Self::MayAlsoHave
            | Self::HasUndenied
            | Self::HasAllowed(_)
            | Self::ShapeUndenied
            | Self::RuntimeExpression => None,
        }
    }

    fn nested(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::IncompleteAll(insufficient)
            | Self::AnyBestAllowed(insufficient)
            | Self::OneBestAllowed(insufficient)
            | Self::OneOfAny(insufficient) // TBD: how are these different?
            | Self::OneOfOne(insufficient) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`.",
                    Aliased::new(&insufficient.attempt)
                )?;
                {
                    let _depth = LocalDepth::new();
                    insufficient.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::OnePassingMultipleAllowed((best, insufficient)) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`:",
                    Aliased::new(&best.attempt)
                )?;
                {
                    let _depth = LocalDepth::new();
                    best.fmt_check(attempt_var, f)?;
                }
                writeln!(f)?;
                Indent.fmt(f)?;
                writeln!(f, "another is `{:#}`:", Aliased::new(&insufficient.attempt))?;
                {
                    let _depth = LocalDepth::new();
                    insufficient.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::OneAllowsMultiple((first, second)) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`:",
                    Aliased::new(&first.attempt)
                )?;
                {
                    let _depth = LocalDepth::new();
                    first.fmt_check(attempt_var, f)?;
                }
                writeln!(f)?;
                Indent.fmt(f)?;
                writeln!(f, "another is `{:#}`:", Aliased::new(&second.attempt))?;
                {
                    let _depth = LocalDepth::new();
                    second.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::NotDenied(insufficient) => {
                {
                    let _depth = LocalDepth::new();
                    insufficient.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::IneffectiveNot(insufficient) => {
                {
                    let _depth = LocalDepth::new();
                    insufficient.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::NotRequired(required) => {
                {
                    let _depth = LocalDepth::new();
                    required.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::ExactlyAllowed(insufficient) => {
                todo!();
                {
                    let _depth = LocalDepth::new();
                    insufficient.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::HasAllowed(insufficient) => {
                todo!();
                {
                    let _depth = LocalDepth::new();
                    insufficient.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::NoneOfAll
            | Self::MayHave
            | Self::MayAlsoHave
            | Self::HasUndenied
            | Self::ShapeUndenied
            | Self::RuntimeExpression => Ok(()),
        }
    }

    fn has_consideration(&self) -> bool {
        matches!(self, Self::MayHave | Self::MayAlsoHave)
    }

    fn consideration(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        // after this assertion, any failures are *this* function's fault, not the caller's.
        debug_assert!(self.has_consideration(), "has no consideration");
        match self {
            Self::MayHave => {
                // let generator = NameGenerator::new(Vec::with_capacity(128));
                let example = attempt
                    .foobar_example(&mut silly_names())
                    .expect("`has` should produce an example"); // TODO: does `attempt` guarantee `TypeBound::Has`?
                write!(
                    f,
                    "be `{example:#}`, which would fulfill `{:#}` but not `{:#}`",
                    Aliased::new(attempt),
                    Aliased::new(bound)
                )?;
                Ok(())
            }

            Self::MayAlsoHave => todo!(),

            _ => Ok(()),
        }
    }

    // TODO: needs review
    // TODO: would be nice to get access to a `MetaType`
    fn since(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::NoneOfAll => {
                write!(
                    f,
                    "neither requires nor denies `{:#}` through any of its bounds",
                    Aliased::new(bound)
                )
            }

            Self::IncompleteAll(BoundCheck { bound, .. }) => todo!(),

            Self::AnyBestAllowed(BoundCheck { bound, .. }) => todo!(),

            Self::OneBestAllowed(BoundCheck { bound, .. }) => todo!(),

            Self::OnePassingMultipleAllowed((
                BoundCheck { bound: best, .. },
                BoundCheck {
                    bound: insufficient,
                    ..
                },
            )) => write!(
                f,
                "would ideally only be `{:#}`, but is compatible with `{:#}`, it might satisfy both; failing exclusivity. thus",
                Aliased::new(best),
                Aliased::new(insufficient)
            ),

            Self::OneAllowsMultiple((
                BoundCheck { bound: first, .. },
                BoundCheck { bound: second, .. },
            )) => write!(
                f,
                "is compatible with both `{:#}` and `{:#}`, it might satisfy both; failing exclusivity. thus",
                Aliased::new(first),
                Aliased::new(second)
            ),

            Self::OneOfOne(BoundCheck { bound, .. }) | Self::OneOfAny(BoundCheck { bound, .. }) => {
                write!(f, "might only satisfy `{:#}`", Aliased::new(bound))
            }

            Self::NotDenied(BoundCheck { bound, .. }) => {
                write!(
                    f,
                    "negates the rejection of `{:#}`, it has the possibility to fulfill the bound. \
                    however, this does not guarantee it, therefore",
                    Aliased::new(bound)
                )
            }

            Self::IneffectiveNot(BoundCheck { bound, .. }) => todo!(),

            Self::NotRequired(BoundCheck { bound, .. }) => {
                write!(
                    f,
                    "negates the requirement of `{:#}`, it cannot be guaranteed. thus",
                    Aliased::new(bound)
                )
            }

            Self::ExactlyAllowed(BoundCheck { bound, .. }) => todo!(),

            Self::MayHave => {
                let CheckedType::Bound(TypeBound::Has { member, req: _ }) = attempt else {
                    unimplemented!()
                };
                write!(
                    f,
                    "neither requires nor denies a type from having the member `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member),
                )
            }

            Self::MayAlsoHave => todo!(),

            Self::HasUndenied => todo!(),

            Self::HasAllowed(BoundCheck { bound, .. }) => todo!(),

            Self::ShapeUndenied => {
                // let TypeBound::Shape(expected) = bound else {
                //     unimplemented!("expected a shape bound, found {bound:#}");
                // };
                write!(
                    f,
                    "neither requires nor denies a type from being called with the required shape",
                )
            }

            Self::RuntimeExpression => todo!(),
        }
    }

    fn effect(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("neither requires nor denies")
    }
}

/// The bound is necessarily unsatisfied
#[derive(Debug, Clone, PartialEq)]
pub enum DeniedBoundReason<'src> {
    /// A [`TypeBound::Combo`] with [`Combinator::All`] denies the bound through one of its requirements
    OneOfAll(Box<DeniedBound<'src>>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::Any`] denies the bound
    OneOfAny(Box<DeniedBound<'src>>),
    /// One of the bounds in a [`TypeBound::Combo`] with [`Combinator::One`] denies the bound
    OneOfOne(Box<DeniedBound<'src>>),
    /// A [`TypeBound::Exactly`] denies the bound by requiring a type that does not satisfy it
    ExactlyUnsatisfying(Box<UnsatisfiedBound<'src>>),
    /// A [`TypeBound::Not`] denies the member
    CantHave(Box<DeniedBound<'src>>),
    /// A [`TypeBound::Has`] requires the member, but denies the bounds placed on it
    HasConflicting(Box<DeniedBound<'src>>),
    /// A [`TypeBound::Satisfies`] contradicts the bound
    ExprContradiction(Todo),
}

impl Reason for DeniedBoundReason<'_> {
    type AliasesNeeded<'a>
        = Option<&'a TypeBound<'a>>
    where
        Self: 'a;

    fn aliases_needed(&self) -> Self::AliasesNeeded<'_> {
        match self {
            Self::OneOfAll(BoundCheck { attempt, .. })
            | Self::OneOfAny(BoundCheck { attempt, .. })
            | Self::OneOfOne(BoundCheck { attempt, .. }) => {
                if let CheckedType::Bound(attempt) = attempt {
                    Some(attempt)
                } else {
                    unimplemented!("expected a bound, found {attempt:#}")
                }
            }

            Self::ExactlyUnsatisfying(_)
            | Self::CantHave(_)
            | Self::HasConflicting(_)
            | Self::ExprContradiction(_) => None,
        }
    }

    fn nested(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::OneOfAll(denial) | Self::OneOfAny(denial) | Self::OneOfOne(denial) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`:",
                    Aliased::new(&denial.attempt)
                )?;
                {
                    let _depth = LocalDepth::new();
                    denial.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::ExactlyUnsatisfying(unsatisfied) => {
                {
                    let _depth = LocalDepth::new();
                    unsatisfied.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::CantHave(denial) => {
                todo!();
                {
                    let _depth = LocalDepth::new();
                    denial.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::HasConflicting(denial) => {
                todo!();
                {
                    let _depth = LocalDepth::new();
                    denial.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::ExprContradiction(_) => todo!(),
        }
    }

    fn has_consideration(&self) -> bool {
        matches!(self, Self::OneOfAny(_) | Self::OneOfOne(_))
    }

    fn consideration(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        // after this assertion, any failures are *this* function's fault, not the caller's.
        debug_assert!(self.has_consideration(), "has no consideration");
        match self {
            Self::OneOfAny(BoundCheck {
                attempt: denied, ..
            })
            | Self::OneOfOne(BoundCheck {
                attempt: denied, ..
            }) => {
                // non-aliasing here is intentional
                match denied
                    .foobar_example(&mut silly_names())
                    .unwrap_or(ExampleType::Checked(Cow::Borrowed(denied)))
                    .as_ref()
                {
                    ExampleTypeRef::Abstract(t) => write!(f, "be the explicit type `{t:#}`"),
                    ExampleTypeRef::Bound(b) => write!(f, "exclusively satisfy the bound `{b:#}`"),
                    ExampleTypeRef::Rec(t) => write!(f, "be some type `{t:#}`"),
                }?;
                write!(
                    f,
                    ": this would fulfill the bound `{:#}`, but not the bound `{:#}`",
                    Aliased::new(attempt),
                    Aliased::new(bound),
                )
            }

            _ => todo!(),
        }
    }

    fn since(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::OneOfAll(BoundCheck {
                attempt: denied, ..
            }) => {
                write!(f, "necessarily rejects `{:#}`", Aliased::new(denied))
            }

            Self::OneOfAny(BoundCheck {
                attempt: denied, ..
            }) => {
                write!(f, "can be satisfied by `{:#}`", Aliased::new(denied))
            }

            Self::OneOfOne(BoundCheck {
                attempt: denied, ..
            }) => {
                write!(f, "could be specifically `{:#}`", Aliased::new(denied))
            }

            Self::ExactlyUnsatisfying(BoundCheck {
                bound: unsatisfied, ..
            }) => {
                write!(
                    f,
                    "can only be satisfied by a type which does not satisfy `{:#}`",
                    Aliased::new(unsatisfied)
                )
            }

            // TODO: where is this constructed?
            Self::CantHave(BoundCheck {
                attempt: denied, ..
            }) => {
                todo!()
            }

            // TODO: where is this constructed?
            Self::HasConflicting(BoundCheck {
                attempt: denied, ..
            }) => {
                todo!()
            }

            Self::ExprContradiction(_) => todo!(),
        }
    }

    fn effect(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cannot, by definition, satisfy")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SatisfiedBoundReason<'src> {
    /// Satisfied a [`TypeBound`] through a bound that requires it.
    Required(RequiredBound<'src>),
    /// Satisfied [`TypeBound::Combo`] with [`Combinator::All`]. Every bound succeeded.
    All,
    /// Satisfied [`TypeBound::Combo`] with [`Combinator::Any`]. `fulfilled` succeeded.
    Any(Box<SatisfiedBound<'src>>),
    /// Satisfied [`TypeBound::Combo`] with [`Combinator::One`]. `fulfilled` succeeded.
    One(Box<SatisfiedBound<'src>>),
    /// Satisfied [`TypeBound::Not`]. `unfulfilled` failed as expected.
    Not(Box<UnsatisfiedBound<'src>>),
    /// Satisfied [`TypeBound::Exactly`]. `attempt` is the exact type.
    Exactly,
    /// Satisfied [`TypeBound::Has`]. `attempt` has the member.
    Has,
    /// Satisfied [`TypeBound::Has`] with a bound on the member. `attempt` has the member.
    HasMatching(Box<SatisfiedBound<'src>>),
    /// Satisfied [`TypeBound::Shape`]. `attempt` has the shape.
    Shape,
    /// Satisfied [`TypeBound::Supports`].
    Supports(Todo),
    /// Satisfied [`TypeBound::Satisfies`] with an expression which can never be false.
    Tautology,
    /// Satisfied [`TypeBound::Satisfies`].
    Satisfies,
}

impl<'src> From<RequiredBound<'src>> for SatisfiedBound<'src> {
    fn from(value: RequiredBound<'src>) -> Self {
        // TBD: should the inner and outer bound/attempt be the same?
        Self {
            bound: value.bound.clone(),
            attempt: value.attempt.clone(),
            reason: SatisfiedBoundReason::Required(value),
        }
    }
}

impl Reason for SatisfiedBoundReason<'_> {
    type AliasesNeeded<'a>
        = Option<&'a TypeBound<'a>>
    where
        Self: 'a;

    fn aliases_needed(&self) -> Self::AliasesNeeded<'_> {
        match self {
            Self::Any(BoundCheck { bound, .. }) | Self::One(BoundCheck { bound, .. }) => {
                Some(bound)
            }

            Self::Required(_)
            | Self::All
            | Self::Not(_)
            | Self::Exactly
            | Self::Has
            | Self::HasMatching(_)
            | Self::Shape
            | Self::Supports(_)
            | Self::Tautology
            | Self::Satisfies { .. } => None,
        }
    }

    fn is_adapter(&self) -> bool {
        matches!(self, Self::Required(_))
    }

    fn adapter_requirement(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Required(_) => {
                #[cfg(debug_assertions)]
                assert_matches!(
                    attempt,
                    CheckedType::Bound(_),
                    "only bounds should be used in Required"
                );
                let t = attempt_var.expect("attempt_var should be Some if attempt is a Bound");
                write!(
                    f,
                    "if the bound `{:#}` requires the bound `{:#}` to be fulfilled by definition, \
                    then `{t:#}` is guaranteed to fulfill `{:#}`",
                    Aliased::new(attempt),
                    Aliased::new(bound),
                    Aliased::new(bound),
                )
            }

            _ => unimplemented!("should guard behind is_adapter"),
        }
    }

    fn nested(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Required(requirement) => {
                {
                    let _depth = LocalDepth::new();
                    requirement.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::Any(fulfilled) | Self::One(fulfilled) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`:",
                    Aliased::new(&fulfilled.bound)
                )?;
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::Not(unfulfilled) => {
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::HasMatching(fulfilled) => {
                let TypeBound::Has { member, req: _ } = fulfilled.bound else {
                    unimplemented!()
                };
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "a member `{:#}` is present:",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )?;
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::All
            | Self::Exactly
            | Self::Has
            | Self::Shape
            | Self::Supports(_)
            | Self::Tautology
            | Self::Satisfies { .. } => Ok(()),
        }
    }

    fn has_consideration(&self) -> bool {
        false
    }

    fn consideration(
        &self,
        _bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        _f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        unimplemented!("satisfaction has no hypotheticals")
    }

    fn since(
        &self,
        bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Required(BoundCheck {
                bound: requirement, ..
            }) => write!(
                f,
                "requires the bound `{:#}` by definition",
                Aliased::new(requirement)
            ),

            Self::All => f.write_str("fulfills all of these"),

            Self::Any(BoundCheck {
                bound: fulfilled, ..
            }) => write!(f, "fulfills the bound `{:#}`", Aliased::new(fulfilled)),

            Self::One(BoundCheck {
                bound: fulfilled, ..
            }) => write!(
                f,
                "exclusively fulfills the bound `{:#}`",
                Aliased::new(fulfilled)
            ),

            Self::Not(BoundCheck { bound: inner, .. }) => {
                write!(
                    f,
                    "does not fulfill the bound `{:#}`, which is being negated",
                    Aliased::new(inner)
                )
            }

            Self::Exactly => write!(f, "is equivalent to the bound `{:#}`", Aliased::new(bound)),

            Self::Has => {
                let TypeBound::Has { member, req: None } = bound else {
                    unimplemented!()
                };
                write!(
                    f,
                    "has a member, `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )
            }

            Self::HasMatching(BoundCheck { bound: inner, .. }) => {
                let TypeBound::Has {
                    member,
                    req: Some(_),
                } = bound
                else {
                    unimplemented!()
                };
                write!(
                    f,
                    "has a member, `{:#}`, satisfying the bound `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member),
                    Aliased::new(inner)
                )
            }

            Self::Shape => f.write_str("can be called as such"),

            Self::Supports(_) => todo!(),

            Self::Tautology => todo!(),

            Self::Satisfies { .. } => todo!(),
        }
    }

    fn effect(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("satisfies")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnsatisfiedBoundReason<'src> {
    /// Failed a [`TypeBound`] through a bound that denies it
    Denied(DeniedBound<'src>),
    /// Failed a [`TypeBound`] through a bound that neither requires nor denies it
    Unrequired(AllowedBound<'src>),
    /// Failed [`TypeBound::Combo`] with [`Combinator::All`]. `unfulfilled` failed.
    NotAll(Box<UnsatisfiedBound<'src>>),
    /// Failed [`TypeBound::Combo`] with [`Combinator::Any`]. Every bound failed.
    NotAny,
    /// Failed [`TypeBound::Combo`] with [`Combinator::One`]. Every bound failed.
    NotOne,
    /// Failed [`TypeBound::Combo`] with [`Combinator::One`]. Both `fulfilled.0` and `fulfilled.1` succeeded.
    OverOne(Box<(SatisfiedBound<'src>, SatisfiedBound<'src>)>),
    /// Failed [`TypeBound::Not`]. `fulfilled` unexpectedly satisfied.
    NotNot(Box<SatisfiedBound<'src>>),
    /// Failed [`TypeBound::Exactly`]. `bound` and `attempt` conflict.
    Unequal,
    /// Failed [`TypeBound::Has`]. `attempt` does not have the member `bound` requires.
    MissingMember,
    /// Failed [`TypeBound::Has`]. `attempt` has the member `bound` requires, but the member fails `unfulfilled`.
    MemberFails(Box<UnsatisfiedBound<'src>>),
    /// Failed [`TypeBound::Shape`]. `attempt` does not have any shape.
    MissingShape,
    /// Failed [`TypeBound::Shape`]. `attempt` does not have the shape described by `bound`. field is the actual shape.
    ShapeFails(FunShape<'src>),
    /// Failed [`TypeBound::Supports`].
    Unsupported(Todo),
    /// Failed [`TypeBound::Satisfies`] because the expression can never be true.
    SelfContradiction(Todo),
    /// Failed [`TypeBound::Satisfies`].
    Contradiction(Todo),
}

impl<'src> From<DeniedBound<'src>> for UnsatisfiedBound<'src> {
    fn from(value: DeniedBound<'src>) -> Self {
        // TBD: should the inner and outer bound/attempt be the same?
        Self {
            bound: value.bound.clone(),
            attempt: value.attempt.clone(),
            reason: UnsatisfiedBoundReason::Denied(value),
        }
    }
}

impl<'src> From<AllowedBound<'src>> for UnsatisfiedBound<'src> {
    fn from(value: AllowedBound<'src>) -> Self {
        // TBD: should the inner and outer bound/attempt be the same?
        Self {
            bound: value.bound.clone(),
            attempt: value.attempt.clone(),
            reason: UnsatisfiedBoundReason::Unrequired(value),
        }
    }
}

impl<'src> TryFrom<super::TypeBoundRequiredness<'src>> for UnsatisfiedBound<'src> {
    /// Unexpectedly success
    type Error = RequiredBound<'src>;

    fn try_from(value: super::TypeBoundRequiredness<'src>) -> Result<Self, Self::Error> {
        match value {
            super::Requiredness::Require(x) => Err(x),
            super::Requiredness::Allow(x) => Ok(Self::from(x)),
            super::Requiredness::Deny(x) => Ok(Self::from(x)),
        }
    }
}

impl Reason for UnsatisfiedBoundReason<'_> {
    type AliasesNeeded<'a>
        = std::iter::Chain<
        std::option::IntoIter<&'a TypeBound<'a>>,
        std::option::IntoIter<&'a TypeBound<'a>>,
    >
    where
        Self: 'a;

    fn aliases_needed(&self) -> Self::AliasesNeeded<'_> {
        match self {
            Self::NotAll(BoundCheck { bound, .. }) => std::iter::chain(Some(bound), None),

            Self::OverOne((BoundCheck { bound: first, .. }, BoundCheck { bound: second, .. })) => {
                std::iter::chain(Some(first), Some(second))
            }

            Self::Denied(_)
            | Self::Unrequired(_)
            | Self::NotAny
            | Self::NotOne
            | Self::NotNot(_)
            | Self::Unequal
            | Self::MissingMember
            | Self::MemberFails(_)
            | Self::MissingShape
            | Self::ShapeFails(_)
            | Self::Unsupported(_)
            | Self::SelfContradiction(_)
            | Self::Contradiction(_) => std::iter::chain(None, None),
        }
    }

    fn is_adapter(&self) -> bool {
        matches!(self, Self::Denied(_) | Self::Unrequired(_))
    }

    fn adapter_requirement(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Denied(_) => {
                #[cfg(debug_assertions)]
                assert_matches!(
                    attempt,
                    CheckedType::Bound(_),
                    "only bounds should be used in Denied"
                );
                let t = attempt_var.expect("attempt_var should be Some if attempt is a Bound");
                write!(
                    f,
                    "for `{t:#}` to fulfill the bound `{:#}`, the definition of `{:#}` cannot contradict `{:#}`",
                    Aliased::new(bound),
                    Aliased::new(attempt),
                    Aliased::new(bound),
                )
            }

            Self::Unrequired(_) => {
                #[cfg(debug_assertions)]
                assert_matches!(
                    attempt,
                    CheckedType::Bound(_),
                    "only bounds should be used in Unrequired"
                );
                let t = attempt_var.expect("attempt_var should be Some if attempt is a Bound");
                write!(
                    f,
                    "if the bound `{:#}` does not require the bound `{:#}` by definition, \
                    then `{t:#}` is not guaranteed to fulfill `{:#}`",
                    Aliased::new(attempt),
                    Aliased::new(bound),
                    Aliased::new(bound),
                )
            }

            _ => unimplemented!("should guard behind is_adapter"),
        }
    }

    fn nested(
        &self,
        bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Denied(denial) => {
                {
                    let _depth = LocalDepth::new();
                    denial.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::Unrequired(denial) => {
                {
                    let _depth = LocalDepth::new();
                    denial.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::NotAll(unfulfilled) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`:",
                    Aliased::new(&unfulfilled.bound)
                )?;
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::OverOne((first, second)) => {
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "one of these bounds is `{:#}`:",
                    Aliased::new(&first.bound)
                )?;
                {
                    let _depth = LocalDepth::new();
                    first.fmt_check(attempt_var, f)?;
                }
                writeln!(f)?;
                Indent.fmt(f)?;
                writeln!(f, "another is `{:#}`:", Aliased::new(&second.bound))?;
                {
                    let _depth = LocalDepth::new();
                    second.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::NotNot(fulfilled) => {
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::MemberFails(unfulfilled) => {
                let TypeBound::Has { member, .. } = bound else {
                    unimplemented!()
                };
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "a member `{:#}` exists:",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member),
                )?;
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt_check(attempt_var, f)?;
                }
                writeln!(f)
            }

            Self::NotAny
            | Self::NotOne
            | Self::Unequal
            | Self::MissingMember
            | Self::MissingShape
            | Self::ShapeFails { .. }
            | Self::Unsupported(_)
            | Self::SelfContradiction(_)
            | Self::Contradiction(_) => Ok(()),
        }
    }

    fn has_consideration(&self) -> bool {
        false
    }

    fn consideration(
        &self,
        bound: &TypeBound<'_>,
        attempt: &CheckedType<'_>,
        attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        // after this assertion, any failures are *this* function's fault, not the caller's.
        debug_assert!(self.has_consideration(), "has no consideration");
        // TODO
        Ok(())
    }

    fn since(
        &self,
        bound: &TypeBound<'_>,
        _attempt: &CheckedType<'_>,
        _attempt_var: Option<MetaType>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Denied(BoundCheck { bound: failed, .. }) => {
                write!(f, "contradicts `{:#}` by definition", Aliased::new(failed))
            }

            Self::Unrequired(BoundCheck {
                bound: unrequired, ..
            }) => {
                write!(f, "does not guarantee `{:#}`", Aliased::new(unrequired))
            }

            Self::NotAll(BoundCheck { bound: failed, .. }) => {
                write!(f, "does not fulfill `{:#}`", Aliased::new(failed))
            }

            Self::NotAny | Self::NotOne => write!(f, "fulfills none of the bounds"),

            Self::OverOne((BoundCheck { bound: first, .. }, BoundCheck { bound: second, .. })) => {
                write!(
                    f,
                    "fulfills both `{:#}` and `{:#}`, thus failing exclusivity",
                    Aliased::new(first),
                    Aliased::new(second)
                )
            }

            Self::NotNot(BoundCheck { bound: inner, .. }) => {
                write!(
                    f,
                    "fulfills `{:#}`, which is being negated",
                    Aliased::new(inner)
                )
            }

            Self::Unequal => write!(f, "is inequivalent to `{:#}`", Aliased::new(bound)),

            Self::MissingMember => {
                let TypeBound::Has { member, .. } = bound else {
                    unimplemented!()
                };
                write!(
                    f,
                    "has no member `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )
            }

            Self::MemberFails(_) => {
                let TypeBound::Has {
                    member,
                    req: Some(req),
                } = bound
                else {
                    unimplemented!()
                };
                write!(
                    f,
                    "has the member `{0:#}`, but `{0:#}` does not fulfill `{1:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member),
                    Aliased::new(req.as_ref())
                )
            }

            Self::MissingShape => f.write_str("cannot be called"),

            Self::ShapeFails(actual) => write!(f, "is instead called as `{actual:#}`"),

            Self::Unsupported(_) => todo!(),

            Self::SelfContradiction(_) => todo!(),

            Self::Contradiction(_) => todo!(),
        }
    }

    fn effect(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("does not satisfy")
    }
}

// TBD: what else would be put in here?
#[derive(Debug, Clone, PartialEq)]
pub struct UnsatisfiedBoundError<'src> {
    pub diagnosis: UnsatisfiedBound<'src>,
}

impl<'src> From<UnsatisfiedBound<'src>> for UnsatisfiedBoundError<'src> {
    fn from(diagnosis: UnsatisfiedBound<'src>) -> Self {
        Self { diagnosis }
    }
}

impl<'src> From<DeniedBound<'src>> for UnsatisfiedBoundError<'src> {
    fn from(value: DeniedBound<'src>) -> Self {
        Self::from(UnsatisfiedBound::from(value))
    }
}

impl<'src> From<AllowedBound<'src>> for UnsatisfiedBoundError<'src> {
    fn from(value: AllowedBound<'src>) -> Self {
        Self::from(UnsatisfiedBound::from(value))
    }
}

impl<'src> TryFrom<super::TypeBoundRequiredness<'src>> for UnsatisfiedBoundError<'src> {
    /// Unexpectedly success
    type Error = RequiredBound<'src>;

    fn try_from(value: super::TypeBoundRequiredness<'src>) -> Result<Self, Self::Error> {
        UnsatisfiedBound::try_from(value).map(Self::from)
    }
}

impl Display for UnsatisfiedBoundError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { diagnosis } = self;
        clear_defs(); // can't trust them to be emptied if Display can error before reaching the end
        let BoundCheck { bound, attempt, .. } = diagnosis;
        {
            let items: [(&dyn AliasPrefix, _); 2] = [(bound, None), (attempt, None)];
            let_if_new(items, f)?;
            writeln!(f, ".")?;
        }
        let attempt_var = match attempt {
            CheckedType::Abstract(_) => {
                write!(f, "the type `{:#}`", Aliased::new(attempt))?;
                None
            }

            CheckedType::Bound(attempt) => {
                let t = let_local(attempt, f)?;
                write!(f, ". the type `{t:#}`")?;
                Some(t)
            }
        };
        writeln!(
            f,
            " does not satisfy the bound `{:#}`.",
            Aliased::new(bound)
        )?;
        writeln!(f, "observe:")?;
        diagnosis.fmt_check(attempt_var, f)
    }
}

impl std::error::Error for UnsatisfiedBoundError<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::types::{StaticType, bounds::TypeBoundDesc};

    #[test]
    fn test_bounds_error_display() {
        let a = TypeBound::Exactly(AbstractType::Char);
        let b = TypeBound::Exactly(AbstractType::Uint);
        let c = TypeBound::Has {
            member: "foo",
            req: None,
        };
        let d = TypeBound::Not(Box::new(c.clone()));
        let e = TypeBound::Combo(Combinator::One, [a.clone(), b.clone(), d.clone()].to_vec());

        let x = CheckedType::Abstract(AbstractType::Uint);

        let error = UnsatisfiedBoundError {
            diagnosis: UnsatisfiedBound {
                bound: e.clone(),
                attempt: x.clone(),
                reason: UnsatisfiedBoundReason::OverOne(Box::new((
                    SatisfiedBound {
                        bound: b,
                        attempt: x.clone(),
                        reason: SatisfiedBoundReason::Exactly,
                    },
                    SatisfiedBound {
                        bound: d,
                        attempt: x.clone(),
                        reason: SatisfiedBoundReason::Not(Box::new(UnsatisfiedBound {
                            bound: c,
                            attempt: x,
                            reason: UnsatisfiedBoundReason::MissingMember,
                        })),
                    },
                ))),
            },
        };
        println!("\n{error}\n");
        println!(
            "NOTE: this unit test passing does not guarantee correctness. the output must be rated manually"
        );
    }

    #[test]
    fn test_bounds_of_bounds_error_display() {
        let mut a = TypeBound::Combo(
            Combinator::All,
            [
                TypeBound::Not(Box::new(TypeBound::Combo(
                    Combinator::Any,
                    [
                        TypeBound::Exactly(AbstractType::Bool),
                        TypeBound::Has {
                            member: "items",
                            req: None,
                        },
                    ]
                    .to_vec(),
                ))),
                TypeBound::Exactly(AbstractType::Uint),
            ]
            .to_vec(),
        );
        a.simplify();
        println!("{a:#}:\n{}", TypeBoundDesc::new(&a));
        println!();

        let mut b = TypeBound::Combo(
            Combinator::One,
            [
                TypeBound::Exactly(AbstractType::Uint),
                TypeBound::Exactly(AbstractType::Sint),
                TypeBound::Combo(
                    Combinator::Any,
                    [
                        TypeBound::Has {
                            member: "join",
                            req: Some(Box::new(TypeBound::Shape(FunShape {
                                params: [StaticType::Text, StaticType::Text].to_vec(),
                                ret: StaticType::Text,
                            }))),
                        },
                        TypeBound::Not(Box::new(TypeBound::Has {
                            member: "items",
                            req: Some(Box::new(TypeBound::Combo(
                                Combinator::All,
                                [
                                    TypeBound::Has {
                                        member: "len",
                                        req: Some(Box::new(TypeBound::Exactly(AbstractType::Uint))),
                                    },
                                    TypeBound::Combo(
                                        Combinator::Any,
                                        [
                                            TypeBound::Exactly(AbstractType::Text),
                                            TypeBound::Exactly(AbstractType::Char),
                                        ]
                                        .to_vec(),
                                    ),
                                ]
                                .to_vec(),
                            ))),
                        })),
                    ]
                    .to_vec(),
                ),
            ]
            .to_vec(),
        );
        b.simplify();
        println!("{b:#}:\n{}", TypeBoundDesc::new(&b));

        let error = UnsatisfiedBoundError::try_from(a.satisfaction_of(&b)).unwrap_or_else(|ok| {
            panic!("expected an error, found: {ok}");
        });
        println!("\n{error}\n");
        println!(
            "NOTE: this unit test passing does not guarantee correctness. the output must be rated manually"
        );
    }

    #[test]
    fn test_non_guaranteeing_error_display() {
        let mut a = TypeBound::Combo(
            Combinator::Any,
            [
                TypeBound::Has {
                    member: "foo",
                    req: Some(Box::new(TypeBound::Exactly(AbstractType::Uint))),
                },
                TypeBound::Shape(FunShape {
                    params: [StaticType::Bool, StaticType::Uint].to_vec(),
                    ret: StaticType::None,
                }),
            ]
            .to_vec(),
        );
        a.simplify();
        println!("{a:#}:\n{}", TypeBoundDesc::new(&a));
        println!();

        let mut b = TypeBound::Combo(
            Combinator::Any,
            [
                TypeBound::Has {
                    member: "bar",
                    req: None,
                },
                TypeBound::Shape(FunShape {
                    params: Vec::new(),
                    ret: StaticType::None,
                }),
            ]
            .to_vec(),
        );
        b.simplify();
        println!("{b:#}:\n{}", TypeBoundDesc::new(&b));

        let error = UnsatisfiedBoundError::try_from(a.satisfaction_of(&b)).unwrap_or_else(|ok| {
            panic!("expected an error, found: {ok}");
        });
        println!("\n{error}\n");
        println!(
            "NOTE: this unit test passing does not guarantee correctness. the output must be rated manually"
        );
    }
}
