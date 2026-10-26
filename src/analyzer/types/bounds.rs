use crate::{
    SYNTAX_STYLE_ANSI, Todo,
    analyzer::{
        static_env::EnvKey,
        types::{AbstractType, FunShape, TypeCompatible},
    },
    error::OpSide,
    highlight::style::StyleWrapper,
};
use std::fmt::Display;

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
    Satisfies(Todo),
}

impl Default for TypeBound<'_> {
    fn default() -> Self {
        Self::Combo(Combinator::All, Vec::new())
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
                    Combinator::Any => " or ", // can't be `|` because that's already used for fallible types
                    Combinator::One => "/",
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

            Self::Exactly(ty) => ty.fmt(f),

            Self::Has { member, req } => {
                SYNTAX_STYLE_ANSI.keyword.alt_style("has ").fmt(f)?;
                SYNTAX_STYLE_ANSI.variable.alt_style(member).fmt(f)?;
                if let Some(req) = req {
                    f.write_str(": ")?;
                    req.fmt(f)
                } else {
                    Ok(())
                }
            }

            Self::Shape(shape) => shape.fmt(f),

            Self::Supports { .. } => todo!(),

            Self::Satisfies(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Satisfaction {
    Deny = -1,
    #[default]
    Unclear = 0,
    Require = 1,
}

impl Satisfaction {
    pub const fn is_impossible(self) -> bool {
        matches!(self, Self::Deny)
    }

    pub const fn is_unclear(self) -> bool {
        matches!(self, Self::Unclear)
    }

    pub const fn is_guaranteed(self) -> bool {
        matches!(self, Self::Require)
    }

    pub const fn is_possible(self) -> bool {
        matches!(self, Self::Unclear | Self::Require)
    }
}

impl Display for Satisfaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Satisfaction::Deny => "impossible",
            Satisfaction::Unclear => "uncertain",
            Satisfaction::Require => "guaranteed",
        })
    }
}

impl<'src> TypeBound<'src> {
    // TODO: change to return a `Result<(), _>` to indicate which bound was broken
    pub fn is_satisfied_by<'a, T: TypeCompatible<'a>>(&self, ty: &T) -> bool {
        let res = match self {
            Self::Combo(kind, bounds) => {
                print!(
                    "`{self:#}` requires {} one of its bounds to be satisfied. ",
                    match kind {
                        Combinator::All => "every",
                        Combinator::Any => "at least",
                        Combinator::One => "exactly",
                    }
                );
                match kind {
                    Combinator::All => {
                        let failing = bounds.iter().find(|bound| !bound.is_satisfied_by(ty));
                        if let Some(failing) = failing {
                            print!(
                                "since `{ty:#}` does not satisfy the bound `{failing:#}` of `{self:#}`, "
                            );
                        } else {
                            print!("since `{ty:#}` satisfies every bound of `{self:#}`, ");
                        }
                        failing.is_none()
                    }
                    Combinator::Any => {
                        let passing = bounds.iter().find(|bound| bound.is_satisfied_by(ty));
                        if let Some(passing) = passing {
                            print!(
                                "since `{ty:#}` satisfies the bound `{passing:#}` of `{self:#}`, "
                            );
                        } else {
                            print!("since `{ty:#}` does not satisfy any bound of `{self:#}`, ");
                        }
                        passing.is_some()
                    }
                    Combinator::One => {
                        let mut iter = bounds.iter().filter(|bound| bound.is_satisfied_by(ty));
                        if let Some(first) = iter.next() {
                            let second = iter.next();
                            if let Some(second) = second {
                                print!(
                                    "since `{ty:#}` satisfies multiple bounds of `{self:#}`, both `{first:#}` and `{second:#}`, "
                                );
                            } else {
                                print!(
                                    "since `{ty:#}` satisfies the bound `{first:#}`, and no others, of `{self:#}`, "
                                );
                            }
                            second.is_none()
                        } else {
                            print!("since `{ty:#}` does not satisfy any bound of `{self:#}`, ");
                            false
                        }
                    }
                }
            }
            Self::Not(bound) => {
                let res = bound.is_satisfied_by(ty);
                print!(
                    "since `{ty:#}` {} `{bound:#}`, and `{self:#}` is the negation of `{bound:#}`, ",
                    if res { "satisfies" } else { "does not satisfy" }
                );
                !res
            }
            Self::Exactly(target) => {
                let res = ty.is_type_eq(target);
                print!(
                    "since `{ty:#}` is {}equal to `{target:#}`, ",
                    if res { "" } else { "not " }
                );
                res
            }
            Self::Has { member, req } => {
                let res = ty.has(member, req.as_deref());
                print!(
                    "since `{ty:#}` has {} member `{:#}` ",
                    if res { "a" } else { "no" },
                    SYNTAX_STYLE_ANSI.variable.alt_style(member)
                );
                if let Some(req) = req {
                    print!("satisfying `{req:#}`");
                }
                print!(", ");
                res
            }
            Self::Shape(shape) => {
                let res = ty.is_shape(shape);
                print!(
                    "since `{ty:#}` can{} be called with arguments of ",
                    if res { "" } else { "not" }
                );
                let mut has_prev = false;
                for param in &shape.params {
                    if has_prev {
                        print!(", ");
                    }
                    has_prev = true;
                    print!("`{param:#}`");
                }
                print!(" to produce a return of `{:#}`, ", shape.ret);
                res
            }
            Self::Supports { .. } => todo!("type bound needs environment access"),
            Self::Satisfies(_) => todo!("type bound needs code access"),
        };
        println!(
            "`{ty:#}` {} `{self:#}`.\n",
            if res { "satisfies" } else { "does not satisfy" }
        );
        res
    }

