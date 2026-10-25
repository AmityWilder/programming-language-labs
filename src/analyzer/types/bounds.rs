use crate::{
    Todo,
    analyzer::{
        static_env::EnvKey,
        types::{AbstractType, FunShape, TypeCompatible},
    },
    error::OpSide,
};

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

impl std::fmt::Display for TypeBound<'_> {
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
                write!(f, "({bound})")
            }
        }

        match self {
            Self::Combo(kind, bounds) => {
                let sep = match kind {
                    Combinator::All => " + ",
                    Combinator::Any => " U ", // can't be `|` because that's already used for fallible types
                    Combinator::One => "/",
                };
                let mut has_prev = false;
                for bound in bounds {
                    if has_prev {
                        f.write_str(sep)?;
                    }
                    has_prev = true;
                    parenthesize_if_needed(bound, f)?;
                }
                Ok(())
            }

            Self::Not(bound) => {
                f.write_str("!")?;
                parenthesize_if_needed(bound, f)
            }

            Self::Exactly(ty) => ty.fmt(f),

            Self::Has { member, req } => {
                write!(f, "has {member}")?;
                if let Some(req) = req {
                    write!(f, ": {req}")
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

impl<'src> TypeBound<'src> {
    // TODO: change to return a `Result<(), _>` to indicate which bound was broken
    pub fn is_satisfied_by<'a, T: TypeCompatible<'a>>(&self, ty: &T) -> bool {
        match self {
            Self::Combo(kind, bounds) => match kind {
                Combinator::All => bounds.iter().all(|bound| bound.is_satisfied_by(ty)),
                Combinator::Any => bounds.iter().any(|bound| bound.is_satisfied_by(ty)),
                Combinator::One => {
                    let mut iter = bounds.iter().filter(|bound| bound.is_satisfied_by(ty));
                    // must have a first
                    iter.next().is_some()
                    // must not have a second
                        && iter.next().is_none()
                }
            },
            Self::Not(bound) => !bound.is_satisfied_by(ty),
            Self::Exactly(target) => ty.is_type_eq(target),
            Self::Has { member, req } => ty.has(member, req.as_deref()),
            Self::Shape(shape) => ty.is_shape(shape),
            Self::Supports { .. } => todo!("type bound needs environment access"),
            Self::Satisfies(_) => todo!("type bound needs code access"),
        }
    }

    pub fn satisfaction_of(&self, req: &TypeBound<'src>) -> Satisfaction {
        use Satisfaction::{Deny, Require, Unclear};
        let res = match self {
            Self::Combo(kind, bounds) => {
                let mut iter = bounds.iter().map(|bound| bound.satisfaction_of(req));
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
                    Combinator::All => iter
                        .find(|x| matches!(x, Deny | Require))
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
                    Combinator::Any | Combinator::One => iter
                        .find(|x| matches!(x, Deny | Unclear))
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
            Self::Not(bound) => match bound.satisfaction_of(req) {
                Deny => Require,
                Unclear => Unclear,
                Require => Deny,
            },

            // being a type that satisfies the requirement clearly satisfies the requirement.
            //
            // because the type must be exactly this one, if this one does not satisfy the requirement,
            // the requirement cannot be satisfied in any case where the type must be this one.
            Self::Exactly(ty) => {
                if req.is_satisfied_by(ty) {
                    Require
                } else {
                    Deny
                }
            }

            Self::Has {
                member: my_member,
                req: my_req,
            } => match req {
                Self::Combo(_, _) | Self::Not(_) => todo!(
                    "combo could be an over-complex equivalent to the bound. ex: `T | !!(T + T)` = `T`"
                ),

                Self::Has { member, req }
                    if my_member == member
                        && req.as_deref().is_none_or(|req| {
                            my_req
                                .as_deref()
                                .is_some_and(|my_req| my_req.satisfaction_of(req) == Require)
                        }) =>
                {
                    Require
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
        println!(
            "{self} satisfaction of {req} is {}",
            match res {
                Deny => "impossible",
                Unclear => "unclear",
                Require => "guaranteed",
            }
        );
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
            assert_eq!(bound.to_string(), "bool U uint",);
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
            assert_eq!(bound.to_string(), "bool U uint U sint",);
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
