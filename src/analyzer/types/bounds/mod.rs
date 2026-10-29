use crate::{
    SYNTAX_STYLE_ANSI,
    analyzer::{
        static_env::EnvKey,
        types::{
            AbstractType, FunShape, TypeCompatible,
            bounds::{
                error::{
                    AllowedBound, AllowedBoundReason, BoundCheck, CheckedType, DeniedBound,
                    DeniedBoundReason, RequiredBound, RequiredBoundReason, SatisfiedBound,
                    SatisfiedBoundReason, UnsatisfiedBound, UnsatisfiedBoundReason,
                },
                pure::PureExpr,
            },
        },
    },
    error::OpSide,
    highlight::style::StyleWrapper,
};
use std::fmt::Display;

pub mod error;
pub mod pure;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Satisfaction<T, U, E> {
    Passing(T),
    Unclear(U),
    Failing(E),
}

impl<T, U, E> Satisfaction<T, U, E> {
    pub const fn is_passing(&self) -> bool {
        matches!(self, Self::Passing(_))
    }

    pub const fn is_unclear(&self) -> bool {
        matches!(self, Self::Unclear(_))
    }

    pub const fn is_failing(&self) -> bool {
        matches!(self, Self::Failing(_))
    }

    pub const fn is_possible(&self) -> bool {
        matches!(self, Self::Passing(_) | Self::Unclear(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Requiredness<T, U, E> {
    Require(T),
    Allow(U),
    Deny(E),
}

impl<T, U, E> Requiredness<T, U, E> {
    pub const fn is_require(&self) -> bool {
        matches!(self, Self::Require(_))
    }

    pub const fn is_allow(&self) -> bool {
        matches!(self, Self::Allow(_))
    }

    pub const fn is_deny(&self) -> bool {
        matches!(self, Self::Deny(_))
    }

    pub const fn is_possible(&self) -> bool {
        matches!(self, Self::Require(_) | Self::Allow(_))
    }
}

pub type TypeBoundSatisfaction<'src> =
    Satisfaction<SatisfiedBound<'src>, AllowedBound<'src>, UnsatisfiedBound<'src>>;

pub type TypeBoundSatisfactionReason<'src> = Satisfaction<
    SatisfiedBoundReason<'src>,
    AllowedBoundReason<'src>,
    UnsatisfiedBoundReason<'src>,
>;

pub type TypeBoundRequiredness<'src> =
    Requiredness<RequiredBound<'src>, AllowedBound<'src>, DeniedBound<'src>>;

pub type TypeBoundRequirednessReason<'src> =
    Requiredness<RequiredBoundReason<'src>, AllowedBoundReason<'src>, DeniedBoundReason<'src>>;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum OpBound<'src> {
    Unary(OpSide),
    Binary { rhs: Option<Box<TypeBound<'src>>> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Combinator {
    /// Combined with `&&`
    #[default]
    All,
    /// Combined with `||`
    Any,
    /// Exactly one must match
    One,
}

/// A single [`TypeBound`] can include multiple bounds through [`Self::Combo`],
/// so prefer [`Box<TypeBound>`] over [`Vec<TypeBound>`]
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum TypeBound<'src> {
    /// Satisfies a combination of conditions
    ///
    /// Conditions combined using [`Combinator::All`] must not conflict (i.e. if one requires, another cannot deny) to be legal
    ///
    /// Sub-conditions that are combos with the same combinator should be expanded into the outer combo
    /// (i.e. there should be no `Any(Any(...), Any(...))`, `All(All(...), All(...))`, or `One(One(...), One(...))`)
    Combo(Combinator, Vec<Self>),

    /// Does not satisfy the condition
    ///
    /// If inner condition is also [`Self::Not`], it should be expanded into the outer bound
    /// (i.e. there should be no `Not(Not(...))`)
    Not(Box<Self>),

    /// Is a specific type
    Exactly(AbstractType<'src>),

    /// Possesses a set of members (methods or fields) which each meet the qualifications
    Has {
        /// Name of the member
        member: &'src str,

        /// Bound every member must satisfy
        req: Option<Box<Self>>,
    },

    /// Fits a function shape
    Shape(FunShape<'src>),

    /// An operator the type must overload
    Supports {
        op: EnvKey<'src>,
        operands: OpBound<'src>,
        ret: Option<Box<TypeBound<'src>>>,
    },

    /// An expression the type must be guaranteed to satisfy at runtime, such as "x!=0", "x>=0", "6<=x<=10", "x.foo==x.bar", etc.
    /// Mainly enforced by checking if conditions/assertions guard this, or if the user promises it unsafely.
    ///
    /// `Satisfies` can only check pure operations. Only builtin operations are trusted.
    /// The expression must result in a bool.
    Satisfies {
        /// The variable representing any instance of the self type
        var: &'src str,
        /// The expression that must be true
        pred: PureExpr<'src>,
    },
}

impl FromIterator<Self> for TypeBound<'_> {
    fn from_iter<I: IntoIterator<Item = Self>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        if let Some(mut first) = iter.next() {
            first.extend(iter);
            first
        } else {
            Self::default()
        }
    }
}

impl Extend<Self> for TypeBound<'_> {
    fn extend<I: IntoIterator<Item = Self>>(&mut self, iter: I) {
        if let Self::Combo(Combinator::All, existing) = self {
            for bound in iter {
                if let Self::Combo(Combinator::All, additional) = bound {
                    existing.extend(additional);
                } else {
                    existing.push(bound);
                }
            }
        } else {
            *self = Self::Combo(
                Combinator::All,
                std::iter::once(self.clone()).chain(iter).collect(),
            );
        }
    }
}

impl<'a> Extend<&'a Self> for TypeBound<'_> {
    fn extend_one(&mut self, item: &'a Self) {
        if let Self::Combo(Combinator::All, existing) = self {
            if let Self::Combo(Combinator::All, additional) = item {
                existing.extend_from_slice(additional);
            } else {
                existing.push(item.clone());
            }
        } else {
            *self = Self::Combo(Combinator::All, vec![self.clone(), item.clone()]);
        }
    }

    fn extend<I: IntoIterator<Item = &'a Self>>(&mut self, iter: I) {
        if let Self::Combo(Combinator::All, existing) = self {
            for bound in iter {
                if let Self::Combo(Combinator::All, additional) = bound {
                    existing.extend_from_slice(additional);
                } else {
                    existing.push(bound.clone());
                }
            }
        } else {
            *self = Self::Combo(
                Combinator::All,
                std::iter::once(self.clone())
                    .chain(iter.into_iter().cloned())
                    .collect(),
            );
        }
    }
}

impl Default for TypeBound<'_> {
    fn default() -> Self {
        Self::Combo(Combinator::All, Vec::new())
    }
}