    pub fn satisfaction_of(&self, req: &TypeBound<'src>) -> Satisfaction {
        use Satisfaction::{Deny, Require, Unclear};
        let res = match self {
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
                    Combinator::All => bounds
                        .iter()
                        .find_map(|bound| match bound.satisfaction_of(req) {
                            x @ (Deny | Require) => {
                                print!("since `{bound:#}` satisfaction of `{req:#}` is {x}, `{req:#}` being be among the bounds of `{self:#}` is {x}. thus, ");
                                Some(x)
                            }
                            Unclear => None,
                        })
                        .unwrap_or(Unclear),

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
                    Combinator::Any | Combinator::One => bounds
                        .iter()
                        .find_map(|bound| match bound.satisfaction_of(req) {
                            x @ (Deny | Unclear) => {
                                print!("since `{bound:#}` satisfaction of `{req:#}` is {x}, and `{self:#}` only requires one bound to be satisfied, \
                                        it is possible `{bound:#}` may be the only satisfied bound. thus, ");
                                Some(x)
                            }
                            Require => None,
                        })
                        .unwrap_or(Require),
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
            Self::Not(bound) => {
                let x = bound.satisfaction_of(req);
                let y = match x {
                    Deny => Require,
                    Unclear => Unclear,
                    Require => Deny,
                };
                print!(
                    "since `{bound:#}` satisfaction of `{req:#}` is {x}, and `{bound:#}` is being negated, "
                );
                y
            }

            // being a type that satisfies the requirement clearly satisfies the requirement.
            //
            // because the type must be exactly this one, if this one does not satisfy the requirement,
            // the requirement cannot be satisfied in any case where the type must be this one.
            Self::Exactly(ty) => {
                let x = req.is_satisfied_by(ty);
                let y = if x { Require } else { Deny };
                print!(
                    "since `{self:#}` must be `{ty:#}`, and `{ty:#}` {} `{req:#}`, ",
                    if x { "satisfies" } else { "does not satisfy" }
                );
                y
            }

            Self::Has {
                member: my_member,
                req: my_req,
            } => match req {
                Self::Combo(_, _) | Self::Not(_) => todo!(
                    "combo could be an over-complex equivalent to the bound. ex: `T | !!(T + T)` = `T`"
                ),

                Self::Has {
                    member: their_member,
                    req: their_req,
                } => {
                    if my_member == their_member {
                        if let Some(their_req) = their_req.as_deref() {
                            if let Some(my_req) = my_req.as_deref() {
                                let res = my_req.satisfaction_of(their_req);
                                println!(
                                    "both `{req:#}` and `{self:#}` require a member `{their_member:#}` to exist. `{self:#}` requires that member to satisfy `{my_req:#}`, \
                                    and`{req:#}` requires that member to satisfy `{their_req:#}`. because `{their_member:#}` exists and its satisfaction of `{their_req:#}` is {res},"
                                );
                                res
                            } else {
                                print!(
                                    "both `{req:#}` and `{self:#}` require a member `{their_member:#}` to exist. however, `{self:#}` places no requirements on that member, \
                                    while `{req:#}` requires it to satisfy `{their_req:#}`. it is possible the type satisfies `{req:#}` \
                                    (for example: `{:#} {{ {their_member}: {their_req:#} }}`), \
                                    but it is also possible that it does not (for example: `{:#} {{ {my_member}: {:#} }}`). thus, ",
                                    SYNTAX_STYLE_ANSI.typename.alt_style("Foo"),
                                    SYNTAX_STYLE_ANSI.typename.alt_style("Bar"),
                                    // ew, an allocation just to make a printable
                                    TypeBound::Not(Box::new(their_req.clone()))
                                );
                                Unclear
                            }
                        } else {
                            print!(
                                "since both `{req:#}` and `{self:#}` requires a member `{their_member:#}` to exist, and `{req:#}` places no requirements on that member, "
                            );
                            Require
                        }
                    } else {
                        println!(
                            "since `{req:#}` requires a type to have a member {their_member}, but `{self:#}` makes no such requirement, \
                            it is possible the type satisfies `{req:#}` (for example: {:#} {{ {my_member}, {their_member} }} would satisfy both), \
                            but it is also possible that it does not (for example: {:#} {{ {my_member} }} would only satisfy `{self:#}`). thus,",
                            SYNTAX_STYLE_ANSI.typename.alt_style("Foo"),
                            SYNTAX_STYLE_ANSI.typename.alt_style("Bar"),
                        );
                        Unclear
                    }
                }

                // it is possible the type satisfies the requirement, but these bounds alone don't guarantee it.
                _ => Unclear,
            },

            Self::Shape(shape) => match req {
                Self::Combo(_, _) | Self::Not(_) => todo!(
                    "combo could be an over-complex equivalent to the bound. ex: `T | !!(T + T)` = `T`"
                ),

                // TODO: are there cases where a function shape can be equivalent but not equal?
                Self::Shape(req_shape) if shape == req_shape => Require,

                // it is possible the type satisfies the requirement, but these bounds alone don't guarantee it.
                _ => Unclear,
            },

            Self::Supports { .. } => todo!("`supports` needs fleshing out"),
            Self::Satisfies(_) => todo!("`satisfies` needs fleshing out"),
        };
        println!("`{self:#}` satisfaction of `{req:#}` is {res}.\n");
        res
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
                assert_eq!(bound.to_string(), expect);
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
                assert_eq!(bound.to_string(), "bool",);
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
            assert_eq!(bound.to_string(), "bool + uint",);
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
            assert_eq!(bound.to_string(), "bool + uint + sint",);
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
            assert_eq!(bound.to_string(), "bool or uint",);
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
            assert_eq!(bound.to_string(), "bool or uint or sint",);
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
            assert_eq!(bound.to_string(), "bool/uint",);
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
            assert_eq!(bound.to_string(), "bool/uint/sint",);
        }

