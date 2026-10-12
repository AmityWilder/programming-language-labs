use crate::{
    error::{ContextError, OpSide},
    grammar::ast::{Binary, Expr, Unary},
    scanner::token::{
        punc::Punctuation,
        value::{Keyword, LexValue},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PpValue<'src> {
    // types (for size operations)
    Bool,
    Sint,
    Uint,
    Frac,
    Char,
    Text,

    // values (for logic or comparison)
    BoolLiteral(bool),
    UIntLiteral(usize),
    SIntLiteral(isize),

    // macro name (only useable in "exists" operation)
    Macro(&'src str),
}

pub fn preproc_eval<'src, F>(
    expr: Expr<'src>,
    mexists: &mut F,
) -> Result<PpValue<'src>, ContextError<'src>>
where
    F: FnMut(&str) -> bool,
{
    // TODO: Some operations are valid for Expr but not in preprocessing
    match expr {
        Expr::Ternary(_inner) => todo!(),

        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = *inner;

            match op.val {
                LexValue::Punctuation(Punctuation::Add) => {
                    match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                        (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                            Ok(PpValue::UIntLiteral(l.saturating_add(r)))
                        }
                        (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                            Ok(PpValue::SIntLiteral(l.saturating_add(r)))
                        }
                        _ => unimplemented!("incompatible"), // TODO: change this to an error
                    }
                }

                LexValue::Punctuation(Punctuation::SubNeg) => {
                    match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                        (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                            Ok(PpValue::UIntLiteral(l.saturating_sub(r)))
                        }
                        (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                            Ok(PpValue::SIntLiteral(l.saturating_sub(r)))
                        }
                        _ => unimplemented!("incompatible"), // TODO: change this to an error
                    }
                }

                LexValue::Punctuation(Punctuation::Mul) => {
                    match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                        (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                            Ok(PpValue::UIntLiteral(l.saturating_mul(r)))
                        }
                        (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                            Ok(PpValue::SIntLiteral(l.saturating_mul(r)))
                        }
                        _ => unimplemented!("incompatible"), // TODO: change this to an error
                    }
                }

                LexValue::Punctuation(Punctuation::Div) => {
                    match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                        (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                            Ok(PpValue::UIntLiteral(l.checked_div(r).unwrap_or(usize::MAX)))
                        }
                        (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                            Ok(PpValue::SIntLiteral(l.checked_div(r).unwrap_or(isize::MAX)))
                        }
                        _ => unimplemented!("incompatible"), // TODO: change this to an error
                    }
                }

                LexValue::Punctuation(Punctuation::Rem) => {
                    match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                        (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                            Ok(PpValue::UIntLiteral(l.checked_rem(r).unwrap_or(0)))
                        }
                        (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                            Ok(PpValue::SIntLiteral(l.checked_rem(r).unwrap_or(0)))
                        }
                        _ => unimplemented!("incompatible"), // TODO: change this to an error
                    }
                }

                LexValue::Punctuation(Punctuation::Eq) => {
                    Ok(PpValue::BoolLiteral(
                        match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => l == r,
                            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => l == r,
                            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => l == r,
                            _ => unimplemented!("incompatible"), // TODO: change this to an error
                        },
                    ))
                }
                LexValue::Punctuation(Punctuation::Ne) => {
                    Ok(PpValue::BoolLiteral(
                        match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => l != r,
                            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => l != r,
                            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => l != r,
                            _ => unimplemented!("incompatible"), // TODO: change this to an error
                        },
                    ))
                }
                LexValue::Punctuation(Punctuation::Lt) => {
                    Ok(PpValue::BoolLiteral(
                        match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => l < r,
                            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => l < r,
                            _ => unimplemented!("incompatible"), // TODO: change this to an error
                        },
                    ))
                }
                LexValue::Punctuation(Punctuation::Le) => {
                    Ok(PpValue::BoolLiteral(
                        match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => l <= r,
                            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => l <= r,
                            _ => unimplemented!("incompatible"), // TODO: change this to an error
                        },
                    ))
                }
                LexValue::Punctuation(Punctuation::Gt) => {
                    Ok(PpValue::BoolLiteral(
                        match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => l > r,
                            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => l > r,
                            _ => unimplemented!("incompatible"), // TODO: change this to an error
                        },
                    ))
                }
                LexValue::Punctuation(Punctuation::Ge) => {
                    Ok(PpValue::BoolLiteral(
                        match (preproc_eval(lhs, mexists)?, preproc_eval(rhs, mexists)?) {
                            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => l >= r,
                            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => l >= r,
                            _ => unimplemented!("incompatible"), // TODO: change this to an error
                        },
                    ))
                }

                _ => unimplemented!("not a preproc operator"),
            }
        }

        Expr::Unary(inner) => {
            let Unary { op, operand, side } = *inner;
            match side {
                OpSide::Left => unimplemented!(),

                OpSide::Right => match op.val {
                    LexValue::Punctuation(Punctuation::SubNeg) => {
                        match preproc_eval(operand, mexists)? {
                            PpValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(x.saturating_neg())),
                            _ => unimplemented!("undefined"), // TODO: change this to an error
                        }
                    }

                    LexValue::Punctuation(Punctuation::Not) => {
                        match preproc_eval(operand, mexists)? {
                            PpValue::UIntLiteral(x) => Ok(PpValue::UIntLiteral(!x)),
                            PpValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(!x)),
                            PpValue::BoolLiteral(x) => Ok(PpValue::BoolLiteral(!x)),
                            _ => unimplemented!("undefined"), // TODO: change this to an error
                        }
                    }

                    LexValue::Punctuation(Punctuation::Exists) => {
                        Ok(PpValue::BoolLiteral(
                            match preproc_eval(operand, mexists)? {
                                PpValue::Macro(x) => mexists(x),
                                _ => unimplemented!("undefined"), // TODO: change this to an error
                            },
                        ))
                    }

                    _ => unimplemented!("not a preproc operator"), // TODO: change this to an error
                },
            }
        }

        Expr::Literal(inner) => match inner.val {
            LexValue::UIntLiteral(x) => Ok(PpValue::UIntLiteral(x)),
            LexValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(x)),
            LexValue::BoolLiteral(x) => Ok(PpValue::BoolLiteral(x)),
            LexValue::Macro => Ok(PpValue::Macro(inner.lex)),
            _ => unimplemented!("not a preproc literal"), // TODO: change this to an error
        },

        Expr::Grouping(inner) => preproc_eval(inner.expr, mexists),

        Expr::Type(inner) => {
            // TODO: Make this an error
            assert_eq!(
                inner.or_ty, None,
                "cannot have fallible conversions in preprocessing"
            );
            match inner.name.val {
                LexValue::Keyword(Keyword::Bool) => Ok(PpValue::Bool),
                LexValue::Keyword(Keyword::Sint) => Ok(PpValue::Sint),
                LexValue::Keyword(Keyword::Uint) => Ok(PpValue::Uint),
                LexValue::Keyword(Keyword::Frac) => Ok(PpValue::Frac),
                LexValue::Keyword(Keyword::Char) => Ok(PpValue::Char),
                LexValue::Keyword(Keyword::Text) => Ok(PpValue::Text),
                _ => unimplemented!("unsupported preprocessor type"), // TODO: make this an error
            }
        }
    }
}
