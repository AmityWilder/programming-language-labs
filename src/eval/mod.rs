//! Code execution

use crate::{
    error::{ContextError, ErrorType, IntConversionFailure, IntValue, OverflowError, TargetTy},
    grammar::{Binary, Expr, Unary},
    scanner::token::{
        Token,
        punc::Punctuation,
        value::{CharLiteral, LexValue},
    },
};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OverflowKind {
    UAdd { l: usize, r: usize },
    SAdd { l: isize, r: isize },
    USub { l: usize, r: usize },
    SSub { l: isize, r: isize },
    UMul { l: usize, r: usize },
    SMul { l: isize, r: isize },
    UPow { l: usize, r: usize },
    SPow { l: isize, r: isize },
    SNeg { r: isize },
}

#[derive(Debug, Clone, PartialEq)]
enum OpError {
    Incompatible(ValueType, ValueType),
    Unsupported(ValueType),
    FailedConversion {
        target_ty: TargetTy,
        value: IntValue,
        is_binary: bool,
    },
    Overflow(OverflowKind),
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
                target_ty,
                value,
                is_binary,
            } => ContextError::error(
                source,
                Some(rhs.range(source)),
                rhs.macro_range(source),
                ErrorType::FailedConvert {
                    op: match op.val {
                        LexValue::Punctuation(punc) => punc,
                        _ => unimplemented!(),
                    },
                    is_binary,
                    op_range: op.lex_range(source),
                    failure: IntConversionFailure::new(value, target_ty)
                        .expect("should not produce an error on infallible conversion"),
                },
            ),
            OpError::DivByZero => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::DivByZero {
                    zero: rhs.range(source),
                },
            ),
            OpError::Overflow(e) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(match e {
                    OverflowKind::UAdd { l, r } => OverflowError::UAdd {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::SAdd { l, r } => OverflowError::SAdd {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::USub { l, r } => OverflowError::USub {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::SSub { l, r } => OverflowError::SSub {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::UMul { l, r } => OverflowError::UMul {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::SMul { l, r } => OverflowError::SMul {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::UPow { l, r } => OverflowError::UPow {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::SPow { l, r } => OverflowError::SPow {
                        l_range: lhs.range(source),
                        r_range: rhs.range(source),
                        l_value: l,
                        r_value: r,
                    },
                    OverflowKind::SNeg { .. } => unimplemented!("not valid for binary"),
                }),
            ),

            OpError::Unsupported(_) | OpError::UNeg => {
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
                target_ty,
                value,
                is_binary,
            } => ContextError::error(
                source,
                Some(rhs.range(source)),
                rhs.macro_range(source),
                ErrorType::FailedConvert {
                    op: match op.val {
                        LexValue::Punctuation(Punctuation::SubNeg) => {
                            unreachable!("grammar module should have converted unary Sub into Neg")
                        }
                        LexValue::Punctuation(punc) => punc,
                        _ => unimplemented!(),
                    },
                    is_binary,
                    op_range: op.lex_range(source),
                    failure: IntConversionFailure::new(value, target_ty)
                        .expect("should not produce an error on infallible conversion"),
                },
            ),
            Self::Overflow(OverflowKind::SNeg { r }) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::SNeg {
                    r_range: rhs.range(source),
                    r_value: r,
                }),
            ),
            Self::UNeg => ContextError::token_error(source, Some(*op), ErrorType::UnsignedNeg),

            Self::Incompatible(_, _) | Self::DivByZero | Self::Overflow(_) => {
                unimplemented!("not valid for unary")
            }
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

    fn and(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l & r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l & r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn or(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l | r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l | r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn xor(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l ^ r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l ^ r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l ^ r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn nand(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l & r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l & r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l & r))),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn nor(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l | r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l | r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l | r))),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn xnor(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l ^ r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l ^ r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l ^ r))),

            // TODO: coersions?
            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn add(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_add(r)
                .map(Self::UInt)
                .ok_or(OpError::Overflow(OverflowKind::UAdd { l, r })),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_add(r)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SAdd { l, r })),
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
                .ok_or(OpError::Overflow(OverflowKind::USub { l, r })),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_sub(r)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SSub { l, r })),
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
                .ok_or(OpError::Overflow(OverflowKind::UMul { l, r })),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_mul(r)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SMul { l, r })),
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
                .checked_pow(u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?)
                .map(Self::UInt)
                .ok_or(OpError::Overflow(OverflowKind::UPow { l, r })),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_pow(u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SPow { l, r })),
            (Self::Frac(l), Self::UInt(r)) => {
                Ok(Self::Frac(l.powi(i32::try_from(r).map_err(|_| {
                    OpError::FailedConversion {
                        is_binary: true,
                        target_ty: TargetTy::S32,
                        value: IntValue::UInt(r),
                    }
                })?)))
            }
            (Self::Frac(l), Self::SInt(r)) => {
                Ok(Self::Frac(l.powi(i32::try_from(r).map_err(|_| {
                    OpError::FailedConversion {
                        is_binary: true,
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
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
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
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn rotl(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn rotr(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
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
                .ok_or(OpError::Overflow(OverflowKind::SNeg { r })),

            // TODO: other types
            r => Err(OpError::Unsupported(r.as_type())),
        }
    }
}

pub fn evaluate<'src>(source: &'src str, ast: &Expr<'src>) -> Result<RunValue, ContextError<'src>> {
    use Punctuation::{
        Add, And, Div, Eq, Ge, Gt, Le, Lt, Mul, Nand, Ne, Nor, Not, Or, Pow, Rem, Rotl, Rotr, Shl,
        Shr, SubNeg, Xnor, Xor,
    };
    // TODO: what about `none`?
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            let l = evaluate(source, lhs)?;
            let r = evaluate(source, rhs)?;
            let LexValue::Punctuation(punc) = op.val else {
                unimplemented!();
            };
            match punc {
                And => l.and(r),
                Xor => l.xor(r),
                Or => l.or(r),
                Nand => l.nand(r),
                Xnor => l.xnor(r),
                Nor => l.nor(r),
                Add => l.add(r),
                SubNeg => l.sub(r),
                Mul => l.mul(r),
                Div => l.div(r),
                Rem => l.rem(r),
                Pow => l.pow(r),
                Shl => l.shl(r),
                Shr => l.shr(r),
                Rotl => l.rotl(r),
                Rotr => l.rotr(r),

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
                SubNeg => r.neg(),

                _ => unimplemented!(),
            }
            .map_err(|e| e.unary(source, op, punc, rhs))
        }

        Expr::Literal(token) => match token.val {
            LexValue::BoolLiteral(b) => Ok(RunValue::Bool(b)),
            LexValue::UIntLiteral(n) => Ok(RunValue::UInt(n)),
            LexValue::SIntLiteral(n) => Ok(RunValue::SInt(n)),
            LexValue::FracLiteral(x) => Ok(RunValue::Frac(x)),
            LexValue::CharLiteral(CharLiteral { ch, .. }) => Ok(RunValue::Char(ch)),
            LexValue::TextLiteral(s) => s
                .process()
                .map(|s| RunValue::Str(s.text))
                .map_err(|e| ContextError::token_error(source, Some(*token), e)),

            _ => unimplemented!(),
        },

        Expr::Grouping(group) => evaluate(source, &group.expr),
    }
}