        #[test]
        fn test_combo_has() {
            let bound = TypeBound::Has {
                member: "x",
                req: None,
            };
            assert_eq!(bound.to_string(), "has x",);
        }

        #[test]
        fn test_combo_has_exact() {
            let bound = TypeBound::Has {
                member: "x",
                req: Some(Box::new(TypeBound::Exactly(AbstractType::Bool))),
            };
            assert_eq!(bound.to_string(), "has x: bool",);
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
            assert_eq!(bound.to_string(), "has x: bool + uint",);
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
            assert_eq!(bound.to_string(), "(has x: bool + uint) + sint",);
        }

        #[test]
        fn test_negative_unwrapped() {
            let bound = TypeBound::Not(Box::new(TypeBound::Exactly(AbstractType::Bool)));
            assert_eq!(bound.to_string(), "!bool",);
        }

        #[test]
        fn test_double_negative_unwrapped() {
            let bound = TypeBound::Not(Box::new(TypeBound::Not(Box::new(TypeBound::Exactly(
                AbstractType::Bool,
            )))));
            assert_eq!(bound.to_string(), "!!bool",);
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
            assert_eq!(bound.to_string(), "!(bool + uint)",);
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
            assert_eq!(bound.to_string(), "!!(bool + uint)",);
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
            assert_eq!(bound.to_string(), "!(!bool + uint)",);
        }
    }

    mod checking {
        use super::*;
        use crate::analyzer::types::StaticType;

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
            assert!(bound.is_satisfied_by(&ty));
        }

        #[test]
        fn test_bound_reflexive() {
            let numeric = TypeBound::Combo(
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
            assert!(numeric.satisfaction_of(&numeric).is_possible());
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
            let integral = TypeBound::Combo(
                Combinator::One,
                vec![
                    TypeBound::Exactly(AbstractType::Byte),
                    TypeBound::Exactly(AbstractType::Uint),
                    TypeBound::Exactly(AbstractType::Sint),
                ],
            );
            assert!(numeric.satisfaction_of(&integral).is_possible());
        }
    }
}
