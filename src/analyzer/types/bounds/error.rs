use crate::{
    SYNTAX_STYLE_ANSI, Todo,
    analyzer::types::{
        AbstractType, FunShape,
        bounds::{Combinator, TypeBound},
    },
    highlight::style::StyleWrapper,
};
use std::fmt::Display;

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

type SatisfiedBound<'src> = BoundCheck<'src, SatisfiedBoundReason<'src>>;
type UnsatisfiedBound<'src> = BoundCheck<'src, UnsatisfiedBoundReason<'src>>;

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

#[derive(Debug, Clone, PartialEq)]
pub enum SatisfiedBoundReason<'src> {
    /// Satisfied [`TypeBound::Combo`] with [`Combinator::All`]. Every bound succeeded.
    All,
    /// Satisfied [`TypeBound::Combo`] with [`Combinator::Any`]. `fulfilled` succeeded.
    Any {
        fulfilled: Box<SatisfiedBound<'src>>,
    },
    /// Satisfied [`TypeBound::Combo`] with [`Combinator::One`]. `fulfilled` succeeded.
    One {
        fulfilled: Box<SatisfiedBound<'src>>,
    },
    /// Satisfied [`TypeBound::Not`]. `unfulfilled` failed as expected.
    Not {
        unfulfilled: Box<UnsatisfiedBound<'src>>,
    },
    /// Satisfied [`TypeBound::Exactly`]. `attempt` is the exact type.
    Exactly,
    /// Satisfied [`TypeBound::Has`]. `attempt` has the member.
    Has,
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
            Any { fulfilled } | One { fulfilled } => {
                Indent.fmt(f)?;
                writeln!(f, "one of these bounds is `{:#}`:", fulfilled.bound)?;
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            Not { unfulfilled } => {
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            All | Exactly | Has | Shape | Supports(_) | Satisfies(_) => {}
        }
        Indent.fmt(f)?;
        write!(f, "since the {} `{attempt:#}` ", attempt.kind_name())?;
        match reason {
            All => f.write_str("fulfills all of these"),

            Any {
                fulfilled: BoundCheck {
                    bound: fulfilled, ..
                },
            } => write!(f, "fulfills the bound `{fulfilled:#}`"),
            One {
                fulfilled: BoundCheck {
                    bound: fulfilled, ..
                },
            } => write!(f, "exclusively fulfills the bound `{fulfilled:#}`"),

            Not {
                unfulfilled: BoundCheck { bound: inner, .. },
            } => write!(f, "does not fulfill `{inner:#}`, which is being negated"),

            Exactly => write!(f, "is equivalent to `{bound:#}`"),

            Has => {
                let TypeBound::Has { member, .. } = bound else {
                    unreachable!()
                };
                write!(
                    f,
                    "has such a member, `{:#}`",
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
    /// Failed [`TypeBound::Combo`] with [`Combinator::All`]. `unfulfilled` failed.
    NotAll {
        unfulfilled: Box<UnsatisfiedBound<'src>>,
    },
    /// Failed [`TypeBound::Combo`] with [`Combinator::Any`]. Every bound failed.
    NotAny,
    /// Failed [`TypeBound::Combo`] with [`Combinator::One`]. Every bound failed.
    NotOne,
    /// Failed [`TypeBound::Combo`] with [`Combinator::One`]. Both `fulfilled.0` and `fulfilled.1` succeeded.
    OverOne {
        fulfilled: Box<(SatisfiedBound<'src>, SatisfiedBound<'src>)>,
    },
    /// Failed [`TypeBound::Not`]. `fulfilled` unexpectedly satisfied.
    NotNot {
        fulfilled: Box<SatisfiedBound<'src>>,
    },
    /// Failed [`TypeBound::Exactly`]. `bound` and `attempt` conflict.
    Unequal,
    /// Failed [`TypeBound::Has`]. `attempt` does not have the member `bound` requires.
    MissingMember,
    /// Failed [`TypeBound::Has`]. `attempt` has the member `bound` requires, but the member fails `unfulfilled`.
    MemberFails {
        unfulfilled: Box<UnsatisfiedBound<'src>>,
    },
    /// Failed [`TypeBound::Shape`]. `attempt` does not have the shape described by `bound`.
    WrongShape,
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
            NotAll { unfulfilled } => {
                Indent.fmt(f)?;
                writeln!(f, "one of these bounds is `{:#}`:", unfulfilled.bound)?;
                {
                    let _depth = LocalDepth::new();
                    unfulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            OverOne {
                fulfilled: (first, second),
            } => {
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

            NotNot { fulfilled } => {
                {
                    let _depth = LocalDepth::new();
                    fulfilled.fmt(f)?;
                }
                writeln!(f)?;
            }

            MemberFails { unfulfilled } => {
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

            NotAny | NotOne | Unequal | MissingMember | WrongShape | Unsupported(_)
            | Unsatisfies(_) => {}
        }
        Indent.fmt(f)?;
        write!(f, "since the {} `{attempt:#}` ", attempt.kind_name())?;
        match reason {
            NotAll {
                unfulfilled: BoundCheck { bound: failed, .. },
            } => {
                write!(f, "does not fulfill `{failed:#}`")
            }

            NotAny | NotOne => write!(f, "fulfills none of the bounds"),

            OverOne {
                fulfilled: (BoundCheck { bound: first, .. }, BoundCheck { bound: second, .. }),
            } => write!(
                f,
                "fulfills both `{first:#}` and `{second:#}`, thus failing exclusivity"
            ),

            NotNot {
                fulfilled: BoundCheck { bound: inner, .. },
            } => write!(f, "fulfills `{inner:#}`, which is being negated"),

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

            MemberFails { .. } => {
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

            // TODO: show what its shape IS (if it has one)
            WrongShape => f.write_str("cannot be called as such"),

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
                reason: UnsatisfiedBoundReason::OverOne {
                    fulfilled: Box::new((
                        SatisfiedBound {
                            bound: b,
                            attempt: x.clone(),
                            reason: SatisfiedBoundReason::Exactly,
                        },
                        SatisfiedBound {
                            bound: d,
                            attempt: x.clone(),
                            reason: SatisfiedBoundReason::Not {
                                unfulfilled: Box::new(UnsatisfiedBound {
                                    bound: c,
                                    attempt: x,
                                    reason: UnsatisfiedBoundReason::MissingMember,
                                }),
                            },
                        },
                    )),
                },
            },
        };
        println!("{error}");
    }
}