#[repr(transparent)]
pub struct TypeBoundDesc<'src>(TypeBound<'src>);

impl<'src> TypeBoundDesc<'src> {
    pub const fn new<'a>(value: &'a TypeBound<'src>) -> &'a Self {
        // SAFETY: TypeBoundDesc is a transparent wrapper for TypeBound
        unsafe { std::mem::transmute(value) }
    }
}

impl std::fmt::Display for TypeBoundDesc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.descriptive_fmt(0, f)
    }
}

impl TypeBound<'_> {
    /// Written in the format "must _"
    fn descriptive_fmt(&self, depth: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indent = |f: &mut std::fmt::Formatter<'_>| write!(f, "\n{0:1$} - ", "", depth * 2);
        match self {
            Self::Combo(kind, bounds) => {
                write!(
                    f,
                    "satisfy {} of the following:",
                    match kind {
                        Combinator::All => "all",
                        Combinator::Any => "any",
                        Combinator::One => "one",
                    }
                )?;
                for bound in bounds {
                    indent(f)?;
                    bound.descriptive_fmt(depth + 1, f)?;
                }
                Ok(())
            }

            Self::Not(bound) => {
                write!(f, "not:")?;
                indent(f)?;
                bound.descriptive_fmt(depth + 1, f)
            }

            Self::Exactly(ty) => {
                write!(f, "be exactly {ty:#}")
            }

            Self::Has { member, req } => {
                write!(f, "have a member named `{member}`")?;
                if let Some(req) = req {
                    write!(f, " which must:")?;
                    indent(f)?;
                    req.descriptive_fmt(depth + 1, f)?;
                }
                Ok(())
            }

            Self::Shape(shape) => {
                write!(f, "be callable with the signature {shape:#}")
            }

            Self::Supports { .. } => todo!(),

            Self::Satisfies { var, pred } => {
                write!(f, "satisfy {pred:#} for every instance {var:#}")
            }
        }
    }
}

