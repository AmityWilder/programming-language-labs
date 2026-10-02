//! Code execution

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::{ContextError, ErrorType, IntValue, OpSide, OverflowError, TargetTy},
    grammar::{Binary, Expr, Unary},
    scanner::token::{
        Token,
        punc::Punctuation,
        value::{CharLiteral, LexValue},
    },
};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
enum OpError {
    Incompatible(ValueType, ValueType),
    Unsupported(ValueType),
    FailedConversion {
        e: std::num::TryFromIntError,
        target_ty: TargetTy,
        value: IntValue,
    },
    OverflowAdd(IntValue, IntValue),
    OverflowSub(IntValue, IntValue),
    OverflowMul(IntValue, IntValue),
    OverflowPow(IntValue, IntValue),
    OverflowNeg(isize),
    UNeg,
    DivByZero,
}

impl OpError {
    fn binary<'src>(
        self,
        source: &'src str,
        op: &Token<'src>,
        lhs: &Expr<'src>,
        rhs: &Expr<'src>,
    ) -> ContextError<'src> {
        match self {
            OpError::Incompatible(l_ty, r_ty) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Incompatible {
                    op: match op.val {
                        LexValue::Punctuation(punc) => punc,
                        _ => unimplemented!(),
                    },
                    l_range: lhs.range(source),
                    r_range: rhs.range(source),
                    l_ty,
                    r_ty,
                },
            ),
            // Assumes the only conversion failure can happen on the right hand side
            OpError::FailedConversion {
                e,
                target_ty,
                value,
            } => ContextError::error(
                source,
                Some(rhs.range(source)),
                rhs.macro_range(source),
                ErrorType::FailedConvert {
                    value,
                    op: match op.val {
                        LexValue::Punctuation(punc) => punc,
                        _ => unimplemented!(),
                    },
                    op_range: op.lex_range(source),
                    side: OpSide::Right,
                    target_ty,
                    e,
                },
            ),
            OpError::DivByZero => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::DivByZero {
                    zero: rhs.range(source),
                },
            ),
            OpError::OverflowAdd(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::Add {
                    l_range: lhs.range(source),
                    r_range: rhs.range(source),
                    l_value: l,
                    r_value: r,
                }),
            ),
            OpError::OverflowSub(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::Sub {
                    l_range: lhs.range(source),
                    r_range: rhs.range(source),
                    l_value: l,
                    r_value: r,
                }),
            ),
            OpError::OverflowMul(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::Mul {
                    l_range: lhs.range(source),
                    r_range: rhs.range(source),
                    l_value: l,
                    r_value: r,
                }),
            ),
            OpError::OverflowPow(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::Pow {
                    l_range: lhs.range(source),
                    r_range: rhs.range(source),
                    l_value: l,
                    r_value: r,
                }),
            ),

            OpError::Unsupported(_) | OpError::UNeg | OpError::OverflowNeg(_) => {
                unimplemented!("not valid for binary")
            }
        }
    }

    fn unary<'src>(
        self,
        source: &'src str,
        op: &Token<'src>,
        punc: Punctuation,
        rhs: &Expr<'src>,
    ) -> ContextError<'src> {
        match self {
            Self::Unsupported(r_ty) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Unsupported {
                    op: punc,
                    r_range: rhs.range(source),
                    r_ty,
                },
            ),
            // assumes all unary operators are prefix ops
            Self::FailedConversion {
                e,
                target_ty,
                value,
            } => ContextError::error(
                source,
                Some(rhs.range(source)),
                rhs.macro_range(source),
                ErrorType::FailedConvert {
                    value,
                    op: match op.val {
                        LexValue::Punctuation(Punctuation::Sub) => Punctuation::Neg,
                        LexValue::Punctuation(punc) => punc,
                        _ => unimplemented!(),
                    },
                    op_range: op.lex_range(source),
                    side: OpSide::Right,
                    target_ty,
                    e,
                },
            ),
            Self::OverflowNeg(r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::Neg {
                    r_value: r,
                    r_range: rhs.range(source),
                }),
            ),
            Self::UNeg => ContextError::token_error(source, Some(*op), ErrorType::UnsignedNeg),

            Self::Incompatible(_, _)
            | Self::DivByZero
            | Self::OverflowAdd(_, _)
            | Self::OverflowSub(_, _)
            | Self::OverflowMul(_, _)
            | Self::OverflowPow(_, _) => unimplemented!("not valid for unary"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Bool,
    UInt,
    SInt,
    Frac,
    Char,
    Str,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Bool => "bool",
            Self::UInt => "uint",
            Self::SInt => "sint",
            Self::Frac => "frac",
            Self::Char => "char",
            Self::Str => "str",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunValue {
    Bool(bool),
    UInt(usize),
    SInt(isize),
    Frac(f64),
    Char(char),
    Str(String),
}

impl RunValue {
    pub const fn as_type(&self) -> ValueType {
        match self {
            Self::Bool(_) => ValueType::Bool,
            Self::UInt(_) => ValueType::UInt,
            Self::SInt(_) => ValueType::SInt,
            Self::Frac(_) => ValueType::Frac,
            Self::Char(_) => ValueType::Char,
            Self::Str(_) => ValueType::Str,
        }
    }

    /// Returns [`None`] if compatible but incomparable
    /// (i.e. a non-existent `NotEqual` variant of [`std::cmp::Ordering`]).
    fn cmp(&self, other: &Self) -> Result<Option<std::cmp::Ordering>, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Some(l.cmp(r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Some(l.cmp(r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Some(l.cmp(r))),
            (Self::Frac(l), Self::Frac(r)) => Ok(l.partial_cmp(r)),
            (Self::Char(l), Self::Char(r)) => Ok(Some(l.cmp(r))),
            (Self::Str(l), Self::Str(r)) => Ok(Some(l.cmp(r))),

            // TODO: coersions?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn add(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_add(r)
                .map(Self::UInt)
                .ok_or(OpError::OverflowAdd(IntValue::UInt(l), IntValue::UInt(r))),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_add(r)
                .map(Self::SInt)
                .ok_or(OpError::OverflowAdd(IntValue::SInt(l), IntValue::SInt(r))),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l + r)),

            (Self::Str(l), Self::Bool(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::UInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::SInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Frac(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Char(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Bool(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::UInt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::SInt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Frac(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Char(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Str(l), Self::Str(r)) => Ok(Self::Str(l + &r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn sub(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_sub(r)
                .map(Self::UInt)
                .ok_or(OpError::OverflowSub(IntValue::UInt(l), IntValue::UInt(r))),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_sub(r)
                .map(Self::SInt)
                .ok_or(OpError::OverflowSub(IntValue::SInt(l), IntValue::SInt(r))),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l - r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn mul(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_mul(r)
                .map(Self::UInt)
                .ok_or(OpError::OverflowMul(IntValue::UInt(l), IntValue::UInt(r))),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_mul(r)
                .map(Self::SInt)
                .ok_or(OpError::OverflowMul(IntValue::SInt(l), IntValue::SInt(r))),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l * r)),

            // TODO: coersions?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn div(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_div(r).map(Self::UInt).ok_or(OpError::DivByZero)
            }
            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_div(r).map(Self::SInt).ok_or(OpError::DivByZero)
            }
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l / r)),

            // TODO: coersions?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn rem(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_rem(r).map(Self::UInt).ok_or(OpError::DivByZero)
            }
            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_rem(r).map(Self::SInt).ok_or(OpError::DivByZero)
            }
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l / r)),

            // TODO: coersions?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn pow(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_pow(u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?)
                .map(Self::UInt)
                .ok_or(OpError::OverflowPow(IntValue::UInt(l), IntValue::UInt(r))),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_pow(u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?)
                .map(Self::SInt)
                .ok_or(OpError::OverflowPow(IntValue::SInt(l), IntValue::SInt(r))),
            (Self::Frac(l), Self::UInt(r)) => {
                Ok(Self::Frac(l.powi(i32::try_from(r).map_err(|e| {
                    OpError::FailedConversion {
                        e,
                        target_ty: TargetTy::S32,
                        value: IntValue::UInt(r),
                    }
                })?)))
            }
            (Self::Frac(l), Self::SInt(r)) => {
                Ok(Self::Frac(l.powi(i32::try_from(r).map_err(|e| {
                    OpError::FailedConversion {
                        e,
                        target_ty: TargetTy::S32,
                        value: IntValue::SInt(r),
                    }
                })?)))
            }
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l.powf(r))),

            // TODO: coersions?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn shl(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l.unbounded_shl(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(l.unbounded_shl(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(l.unbounded_shl(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l.unbounded_shl(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn shr(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l.unbounded_shr(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(l.unbounded_shr(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(l.unbounded_shr(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l.unbounded_shr(
                u32::try_from(r).map_err(|e| OpError::FailedConversion {
                    e,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn not(self) -> Result<Self, OpError> {
        match self {
            Self::Bool(r) => Ok(Self::Bool(!r)),
            Self::UInt(r) => Ok(Self::UInt(!r)),
            Self::SInt(r) => Ok(Self::SInt(!r)),

            // TODO: other types
            r => Err(OpError::Unsupported(r.as_type())),
        }
    }

    fn neg(self) -> Result<Self, OpError> {
        match self {
            Self::Bool(r) => Ok(Self::Bool(!r)),
            Self::UInt(_) => Err(OpError::UNeg),
            Self::SInt(r) => r
                .checked_neg()
                .map(Self::SInt)
                .ok_or(OpError::OverflowNeg(r)),

            // TODO: other types
            r => Err(OpError::Unsupported(r.as_type())),
        }
    }
}

pub fn evaluate<'src>(source: &'src str, ast: &Expr<'src>) -> Result<RunValue, ContextError<'src>> {
    use Punctuation::{Add, Div, Eq, Ge, Gt, Le, Lt, Mul, Ne, Neg, Not, Pow, Rem, Shl, Shr, Sub};
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            let l = evaluate(source, lhs)?;
            let r = evaluate(source, rhs)?;
            let LexValue::Punctuation(punc) = op.val else {
                unimplemented!();
            };
            match punc {
                Add => l.add(r),
                Sub => l.sub(r),
                Mul => l.mul(r),
                Div => l.div(r),
                Rem => l.rem(r),
                Pow => l.pow(r),
                Shl => l.shl(r),
                Shr => l.shr(r),
                Eq | Ne | Lt | Gt | Le | Ge => l.cmp(&r).map(|ord| {
                    RunValue::Bool(match punc {
                        Ne => ord.is_none_or(Ordering::is_ne),
                        Eq => ord.is_some_and(Ordering::is_eq),
                        Lt => ord.is_some_and(Ordering::is_lt),
                        Gt => ord.is_some_and(Ordering::is_gt),
                        Le => ord.is_some_and(Ordering::is_le),
                        Ge => ord.is_some_and(Ordering::is_ge),
                        _ => unreachable!(),
                    })
                }),

                _ => unimplemented!(),
            }
            .map_err(|e| e.binary(source, op, lhs, rhs))
        }

        Expr::Unary(inner) => {
            let Unary { op, rhs } = &**inner;
            let r = evaluate(source, rhs)?;
            let LexValue::Punctuation(punc) = op.val else {
                unimplemented!();
            };
            match punc {
                Not => r.not(),
                Sub | Neg => r.neg(),

                _ => unimplemented!(),
            }
            .map_err(|e| e.unary(source, op, punc, rhs))
        }

        Expr::Literal(token) => match token.val {
            LexValue::BoolLiteral(b) => Ok(RunValue::Bool(b)),
            LexValue::UIntLiteral(n) => Ok(RunValue::UInt(n)),
            LexValue::SIntLiteral(n) => Ok(RunValue::SInt(n)),
            LexValue::FltLiteral(x) => Ok(RunValue::Frac(x)),
            LexValue::CharLiteral(CharLiteral { ch, .. }) => Ok(RunValue::Char(ch)),
            LexValue::StringLiteral(s) => s
                .process()
                .map(|s| RunValue::Str(s.text))
                .map_err(|e| ContextError::token_error(source, Some(*token), e)),

            _ => unimplemented!(),
        },

        Expr::Grouping(group) => evaluate(source, &group.expr),
    }
}
