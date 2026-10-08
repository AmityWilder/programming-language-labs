//! Code execution

use crate::{
    error::{
        ContextError, ErrorType, IntConversionFailure, IntValue, OpSide, OverflowError, TargetTy,
    },
    grammar::ast::{Binary, Expr, Unary},
    scanner::token::{
        Token,
        keyword::Keyword,
        punc::Punctuation,
        value::{CharLiteral, LexValue},
    },
};
use std::{borrow::Cow, cmp::Ordering};

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
                rhs.expansion(source),
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
                rhs.expansion(source),
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

// TODO: add a `byte` type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ValueType {
    #[default]
    None,
    /// Not constructable
    // TBD: does this make sense to be a value type?
    Nevr,
    Bool,
    UInt,
    SInt,
    Frac,
    Char,
    Text,
    Fail,
    Type,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Nevr => "nevr",
            Self::Bool => "bool",
            Self::UInt => "uint",
            Self::SInt => "sint",
            Self::Frac => "frac",
            Self::Char => "char",
            Self::Text => "text",
            Self::Fail => "fail",
            // TBD: where does this appear?
            Self::Type => "typename",
        })
    }
}

#[derive(Debug, Default)]
pub enum RunValue {
    #[default]
    None,
    /// `none` that also outputs `none` as a result of every operation, instead of erroring
    // TBD: is a variant a good way of handling this?
    CoalesceNone,
    // TODO: would love if this could use a ContexError (for nicer display) and not just any error
    Fail(Box<dyn std::error::Error>),
    Bool(bool),
    UInt(usize),
    SInt(isize),
    Frac(f64),
    Char(char),
    Text(Cow<'static, str>),
    Type(ValueType),
}

impl RunValue {
    pub const fn as_type(&self) -> ValueType {
        match self {
            Self::None | Self::CoalesceNone => ValueType::None,
            Self::Fail(_) => ValueType::Fail,
            Self::Bool(_) => ValueType::Bool,
            Self::UInt(_) => ValueType::UInt,
            Self::SInt(_) => ValueType::SInt,
            Self::Frac(_) => ValueType::Frac,
            Self::Char(_) => ValueType::Char,
            Self::Text(_) => ValueType::Text,
            // TBD: is this a good idea?
            Self::Type(t) => *t,
        }
    }

    // Binary