impl Display for TypeBound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn parenthesize_if_needed(
            bound: &TypeBound<'_>,
            f: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            if matches!(
                bound,
                TypeBound::Exactly(_)
                    | TypeBound::Not(_)
                    | TypeBound::Has {
                        member: _,
                        req: None,
                    }
                    | TypeBound::Shape(_)
            ) {
                bound.fmt(f)
            } else {
                f.write_str("(")?;
                bound.fmt(f)?;
                f.write_str(")")
            }
        }

        match self {
            Self::Combo(kind, bounds) => {
                let sep = match kind {
                    Combinator::All => " + ",
                    Combinator::Any => " | ", // what about fallible types?
                    Combinator::One => " / ",
                };
                let mut has_prev = false;
                for bound in bounds {
                    if has_prev {
                        SYNTAX_STYLE_ANSI.keyword.alt_style(sep).fmt(f)?;
                    }
                    has_prev = true;
                    parenthesize_if_needed(bound, f)?;
                }
                Ok(())
            }

            Self::Not(bound) => {
                SYNTAX_STYLE_ANSI.keyword.alt_style("!").fmt(f)?;
                parenthesize_if_needed(bound, f)
            }

            Self::Exactly(ty) => {
                SYNTAX_STYLE_ANSI.keyword.alt_style("only ").fmt(f)?;
                ty.fmt(f)
            }

            Self::Has { member, req } => {
                SYNTAX_STYLE_ANSI.keyword.alt_style("has ").fmt(f)?;
                SYNTAX_STYLE_ANSI.variable.alt_style(member).fmt(f)?;
                if let Some(req) = req {
                    SYNTAX_STYLE_ANSI.keyword.alt_style(": ").fmt(f)?;
                    req.fmt(f)
                } else {
                    Ok(())
                }
            }

            Self::Shape(shape) => shape.fmt(f),

            Self::Supports { .. } => todo!(),

            Self::Satisfies { var, pred } => {
                SYNTAX_STYLE_ANSI.keyword.alt_style("for(").fmt(f)?;
                SYNTAX_STYLE_ANSI.variable.alt_style(var).fmt(f)?;
                SYNTAX_STYLE_ANSI.keyword.alt_style("): ").fmt(f)?;
                pred.fmt(f)
            }
        }
    }
}

pub trait TypeSatisfaction<'src> {
    fn satisfaction_of(&self, bound: &TypeBound<'src>) -> TypeBoundSatisfaction<'src>;
}

