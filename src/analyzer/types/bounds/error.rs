use crate::{
    SYNTAX_STYLE_ANSI, Todo,
    analyzer::types::{
        AbstractType, FunShape,
        bounds::{Combinator, TypeBound},
    },
    highlight::style::StyleWrapper,
};
use std::fmt::Display;

impl TypeBound<'_> {
    fn write_requirement(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the bound `{self:#}` requires ")?;
        match self {
            Self::Combo(kind, _) => {
                f.write_str(match kind {
                    Combinator::All => "every",
                    Combinator::Any => "at least",
                    Combinator::One => "exactly",
                })?;
                f.write_str(" one of its bounds to be fulfilled")
            }

            Self::Not(inner) => write!(f, "its bound `{inner:#}` to be unfulfilled"),

            Self::Exactly(ty) => write!(f, "exactly `{ty:#}`"),

            Self::Has { member, req } => {
                write!(
                    f,
                    "a member `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )?;
                if let Some(req) = req {
                    write!(f, " satisfying `{req:#}`")?;
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
                write!(f, "for an output of `{ret:#}`")
            }

            Self::Supports { .. } => todo!(),

            Self::Satisfies(_) => todo!(),
        }
    }
}

mod local_depth {
    use std::cell::Cell;

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
}
use local_depth::{Indent, LocalDepth};

#[derive(Debug, Clone, PartialEq)]
pub enum CheckedType<'src> {
    Abstract(AbstractType<'src>),
    Bound(TypeBound<'src>),
}

impl CheckedType<'_> {
    const fn kind_name(&self) -> &'static str {
        match self {
            Self::Abstract(_) => "type",
            Self::Bound(_) => "bound",
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

#[derive(Debug, Clone, PartialEq)]
pub struct BoundCheck<'src, T> {
    // TODO: having EACH NODE deep-clone the ENTIRE bound below that point could get really expensive
    pub bound: TypeBound<'src>,
    pub attempt: CheckedType<'src>,
    pub reason: T,
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
}

impl Display for RequiredBound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use RequiredBoundReason::*;
        let Self {
            bound,
            attempt,
            reason,
        } = self;
        Indent.fmt(f)?;
        bound.write_requirement(f)?;
        writeln!(f, ".")?;
        // recursive
        match reason {
            OneOfAll(bound_check) => {
                // TODO
            }

            Exactly(bound_check) => {
                // TODO
            }

            HasMatching(bound_check) => {
                // TODO
            }

            AllOfAny | AllOfOne | Has | Shape => {}
        }
        Indent.fmt(f)?;
        write!(f, "since the {} `{attempt:#}` ", attempt.kind_name())?;
        match reason {
            OneOfAll(bound_check) => {
                write!(f, "{bound_check:#}") // TODO
            }

            AllOfAny => {
                Ok(()) // TODO
            }

            AllOfOne => {
                Ok(()) // TODO
            }

            Exactly(bound_check) => {
                Ok(()) // TODO
            }

            Has => {
                Ok(()) // TODO
            }

            HasMatching(bound_check) => {
                Ok(()) // TODO
            }

            Shape => {
                Ok(()) // TODO
            }
        }?;
        write!(
            f,
            ", the {} `{attempt:#}` directly contradicts the bound `{bound:#}`.",
            attempt.kind_name()
        )
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
    OneOnlyAllowed(Box<AllowedBound<'src>>),
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
}

impl Display for DeniedBound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DeniedBoundReason::*;
        let Self {
            bound,
            attempt,
            reason,
        } = self;
        Indent.fmt(f)?;
        bound.write_requirement(f)?;
        writeln!(f, ".")?;
        // recursive
        match reason {
            OneOfAll(denial) => {
                // TODO
            }

            OneOfAny(denial) => {
                // TODO
            }

            OneOfOne(denial) => {
                // TODO
            }

            ExactlyUnsatisfying(unsatisfied) => {
                // TODO
            }

            CantHave(denial) => {
                // TODO
            }

            HasConflicting(denial) => {
                // TODO
            }
        }
        Indent.fmt(f)?;
        write!(f, "since the {} `{attempt:#}` ", attempt.kind_name())?;
        match reason {
            OneOfAll(BoundCheck { bound: denied, .. }) => {
                write!(f, "`{denied:#}`") // TODO
            }

            OneOfAny(BoundCheck { bound: denied, .. }) => {
                Ok(()) // TODO
            }

            OneOfOne(BoundCheck { bound: denied, .. }) => {
                Ok(()) // TODO
            }

            ExactlyUnsatisfying(BoundCheck {
                bound: unsatisfied, ..
            }) => {
                Ok(()) // TODO
            }

            CantHave(BoundCheck { bound: denied, .. }) => {
                Ok(()) // TODO
            }

            HasConflicting(BoundCheck { bound: denied, .. }) => {
                Ok(()) // TODO
            }
        }?;
        write!(
            f,
            ", the {} `{attempt:#}` directly contradicts the bound `{bound:#}`.",
            attempt.kind_name()
        )
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
    /// Satisfied [`TypeBound::Satisfies`].
    Satisfies(Todo),
}

impl Display for SatisfiedBound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SatisfiedBoundReason::*;
        let Self {
            bound,
            attempt,
            reason,
        } = self;
        Indent.fmt(f)?;
        bound.write_requirement(f)?;
        writeln!(f, ".")?;
        // recursive
        match reason {
            Required(requirement) => {
                {
                    let _depth = LocalDepth::new();
                    requirement.fmt(f)?;
                }
                writeln!(f)?;
            }

            Any(fulfilled) | One(fulfilled) => {
                Indent.fmt(f)?;
                writeln!(f, "one of these bounds is `{:#}`:", fulfilled.bound)?;
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            Not(unfulfilled) => {
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            HasMatching(fulfilled) => {
                let TypeBound::Has { member, req: _ } = fulfilled.bound else {
                    unreachable!()
                };
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "a member `{:#}` is present:",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )?;
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            All | Exactly | Has | Shape | Supports(_) | Satisfies(_) => {}
        }
        Indent.fmt(f)?;
        write!(f, "since the {} `{attempt:#}` ", attempt.kind_name())?;
        match reason {
            Required(BoundCheck {
                bound: requirement, ..
            }) => write!(f, "requires `{requirement:#}` by definition"),

            All => f.write_str("fulfills all of these"),

            Any(BoundCheck {
                bound: fulfilled, ..
            }) => write!(f, "fulfills the bound `{fulfilled:#}`"),

            One(BoundCheck {
                bound: fulfilled, ..
            }) => write!(f, "exclusively fulfills the bound `{fulfilled:#}`"),

            Not(BoundCheck { bound: inner, .. }) => {
                write!(f, "does not fulfill `{inner:#}`, which is being negated")
            }

            Exactly => write!(f, "is equivalent to `{bound:#}`"),

            Has => {
                let TypeBound::Has { member, req: None } = bound else {
                    unreachable!()
                };
                write!(
                    f,
                    "has a member, `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )
            }

            HasMatching(BoundCheck { bound: inner, .. }) => {
                let TypeBound::Has {
                    member,
                    req: Some(_),
                } = bound
                else {
                    unreachable!()
                };
                write!(
                    f,
                    "has a member, `{:#}`, satisfying `{inner:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )
            }

            Shape => f.write_str("can be called as such"),

            Supports(_) => todo!(),

            Satisfies(_) => todo!(),
        }?;
        write!(
            f,
            ", the {} `{attempt:#}` satisfies the bound `{bound:#}`.",
            attempt.kind_name()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnsatisfiedBoundReason<'src> {
    /// Failed a [`TypeBound`] through a bound that denies it
    Denied(DeniedBound<'src>),
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
    /// Failed [`TypeBound::Satisfies`].
    Unsatisfies(Todo),
}

// TODO: indentation
impl Display for UnsatisfiedBound<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UnsatisfiedBoundReason::*;
        let Self {
            bound,
            attempt,
            reason,
        } = self;
        Indent.fmt(f)?;
        bound.write_requirement(f)?;
        writeln!(f, ".")?;
        // recursive
        match reason {
            Denied(denial) => {
                {
                    let _depth = LocalDepth::new();
                    denial.fmt(f)?;
                }
                writeln!(f)?;
            }

            NotAll(unfulfilled) => {
                Indent.fmt(f)?;
                writeln!(f, "one of these bounds is `{:#}`:", unfulfilled.bound)?;
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            OverOne((first, second)) => {
                Indent.fmt(f)?;
                writeln!(f, "one of these bounds is `{:#}`:", first.bound)?;
                {
                    let _depth = LocalDepth::new();
                    first.fmt(f)?;
                }
                writeln!(f)?;
                Indent.fmt(f)?;
                writeln!(f, "another is `{:#}`:", second.bound)?;
                {
                    let _depth = LocalDepth::new();
                    second.fmt(f)?;
                }
                writeln!(f)?;
            }

            NotNot(fulfilled) => {
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            MemberFails(unfulfilled) => {
                let TypeBound::Has { member, .. } = bound else {
                    unreachable!()
                };
                Indent.fmt(f)?;
                writeln!(
                    f,
                    "a member `{:#}` exists:",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member),
                )?;
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            NotAny
            | NotOne
            | Unequal
            | MissingMember
            | MissingShape
            | ShapeFails { .. }
            | Unsupported(_)
            | Unsatisfies(_) => {}
        }
        Indent.fmt(f)?;
        write!(f, "since the {} `{attempt:#}` ", attempt.kind_name())?;
        match reason {
            Denied(BoundCheck { bound: failed, .. }) => {
                write!(f, "contradicts `{failed:#}` by definition")
            }

            NotAll(BoundCheck { bound: failed, .. }) => {
                write!(f, "does not fulfill `{failed:#}`")
            }

            NotAny | NotOne => write!(f, "fulfills none of the bounds"),

            OverOne((BoundCheck { bound: first, .. }, BoundCheck { bound: second, .. })) => write!(
                f,
                "fulfills both `{first:#}` and `{second:#}`, thus failing exclusivity"
            ),

            NotNot(BoundCheck { bound: inner, .. }) => {
                write!(f, "fulfills `{inner:#}`, which is being negated")
            }

            Unequal => write!(f, "is inequivalent to `{bound:#}`"),

            MissingMember => {
                let TypeBound::Has { member, .. } = bound else {
                    unreachable!()
                };
                write!(
                    f,
                    "has no member `{:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                )
            }

            MemberFails(_) => {
                let TypeBound::Has {
                    member,
                    req: Some(req),
                } = bound
                else {
                    unreachable!()
                };
                write!(
                    f,
                    "has the member `{0:#}`, but `{0:#}` does not fulfill `{req:#}`",
                    SYNTAX_STYLE_ANSI.variable.alt_style(member),
                )
            }

            MissingShape => f.write_str("cannot be called"),

            // TODO: show what its shape *is*
            ShapeFails(actual) => write!(f, "is instead called as `{actual:#}`"),

            Unsupported(_) => todo!(),

            Unsatisfies(_) => todo!(),
        }?;
        write!(
            f,
            ", the {} `{attempt:#}` does not satisfy the bound `{bound:#}`.",
            attempt.kind_name()
        )
    }
}

// TBD: what else would be put in here?
#[derive(Debug, Clone, PartialEq)]
pub struct UnsatisfiedBoundError<'src> {
    pub diagnosis: UnsatisfiedBound<'src>,
}

impl Display for UnsatisfiedBoundError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { diagnosis } = self;
        let BoundCheck { bound, attempt, .. } = diagnosis;
        writeln!(
            f,
            "the {} `{attempt:#}` does not satisfy the bound `{bound:#}`.",
            diagnosis.attempt.kind_name(),
        )?;
        writeln!(f, "observe:")?;
        diagnosis.fmt(f)
    }
}

impl std::error::Error for UnsatisfiedBoundError<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

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
        println!("{error}");
    }
}