    /// Returns [`None`] if compatible but incomparable
    /// (i.e. a non-existent `NotEqual` variant of [`std::cmp::Ordering`]).
    fn cmp(&self, other: &Self) -> Result<Option<std::cmp::Ordering>, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Some(l.cmp(r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Some(l.cmp(r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Some(l.cmp(r))),
            (Self::Frac(l), Self::Frac(r)) => Ok(l.partial_cmp(r)),
            (Self::Char(l), Self::Char(r)) => Ok(Some(l.cmp(r))),
            (Self::Text(l), Self::Text(r)) => Ok(Some(l.cmp(r))),

            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn and(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l & r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l & r)),

            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn or(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l | r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l | r)),

            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn xor(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l ^ r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l ^ r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l ^ r)),

            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn nand(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l & r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l & r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l & r))),

            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn nor(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l | r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l | r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l | r))),

            // TODO: char arithmetic?
            (l, r) => Err(OpError::Incompatible(l.as_type(), r.as_type())),
        }
    }

    fn xnor(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l ^ r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l ^ r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l ^ r))),

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

            (Self::Text(l), Self::Bool(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::Text(l), Self::UInt(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::Text(l), Self::SInt(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::Text(l), Self::Frac(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::Text(l), Self::Char(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),

            (Self::Bool(l), Self::Text(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::UInt(l), Self::Text(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::SInt(l), Self::Text(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::Frac(l), Self::Text(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),
            (Self::Char(l), Self::Text(r)) => Ok(Self::Text(Cow::Owned(format!("{l}{r}")))),

            (Self::Text(l), Self::Text(r)) => Ok(Self::Text(Cow::Owned(l.into_owned() + &*r))),

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

    /// The equivalent of [`Into`]/[`TryInto`]
    fn convert(self, into_ty: ValueType) -> Result<Self, OpError> {
        match (self, into_ty) {
            (Self::Type(_), _) => unimplemented!("should be caught by grammar"),
            (_, ValueType::Type) => unimplemented!("`type` isn't a type"),

            // TODO: add a warning about converting into self(?)
            // TBD: what is the value of `nevr`?
            (x @ (Self::None | Self::CoalesceNone), ValueType::None)
            | (x @ Self::Bool(_), ValueType::Bool)
            | (x @ Self::UInt(_), ValueType::UInt)
            | (x @ Self::SInt(_), ValueType::SInt)
            | (x @ Self::Frac(_), ValueType::Frac)
            | (x @ Self::Char(_), ValueType::Char)
            | (x @ Self::Text(_), ValueType::Text) => Ok(x),

            (x, t @ (ValueType::None | ValueType::Nevr | ValueType::Fail))
            | (x @ (Self::None | Self::CoalesceNone), t) => {
                Err(OpError::Incompatible(x.as_type(), t))
            }

            // stringify value
            (
                x @ (Self::Bool(_) | Self::UInt(_) | Self::SInt(_) | Self::Frac(_) | Self::Char(_)),
                ValueType::Text,
            ) => Ok(Self::Text(Cow::Owned(match &x {
                Self::Bool(x) => x.to_string(),
                Self::UInt(x) => x.to_string(),
                Self::SInt(x) => x.to_string(),
                Self::Frac(x) => x.to_string(),
                Self::Char(x) => x.to_string(),
                _ => unreachable!("guarded by match arm"),
            }))),

            // parse string
            (Self::Text(s), ValueType::Bool) => Ok(match s.parse() {
                Ok(x) => Self::Bool(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::Text(s), ValueType::UInt) => Ok(match s.parse() {
                Ok(x) => Self::UInt(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::Text(s), ValueType::SInt) => Ok(match s.parse() {
                Ok(x) => Self::SInt(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::Text(s), ValueType::Frac) => Ok(match s.parse() {
                Ok(x) => Self::Frac(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::Text(s), ValueType::Char) => Ok(match s.parse() {
                Ok(x) => Self::Char(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),

            // TBD: is this even a good idea?
            (x, ValueType::Bool) => Ok(x.exists()),

            (Self::Bool(x), ValueType::UInt) => Ok(Self::UInt(x.into())),
            (Self::Bool(x), ValueType::SInt) => Ok(Self::SInt(x.into())),
            (Self::Bool(x), ValueType::Frac) => Ok(Self::Frac(x.into())),
            (Self::Bool(x), ValueType::Char) => Ok(Self::Char(if x { '1' } else { '0' })),

            (Self::UInt(x), ValueType::SInt) => Ok(match x.try_into() {
                Ok(x) => Self::SInt(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::SInt(x), ValueType::UInt) => Ok(match x.try_into() {
                Ok(x) => Self::UInt(x),
                Err(e) => Self::Fail(Box::new(e)),
            }),

            // TODO: need more `FailedConversion` errors for things besides integers!
            (Self::UInt(x), ValueType::Frac) => Ok(Self::Frac(x as f64)), // TBD: error (or warning) for loss of data?
            (Self::UInt(x), ValueType::Char) => Ok(match u8::try_from(x) {
                // TODO: what about unicode?
                Ok(x) => Self::Char(char::from(x)),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::SInt(x), ValueType::Frac) => Ok(Self::Frac(x as f64)), // TBD: error (or warning) for loss of data?
            (Self::SInt(x), ValueType::Char) => Ok(match u8::try_from(x) {
                // TODO: what about unicode?
                Ok(x) => Self::Char(char::from(x)),
                Err(e) => Self::Fail(Box::new(e)),
            }),
            (Self::Frac(x), ValueType::UInt) => Ok(Self::UInt(x as usize)), // TBD: should truncation be an error/warning?
            (Self::Frac(x), ValueType::SInt) => Ok(Self::SInt(x as isize)), // TBD: should truncation be an error/warning?

            #[cfg(not(target_pointer_width = "16"))]
            #[expect(
                clippy::as_conversions,
                reason = "usize has no into impl even if wide enough to store the value"
            )]
            (Self::Char(ch), ValueType::UInt) => Ok(Self::UInt(ch.to_u32() as usize)),

            (l @ Self::Char(_), ValueType::SInt) => {
                // HACK: this might screw up error messages
                l.convert(ValueType::UInt)?.convert(ValueType::SInt)
            }

            // TODO: give this its own error
            (l, r) => Err(OpError::Incompatible(l.as_type(), r)),
        }
    }

    /// The equivalent of [`std::mem::transmute`]
    fn transmute(self, into_ty: ValueType) -> Result<Self, OpError> {
        match (self, into_ty) {
            (Self::Type(_), _) => unimplemented!("should be caught by grammar"),
            (_, ValueType::Type) => unimplemented!("`type` isn't a type"),

            // TODO: add a warning about transmuting into self(?)
            (x @ (Self::None | Self::CoalesceNone), ValueType::None | ValueType::Nevr)
            | (x @ Self::Bool(_), ValueType::Bool)
            | (x @ Self::UInt(_), ValueType::UInt)
            | (x @ Self::SInt(_), ValueType::SInt)
            | (x @ Self::Frac(_), ValueType::Frac)
            | (x @ Self::Char(_), ValueType::Char)
            | (x @ Self::Text(_), ValueType::Text) => Ok(x),

            // TBD: what about custom types?
            (Self::None | Self::CoalesceNone, _)
            | (_, ValueType::None | ValueType::Nevr | ValueType::Fail) => {
                todo!("'transmute involving none/nevr/fail' error")
            }

            (Self::UInt(x), ValueType::SInt) => Ok(Self::SInt(x.cast_signed())),
            (Self::SInt(x), ValueType::UInt) => Ok(Self::UInt(x.cast_unsigned())),

            // TODO: specialize for pointer widths
            (Self::UInt(x), ValueType::Frac) => Ok(Self::Frac(f64::from_bits(x as u64))),
            (Self::Frac(x), ValueType::UInt) => Ok(Self::UInt(x.to_bits() as usize)),

            (x @ Self::SInt(_), ValueType::Frac) => {
                // HACK: this might screw up error messages
                x.transmute(ValueType::UInt)?.transmute(ValueType::Frac)
            }
            (x @ Self::Frac(_), ValueType::SInt) => {
                // HACK: this might screw up error messages
                x.transmute(ValueType::UInt)?.transmute(ValueType::SInt)
            }

            (Self::Text(_), ValueType::UInt) => todo!("text pointer?"),

            // TBD: what about custom types? how will we measure their sizes?
            _ => todo!("incompatible layout error"),
        }
    }

    // Unary

    fn not(self) -> Result<Self, OpError> {
        match self {
            Self::Bool(r) => Ok(Self::Bool(!r)),
            Self::UInt(r) => Ok(Self::UInt(!r)),
            Self::SInt(r) => Ok(Self::SInt(!r)),

            r => Err(OpError::Unsupported(r.as_type())),
        }
    }

    fn neg(self) -> Result<Self, OpError> {
        match self {
            Self::UInt(_) => Err(OpError::UNeg),
            Self::SInt(r) => r
                .checked_neg()
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SNeg { r })),

            r => Err(OpError::Unsupported(r.as_type())),
        }
    }

    fn exists(self) -> Self {
        Self::Bool(match self {
            Self::None => false,
            Self::Bool(r) => r,
            Self::Frac(r) => !r.is_nan(),
            Self::Char(r) => r != '\0',
            Self::Text(r) => !r.is_empty(),
            _ => true,
        })
    }
}

pub fn evaluate<'src>(source: &'src str, ast: &Expr<'src>) -> Result<RunValue, ContextError<'src>> {
    use Punctuation::{
        Add, And, Coalesce, Convert, Div, Eq, Exists, Ge, Gt, Le, Lt, Mul, Nand, Ne, Nor, Not, Or,
        Pow, Rem, Rotl, Rotr, Shl, Shr, SubNeg, Transmute, Xnor, Xor,
    };
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            let l = evaluate(source, lhs)?;
            let r = evaluate(source, rhs)?;

            if matches!(
                (&l, &r),
                (RunValue::CoalesceNone, _) | (_, RunValue::CoalesceNone)
            ) {
                return Ok(RunValue::CoalesceNone);
            }

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
                Convert if let RunValue::Type(r) = r => l.convert(r),
                Transmute if let RunValue::Type(r) = r => l.transmute(r),

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
            let Unary { op, operand, side } = &**inner;
            let x = evaluate(source, operand)?;

            if matches!(&x, RunValue::CoalesceNone) {
                return Ok(RunValue::CoalesceNone);
            }

            let LexValue::Punctuation(punc) = op.val else {
                unimplemented!();
            };
            match (punc, side) {
                // prefix
                (Not, OpSide::Right) => x.not(),
                (SubNeg, OpSide::Right) => x.neg(),
                (Exists, OpSide::Right) => Ok(x.exists()),

                // postfix
                (Coalesce, OpSide::Left) => return Ok(RunValue::CoalesceNone),

                _ => unimplemented!(),
            }
            .map_err(|e| e.unary(source, op, punc, operand))
        }

        Expr::Literal(token) => match token.val {
            LexValue::BoolLiteral(b) => Ok(RunValue::Bool(b)),
            LexValue::UIntLiteral(n) => Ok(RunValue::UInt(n)),
            LexValue::SIntLiteral(n) => Ok(RunValue::SInt(n)),
            LexValue::FracLiteral(x) => Ok(RunValue::Frac(x)),
            LexValue::CharLiteral(CharLiteral { ch, .. }) => Ok(RunValue::Char(ch)),
            LexValue::TextLiteral(s) => s
                .process()
                .map(|s| RunValue::Text(Cow::Owned(s.text)))
                .map_err(|e| ContextError::token_error(source, Some(*token), e)),
            LexValue::Keyword(Keyword::None) => Ok(RunValue::None),

            _ => unimplemented!(),
        },

        Expr::Grouping(group) => evaluate(source, &group.expr),

        Expr::Type(inner) => match inner.name.val {
            LexValue::Keyword(kw) if kw.is_type() => Ok(RunValue::Type(match kw {
                Keyword::None => ValueType::None,
                Keyword::Nevr => ValueType::Nevr,
                Keyword::Bool => ValueType::Bool,
                Keyword::Uint => ValueType::UInt,
                Keyword::Sint => ValueType::SInt,
                Keyword::Frac => ValueType::Frac,
                Keyword::Char => ValueType::Char,
                Keyword::Text => ValueType::Text,
                Keyword::Fail => ValueType::Fail,

                Keyword::SelfKw // TBD: should `self` be a type?
                | Keyword::Rec
                | Keyword::Sup
                | Keyword::Cat
                | Keyword::Alt
                | Keyword::Sub
                | Keyword::Def
                | Keyword::Fn
                | Keyword::Mem
                | Keyword::Let
                | Keyword::Uni
                | Keyword::Pvt
                | Keyword::Has
                | Keyword::If
                | Keyword::Or
                | Keyword::Pick
                | Keyword::Rep
                | Keyword::For
                | Keyword::In
                | Keyword::Loop
                | Keyword::Cord
                | Keyword::Halt
                | Keyword::Skip
                | Keyword::Give
                | Keyword::Emit => unimplemented!("not a typename"),
            })),

            LexValue::Identifier | LexValue::Callable => todo!("custom types"),

            _ => unimplemented!(),
        },
    }
}