impl<'src, T: TypeCompatible<'src>> TypeSatisfaction<'src> for T {
    fn satisfaction_of(&self, bound: &TypeBound<'src>) -> TypeBoundSatisfaction<'src> {
        use Satisfaction::{Failing, Passing, Unclear};
        let reason: TypeBoundSatisfactionReason = match bound {
            TypeBound::Combo(kind, bounds) => match kind {
                Combinator::All => {
                    let mut res = None;
                    for item in bounds.iter().map(|bound| self.satisfaction_of(bound)) {
                        // failure takes priority over uncertainty
                        match item {
                            Unclear(_) => {
                                if res.is_none() {
                                    res = Some(item);
                                }
                            }

                            Failing(_) => {
                                res = Some(item);
                                break;
                            }

                            Passing(_) => {} // not enough on its own
                        }
                    }
                    if let Some(res) = res {
                        match res {
                            Unclear(x) => Unclear(AllowedBoundReason::IncompleteAll(Box::new(x))),

                            Failing(x) => Failing(UnsatisfiedBoundReason::NotAll(Box::new(x))),

                            Passing(_) => unreachable!(),
                        }
                    } else {
                        Passing(SatisfiedBoundReason::All)
                    }
                }

                Combinator::Any => {
                    let mut res = None;
                    for item in bounds.iter().map(|bound| self.satisfaction_of(bound)) {
                        // even if an unclear exists, there might be a passing item, and that would guarantee success.
                        match item {
                            Passing(_) => {
                                res = Some(item);
                                break;
                            }

                            Unclear(_) => {
                                if res.is_none() {
                                    res = Some(item);
                                }
                            }

                            Failing(_) => {} // not enough on its own
                        }
                    }
                    if let Some(res) = res {
                        match res {
                            Passing(x) => Passing(SatisfiedBoundReason::Any(Box::new(x))),
                            Unclear(x) => Unclear(AllowedBoundReason::AnyBestAllowed(Box::new(x))),
                            Failing(_) => unreachable!(),
                        }
                    } else {
                        Failing(UnsatisfiedBoundReason::NotAny)
                    }
                }

                Combinator::One => {
                    let mut first = None;
                    let mut second = None;
                    for item in bounds.iter().map(|bound| self.satisfaction_of(bound)) {
                        // passing takes priority over unclear
                        match item {
                            Passing(_) => {
                                if first.is_none() {
                                    first = Some(item);
                                } else {
                                    second = Some(item);
                                    break;
                                }
                            }

                            Unclear(_) => {
                                if first.is_none() {
                                    first = Some(item);
                                } else if second.is_none() {
                                    second = Some(item);
                                }
                            }

                            Failing(_) => {} // not enough on its own
                        }
                    }
                    if let Some(first) = first {
                        if let Some(second) = second {
                            // multiple
                            match (first, second) {
                                (Passing(first), Passing(second)) => Failing(
                                    UnsatisfiedBoundReason::OverOne(Box::new((first, second))),
                                ),

                                (Passing(passing), Unclear(unclear))
                                | (Unclear(unclear), Passing(passing)) => {
                                    Unclear(AllowedBoundReason::OnePassingMultipleAllowed(
                                        Box::new((passing, unclear)),
                                    ))
                                }

                                (Unclear(first), Unclear(second)) => {
                                    Unclear(AllowedBoundReason::OneAllowsMultiple(Box::new((
                                        first, second,
                                    ))))
                                }

                                (Failing(_), _) | (_, Failing(_)) => unreachable!(),
                            }
                        } else {
                            // one
                            match first {
                                Passing(x) => Passing(SatisfiedBoundReason::One(Box::new(x))),

                                Unclear(x) => {
                                    Unclear(AllowedBoundReason::OneBestAllowed(Box::new(x)))
                                }

                                Failing(_) => unreachable!(),
                            }
                        }
                    } else {
                        Failing(UnsatisfiedBoundReason::NotOne)
                    }
                }
            },

            TypeBound::Not(bound) => match self.satisfaction_of(bound) {
                Passing(reason) => Failing(UnsatisfiedBoundReason::NotNot(Box::new(reason))),

                Unclear(reason) => Unclear(AllowedBoundReason::IneffectiveNot(Box::new(reason))),

                Failing(reason) => Passing(SatisfiedBoundReason::Not(Box::new(reason))),
            },

            TypeBound::Exactly(target) => {
                if self.is_type_eq(target) {
                    Passing(SatisfiedBoundReason::Exactly)
                } else {
                    Failing(UnsatisfiedBoundReason::Unequal)
                }
            }

            TypeBound::Has { member, req } => {
                if let Some(attempt) = self.get_member(member) {
                    if let Some(req) = req.as_deref() {
                        let res: TypeBoundSatisfaction = match &attempt {
                            CheckedType::Abstract(ty) => ty.satisfaction_of(req),

                            CheckedType::Bound(b) => {
                                b.satisfaction_of(req);
                                todo!()
                            }
                        };
                        match res {
                            Passing(x) => Passing(SatisfiedBoundReason::HasMatching(Box::new(x))),
                            Unclear(x) => Unclear(AllowedBoundReason::HasAllowed(Box::new(x))),
                            Failing(x) => Failing(UnsatisfiedBoundReason::MemberFails(Box::new(x))),
                        }
                    } else {
                        Passing(SatisfiedBoundReason::Has)
                    }
                } else {
                    Failing(UnsatisfiedBoundReason::MissingMember)
                }
            }

            TypeBound::Shape(shape) => {
                if let Some(actual) = self.shape() {
                    if *shape == actual {
                        Passing(SatisfiedBoundReason::Shape)
                    } else {
                        Failing(UnsatisfiedBoundReason::ShapeFails(actual))
                    }
                } else {
                    Failing(UnsatisfiedBoundReason::MissingShape)
                }
            }

            TypeBound::Supports { .. } => todo!("type bound needs environment access"),

            TypeBound::Satisfies { .. } => todo!("type bound needs code access"),
        };

        // ew, so much recursive cloning...
        let bound = bound.clone();
        let attempt = CheckedType::Abstract(self.to_abstract());
        match reason {
            Passing(reason) => Passing(BoundCheck {
                bound,
                attempt,
                reason,
            }),
            Unclear(reason) => Unclear(BoundCheck {
                bound,
                attempt,
                reason,
            }),
            Failing(reason) => Failing(BoundCheck {
                bound,
                attempt,
                reason,
            }),
        }
    }
}

impl<'src> TypeBound<'src> {
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Combo(_, []))
    }

    /// Expand nested combos and double-negations
    pub fn simplify(&mut self) {
        match self {
            &mut Self::Combo(kind, ref mut bounds) => {
                for bound in bounds.iter_mut() {
                    bound.simplify();
                }

                if bounds.len() == 1 {
                    *self = bounds.pop().expect("just checked");
                }
                // associative
                else if bounds
                    .iter()
                    .any(|bound| matches!(bound, &Self::Combo(inner_kind, _) if inner_kind == kind))
                {
                    let len = bounds
                        .iter()
                        .map(|bound| match bound {
                            &Self::Combo(inner_kind, ref inner_bounds) if inner_kind == kind => {
                                inner_bounds.len()
                            }
                            _ => 1,
                        })
                        .sum();
                    for bound in std::mem::replace(bounds, Vec::with_capacity(len)) {
                        match bound {
                            Self::Combo(inner_kind, inner_bounds) if inner_kind == kind => {
                                bounds.extend(inner_bounds);
                            }
                            bound => {
                                bounds.push(bound);
                            }
                        }
                    }
                    bounds.sort_by_cached_key(ToString::to_string);
                    // duplicates in `Combinator::One` make it illegal. deduping would make it legal, eliminating a helpful error for users.
                    if kind != Combinator::One {
                        bounds.dedup();
                        if bounds.len() == 1 {
                            *self = bounds.pop().expect("just checked");
                        }
                    }
                }
            }

            Self::Not(bound) => {
                bound.simplify();

                // double negation
                if let Self::Not(inner) = bound {
                    *self = *std::mem::take(inner);
                }

                debug_assert!(
                    !matches!(self, Self::Not(Self::Not(_))),
                    "should have at most 1 `not` after reducing double negation"
                );
            }

            Self::Has {
                req: req @ Some(_), ..
            } => {
                let Some(bound) = req else { unreachable!() };

                bound.simplify();

                // empty bound is no bound
                if bound.is_empty() {
                    *req = None;
                }
            }

            Self::Satisfies { .. } => {
                println!("TODO: simplify expression");
            }

            // do not contain nested bounds
            Self::Exactly(_)
            | Self::Has { req: None, .. }
            | Self::Shape(_)
            | Self::Supports { .. } => {}
        }
    }

    pub fn is_legal(&self) -> bool {
        !matches!(self, Self::Combo(_, []) | Self::Not(Self::Combo(_, [])))
            && matches!(self.satisfaction_of(self), Requiredness::Require(_))
    }

    pub fn satisfaction_of(&self, req: &TypeBound<'src>) -> TypeBoundRequiredness<'src> {
        use Requiredness::{Allow, Deny, Require};
        use Satisfaction::{Failing, Passing, Unclear};
        let reason: TypeBoundRequirednessReason = match self {
            Self::Combo(kind, bounds) => {
                match kind {
                    // if any bound requires the condition, the requirement is satisfied.
                    // since all bounds must be satisfied, if one of them requires it, it is always required.
                    //
                    // if any bound rejects the condition, the requirement is denied.
                    // since all bounds must be satisfied, if one of them rejects it, it is always rejected.
                    //
                    // if no bound requires or rejects the condition,
                    // we do not have enough information to determine if it is satisfied or not.
                    //
                    // if one bound rejects the condition and another requires it,
                    // the bound is illegal and we needn't cover it in this method.
                    Combinator::All => {
                        if let Some(res) = bounds
                            .iter()
                            .map(|bound| bound.satisfaction_of(req))
                            .find(|res| matches!(res, Require(_) | Deny(_)))
                        {
                            match res {
                                Require(x) => Require(RequiredBoundReason::OneOfAll(Box::new(x))),

                                Deny(x) => Deny(DeniedBoundReason::OneOfAll(Box::new(x))),

                                Allow(_) => unreachable!(),
                            }
                        } else {
                            Allow(AllowedBoundReason::NoneOfAll)
                        }
                    }

                    // if every bound requires the condition, the requirement is satisfied.
                    // since only one bound must be satisfied, if any of them does not satisfy the condition,
                    // that might be the only one being satisfied.
                    //
                    // if any bound rejects the condition, the requirement is denied.
                    // since only one bound must be satisfied, if any of them rejects the condition,
                    // that could be among the ones being satisfied.
                    //
                    // if no bound requires or rejects the condition,
                    // we do not have enough information to determine if it is satisfied or not.
                    //
                    // if one bound rejects the condition and another requires it,
                    // the bound is illegal and we needn't cover it in this method.
                    Combinator::Any | Combinator::One => {
                        let mut res = None;
                        for item in bounds.iter().map(|bound| bound.satisfaction_of(req)) {
                            match item {
                                Deny(x) => {
                                    res = Some(Deny(match kind {
                                        Combinator::All => unreachable!(),
                                        Combinator::Any => DeniedBoundReason::OneOfAny(Box::new(x)),
                                        Combinator::One => DeniedBoundReason::OneOfOne(Box::new(x)),
                                    }));
                                    break;
                                }

                                Allow(x) => {
                                    if res.is_none() {
                                        res = Some(Allow(match kind {
                                            Combinator::All => unreachable!(),
                                            Combinator::Any => {
                                                AllowedBoundReason::OneOfAny(Box::new(x))
                                            }
                                            Combinator::One => {
                                                AllowedBoundReason::OneOfOne(Box::new(x))
                                            }
                                        }));
                                    }
                                }

                                Require(_) => {}
                            }
                        }
                        res.unwrap_or_else(|| {
                            Require(match kind {
                                Combinator::All => unreachable!(),
                                Combinator::Any => RequiredBoundReason::AllOfAny,
                                Combinator::One => RequiredBoundReason::AllOfOne,
                            })
                        })
                    }
                }
            }

            // if the negated bound requires the condition, the requirement is denied.
            // since the bound must be satisfied, negating it means the bound must be unsatisfied.
            //
            // if the negated bound rejects the condition, the requirement is satisfied.
            // since the bound must be unsatisfied, negating it means the bound must be satisfied.
            //
            // if the negated bound neither requires nor rejects the condition,
            // we do not have enough information to determine if it is satisfied or not.
            Self::Not(bound) => match bound.satisfaction_of(req) {
                Require(x) => Allow(AllowedBoundReason::NotRequired(Box::new(x))),

                Allow(x) => Allow(AllowedBoundReason::IneffectiveNot(Box::new(x))),

                Deny(x) => Allow(AllowedBoundReason::NotDenied(Box::new(x))),
            },

            // being a type that satisfies the requirement clearly satisfies the requirement.
            //
            // because the type must be exactly this one, if this one does not satisfy the requirement,
            // the requirement cannot be satisfied in any case where the type must be this one.
            Self::Exactly(ty) => match ty.satisfaction_of(req) {
                Passing(x) => Require(RequiredBoundReason::Exactly(Box::new(x))),
                Unclear(u) => Allow(AllowedBoundReason::ExactlyAllowed(Box::new(u))),
                Failing(e) => Deny(DeniedBoundReason::ExactlyUnsatisfying(Box::new(e))),
            },

            Self::Has {
                member: my_member,
                req: my_req,
            } => match req {
                Self::Not(_) => todo!(),

                Self::Has {
                    member: their_member,
                    req: their_req,
                } => {
                    if my_member == their_member {
                        if let Some(their_req) = their_req.as_deref() {
                            if let Some(my_req) = my_req.as_deref() {
                                match my_req.satisfaction_of(their_req) {
                                    Require(x) => {
                                        Require(RequiredBoundReason::HasMatching(Box::new(x)))
                                    }

                                    Allow(x) => Allow(AllowedBoundReason::HasAllowed(Box::new(x))),

                                    Deny(x) => Deny(DeniedBoundReason::HasConflicting(Box::new(x))),
                                }
                            } else {
                                Allow(AllowedBoundReason::HasUndenied)
                            }
                        } else {
                            Require(RequiredBoundReason::Has)
                        }
                    } else {
                        Allow(AllowedBoundReason::MayAlsoHave)
                    }
                }

                // it is possible the type satisfies the requirement, but these bounds alone don't guarantee it.
                _ => Allow(AllowedBoundReason::MayHave),
            },

            Self::Shape(shape) => match req {
                Self::Not(_) => todo!(),

                // TODO: are there cases where a function shape can be equivalent but not equal?
                Self::Shape(req_shape) if shape == req_shape => Require(RequiredBoundReason::Shape),

                // it is possible the type satisfies the requirement, but these bounds alone don't guarantee it.
                _ => Allow(AllowedBoundReason::ShapeUndenied),
            },

            Self::Supports { .. } => todo!("`supports` needs fleshing out"),

            Self::Satisfies { var, pred } => todo!("`satisfies`"),
        };

        let bound = req.clone();
        let attempt = CheckedType::Bound(self.clone());
        match reason {
            Require(reason) => Require(BoundCheck {
                bound,
                attempt,
                reason,
            }),

            Allow(reason) => Allow(BoundCheck {
                bound,
                attempt,
                reason,
            }),

            Deny(reason) => Deny(BoundCheck {
                bound,
                attempt,
                reason,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod display {
        use super::*;

        #[test]
        fn test_exactly_primitives() {
            for ty in [
                AbstractType::None,
                AbstractType::Nevr,
                AbstractType::Fail,
                AbstractType::Bool,
                AbstractType::Byte,
                AbstractType::Uint,
                AbstractType::Sint,
                AbstractType::Flpt,
                AbstractType::Char,
                AbstractType::Text,
            ] {
                let expect = ty.to_string();
                let bound = TypeBound::Exactly(ty);
                assert_eq!(bound.to_string(), format!("only {expect}"));
            }
        }

        #[test]
        fn test_combo_none() {
            for kind in [Combinator::All, Combinator::Any, Combinator::One] {
                let bound = TypeBound::Combo(kind, Vec::new());
                // TBD: is this what the bound should actually look like? should empty combos even be legal?
                assert_eq!(bound.to_string(), "");
            }
        }

        #[test]
        fn test_combo_single() {
            for kind in [Combinator::All, Combinator::Any, Combinator::One] {
                // TBD: should singular combos even be legal?
                let bound = TypeBound::Combo(kind, vec![TypeBound::Exactly(AbstractType::Bool)]);
                assert_eq!(bound.to_string(), "only bool");
            }
        }

        #[test]
        fn test_combo_all_exact_pair() {
            let bound = TypeBound::Combo(
                Combinator::All,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                ],
            );
            assert_eq!(bound.to_string(), "only bool + only uint");
        }

        #[test]
        fn test_combo_all_exact_triplet() {
            let bound = TypeBound::Combo(
                Combinator::All,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                    TypeBound::Exactly(AbstractType::Sint),
                ],
            );
            assert_eq!(bound.to_string(), "only bool + only uint + only sint");
        }

        #[test]
        fn test_combo_any_exact_pair() {
            let bound = TypeBound::Combo(
                Combinator::Any,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                ],
            );
            assert_eq!(bound.to_string(), "only bool | only uint");
        }

        #[test]
        fn test_combo_any_exact_triplet() {
            let bound = TypeBound::Combo(
                Combinator::Any,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                    TypeBound::Exactly(AbstractType::Sint),
                ],
            );
            assert_eq!(bound.to_string(), "only bool | only uint | only sint");
        }

        #[test]
        fn test_combo_one_exact_pair() {
            let bound = TypeBound::Combo(
                Combinator::One,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                ],
            );
            assert_eq!(bound.to_string(), "only bool / only uint");
        }

        #[test]
        fn test_combo_one_exact_triplet() {
            let bound = TypeBound::Combo(
                Combinator::One,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                    TypeBound::Exactly(AbstractType::Sint),
                ],
            );
            assert_eq!(bound.to_string(), "only bool / only uint / only sint");
        }

        #[test]
        fn test_combo_has() {
            let bound = TypeBound::Has {
                member: "x",
                req: None,
            };
            assert_eq!(bound.to_string(), "has x");
        }

        #[test]
        fn test_combo_has_exact() {
            let bound = TypeBound::Has {
                member: "x",
                req: Some(Box::new(TypeBound::Exactly(AbstractType::Bool))),
            };
            assert_eq!(bound.to_string(), "has x: only bool");
        }

        #[test]
        fn test_combo_has_combo_exact() {
            let bound = TypeBound::Has {
                member: "x",
                req: Some(Box::new(TypeBound::Combo(
                    Combinator::All,
                    vec![
                        TypeBound::Exactly(AbstractType::Bool),
                        TypeBound::Exactly(AbstractType::Uint),
                    ],
                ))),
            };
            assert_eq!(bound.to_string(), "has x: only bool + only uint");
        }

        #[test]
        fn test_combo_has_combo_exact_wrapped() {
            let bound = TypeBound::Combo(
                Combinator::All,
                vec![
                    TypeBound::Has {
                        member: "x",
                        req: Some(Box::new(TypeBound::Combo(
                            Combinator::All,
                            vec![
                                TypeBound::Exactly(AbstractType::Bool),
                                TypeBound::Exactly(AbstractType::Uint),
                            ],
                        ))),
                    },
                    TypeBound::Exactly(AbstractType::Sint),
                ],
            );
            assert_eq!(
                bound.to_string(),
                "(has x: only bool + only uint) + only sint"
            );
        }

        #[test]
        fn test_negative_unwrapped() {
            let bound = TypeBound::Not(Box::new(TypeBound::Exactly(AbstractType::Bool)));
            assert_eq!(bound.to_string(), "!only bool");
        }

        #[test]
        fn test_double_negative_unwrapped() {
            let bound = TypeBound::Not(Box::new(TypeBound::Not(Box::new(TypeBound::Exactly(
                AbstractType::Bool,
            )))));
            assert_eq!(bound.to_string(), "!!only bool");
        }

        #[test]
        fn test_negative_combo_wrapped() {
            let bound = TypeBound::Not(Box::new(TypeBound::Combo(
                Combinator::All,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                ],
            )));
            assert_eq!(bound.to_string(), "!(only bool + only uint)");
        }

        #[test]
        fn test_double_negative_combo_wrapped() {
            let bound = TypeBound::Not(Box::new(TypeBound::Not(Box::new(TypeBound::Combo(
                Combinator::All,
                vec![
                    TypeBound::Exactly(AbstractType::Bool),
                    TypeBound::Exactly(AbstractType::Uint),
                ],
            )))));
            assert_eq!(bound.to_string(), "!!(only bool + only uint)");
        }

        #[test]
        fn test_negative_combo_negative() {
            let bound = TypeBound::Not(Box::new(TypeBound::Combo(
                Combinator::All,
                vec![
                    TypeBound::Not(Box::new(TypeBound::Exactly(AbstractType::Bool))),
                    TypeBound::Exactly(AbstractType::Uint),
                ],
            )));
            assert_eq!(bound.to_string(), "!(!only bool + only uint)");
        }

        #[test]
        fn test_satisfies() {
            use pure::*;
            let bound = TypeBound::Satisfies {
                var: "x",
                pred: PureExpr::Operation(Box::new(PureOperation::Gt(
                    PureExpr::Operation(Box::new(PureOperation::Mul(
                        PureExpr::Literal(PureLiteral::Sint(3)),
                        PureExpr::Variable(PureVariable("x")),
                    ))),
                    PureExpr::Operation(Box::new(PureOperation::Add(
                        PureExpr::Variable(PureVariable("x")),
                        PureExpr::Literal(PureLiteral::Sint(5)),
                    ))),
                ))),
            };
            println!("{bound:#}");
            assert_eq!(bound.to_string(), "for(x): 3*x > x + 5");
        }

        #[test]
        fn test_satisfies_precedence1() {
            use pure::*;
            let bound = TypeBound::Satisfies {
                var: "x",
                pred: PureExpr::Operation(Box::new(PureOperation::Gt(
                    PureExpr::Operation(Box::new(PureOperation::Mul(
                        PureExpr::Literal(PureLiteral::Sint(3)),
                        PureExpr::Variable(PureVariable("x")),
                    ))),
                    PureExpr::Operation(Box::new(PureOperation::Mul(
                        PureExpr::Variable(PureVariable("x")),
                        PureExpr::Operation(Box::new(PureOperation::Add(
                            PureExpr::Variable(PureVariable("x")),
                            PureExpr::Literal(PureLiteral::Sint(5)),
                        ))),
                    ))),
                ))),
            };
            println!("{bound:#}");
            assert_eq!(bound.to_string(), "for(x): 3*x > x*(x + 5)");
        }

        #[test]
        fn test_satisfies_precedence2() {
            use pure::*;
            let bound = TypeBound::Satisfies {
                var: "x",
                pred: PureExpr::Operation(Box::new(PureOperation::Gt(
                    PureExpr::Operation(Box::new(PureOperation::Mul(
                        PureExpr::Literal(PureLiteral::Sint(3)),
                        PureExpr::Variable(PureVariable("x")),
                    ))),
                    PureExpr::Operation(Box::new(PureOperation::Mul(
                        PureExpr::Variable(PureVariable("x")),
                        PureExpr::Operation(Box::new(PureOperation::Mul(
                            PureExpr::Variable(PureVariable("x")),
                            PureExpr::Literal(PureLiteral::Sint(5)),
                        ))),
                    ))),
                ))),
            };
            println!("{bound:#}");
            assert_eq!(bound.to_string(), "for(x): 3*x > x*x*5");
        }
    }

    // NOT PASSING
    mod checking {
        use super::*;
        use crate::analyzer::types::{StaticType, bounds::error::UnsatisfiedBoundError};

        #[test]
        fn test_match_one() {
            let bound = TypeBound::Combo(
                Combinator::One,
                vec![
                    TypeBound::Exactly(AbstractType::Byte),
                    TypeBound::Exactly(AbstractType::Char),
                    TypeBound::Exactly(AbstractType::Text),
                ],
            );
            let ty = AbstractType::Text;
            assert!(ty.satisfaction_of(&bound).is_passing());
        }

        #[test]
        fn test_bound_illegal() {
            let mut numeric = TypeBound::Combo(
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
                                            req: Some(Box::new(TypeBound::Exactly(
                                                AbstractType::Uint,
                                            ))),
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
            numeric.simplify();
            assert!(!numeric.is_legal());
        }

        #[test]
        fn test_bound_overlap() {
            let numeric = TypeBound::Combo(
                Combinator::One,
                vec![
                    TypeBound::Exactly(AbstractType::Uint),
                    TypeBound::Exactly(AbstractType::Sint),
                    TypeBound::Exactly(AbstractType::Flpt),
                ],
            );
            assert!(numeric.is_legal());
            let integral = TypeBound::Combo(
                Combinator::Any,
                vec![
                    TypeBound::Exactly(AbstractType::Byte),
                    TypeBound::Exactly(AbstractType::Uint),
                    TypeBound::Exactly(AbstractType::Sint),
                ],
            );
            assert!(integral.is_legal());
            let res = numeric.satisfaction_of(&integral);
            if let Ok(e) = UnsatisfiedBoundError::try_from(res) {
                println!("{e}");
                panic!("should be satisfied");
            }
        }
    }
}
