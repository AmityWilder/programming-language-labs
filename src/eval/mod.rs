//! Code execution

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::{ContextError, ErrorType, OverflowError},
    grammar::{Binary, Expr, Unary},
    scanner::token::{
        Token,
        punc::Punctuation,
        value::{CharLiteral, LexValue},
    },
};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpError {
    TypeMismatch,
    FailedConversion(std::num::TryFromIntError),
    OverflowUAdd(usize, usize),
    OverflowSAdd(isize, isize),
    OverflowUSub(usize, usize),
    OverflowSSub(isize, isize),
    OverflowUMul(usize, usize),
    OverflowSMul(isize, isize),
    OverflowUPow(usize, usize),
    OverflowSPow(isize, isize),
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
        l_ty: ValueType,
        r_ty: ValueType,
    ) -> ContextError<'src> {
        match self {
            OpError::TypeMismatch => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Incompatible {
                    op: match op.val {
                        LexValue::Punctuation(punc) => punc,
                        _ => unimplemented!(),
                    },
                    lhs: (l_ty, lhs.range(source)),
                    rhs: (r_ty, rhs.range(source)),
                },
            ),
            // Assumes the only conversion failure can happen on the right hand side
            OpError::FailedConversion(e) => ContextError::error(
                source,
                Some(rhs.range(source)),
                rhs.macro_range(source),
                ErrorType::FailedConvert(e),
            ),
            OpError::DivByZero => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::DivByZero {
                    zero: rhs.range(source),
                },
            ),
            OpError::OverflowUAdd(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::UAdd {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowSAdd(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::SAdd {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowUSub(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::USub {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowSSub(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::SSub {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowUMul(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::UMul {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowSMul(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::SMul {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowUPow(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::UPow {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),
            OpError::OverflowSPow(l, r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::SPow {
                    lhs: (l, lhs.range(source)),
                    rhs: (r, rhs.range(source)),
                }),
            ),

            OpError::UNeg | OpError::OverflowNeg(_) => unimplemented!("not valid for binary"),
        }
    }

    fn unary<'src>(
        self,
        source: &'src str,
        op: &Token<'src>,
        punc: Punctuation,
        rhs: &Expr<'src>,
        r_ty: ValueType,
    ) -> ContextError<'src> {
        match self {
            Self::TypeMismatch => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Unsupported {
                    op: punc,
                    rhs: (r_ty, rhs.range(source)),
                },
            ),
            Self::FailedConversion(e) => ContextError::error(
                source,
                Some(rhs.range(source)),
                rhs.macro_range(source),
                ErrorType::FailedConvert(e),
            ),
            Self::OverflowNeg(r) => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::Overflow(OverflowError::Neg {
                    rhs: (r, rhs.range(source)),
                }),
            ),
            Self::UNeg => ContextError::token_error(source, Some(*op), ErrorType::UnsignedNeg),

            Self::DivByZero
            | Self::OverflowUAdd(_, _)
            | Self::OverflowSAdd(_, _)
            | Self::OverflowUSub(_, _)
            | Self::OverflowSSub(_, _)
            | Self::OverflowUMul(_, _)
            | Self::OverflowSMul(_, _)
            | Self::OverflowUPow(_, _)
            | Self::OverflowSPow(_, _) => unimplemented!("not valid for unary"),
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
            _ => Err(OpError::TypeMismatch),
        }
    }

    fn add(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_add(r)
                .map(Self::UInt)
                .ok_or(OpError::OverflowUAdd(l, r)),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_add(r)
                .map(Self::SInt)
                .ok_or(OpError::OverflowSAdd(l, r)),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l + r)),

            (Self::Str(l), Self::Bool(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::UInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::SInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Frac(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Char(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Bool(r), Self::Str(l)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::UInt(r), Self::Str(l)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::SInt(r), Self::Str(l)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Frac(r), Self::Str(l)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Char(r), Self::Str(l)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Str(l), Self::Str(r)) => Ok(Self::Str(l + &r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            _ => Err(OpError::TypeMismatch),
        }
    }

    fn sub(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_sub(r)
                .map(Self::UInt)
                .ok_or(OpError::OverflowUSub(l, r)),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_sub(r)
                .map(Self::SInt)
                .ok_or(OpError::OverflowSSub(l, r)),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l - r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            _ => Err(OpError::TypeMismatch),
        }
    }

    fn mul(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_mul(r)
                .map(Self::UInt)
                .ok_or(OpError::OverflowUMul(l, r)),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_mul(r)
                .map(Self::SInt)
                .ok_or(OpError::OverflowSMul(l, r)),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l * r)),

            // TODO: coersions?
            _ => Err(OpError::TypeMismatch),
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
            _ => Err(OpError::TypeMismatch),
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
            _ => Err(OpError::TypeMismatch),
        }
    }

    fn pow(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_pow(r.try_into().map_err(OpError::FailedConversion)?)
                .map(Self::UInt)
                .ok_or(OpError::OverflowUPow(l, r)),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_pow(r.try_into().map_err(OpError::FailedConversion)?)
                .map(Self::SInt)
                .ok_or(OpError::OverflowSPow(l, r)),
            (Self::Frac(l), Self::UInt(r)) => Ok(Self::Frac(
                l.powi(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::Frac(l), Self::SInt(r)) => Ok(Self::Frac(
                l.powi(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l.powf(r))),

            // TODO: coersions?
            _ => Err(OpError::TypeMismatch),
        }
    }

    fn shl(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.unbounded_shl(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(
                l.unbounded_shl(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(
                l.unbounded_shl(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.unbounded_shl(r.try_into().map_err(OpError::FailedConversion)?),
            )),

            _ => Err(OpError::TypeMismatch),
        }
    }

    fn shr(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.unbounded_shr(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::UInt(l), Self::SInt(r)) => Ok(Self::UInt(
                l.unbounded_shr(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::SInt(l), Self::UInt(r)) => Ok(Self::SInt(
                l.unbounded_shr(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.unbounded_shr(r.try_into().map_err(OpError::FailedConversion)?),
            )),

            _ => Err(OpError::TypeMismatch),
        }
    }

    fn not(self) -> Result<Self, OpError> {
        match self {
            Self::Bool(r) => Ok(Self::Bool(!r)),
            Self::UInt(r) => Ok(Self::UInt(!r)),
            Self::SInt(r) => Ok(Self::SInt(!r)),

            // TODO: other types
            _ => Err(OpError::TypeMismatch),
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
            _ => Err(OpError::TypeMismatch),
        }
    }
}

pub fn evaluate<'src>(source: &'src str, ast: &Expr<'src>) -> Result<RunValue, ContextError<'src>> {
    use Punctuation::*;
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            let l = evaluate(source, lhs)?;
            let r = evaluate(source, rhs)?;
            let l_ty = l.as_type();
            let r_ty = r.as_type();
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
            .map_err(|e| e.binary(source, op, lhs, rhs, l_ty, r_ty))
        }

        Expr::Unary(inner) => {
            let Unary { op, rhs } = &**inner;
            let r = evaluate(source, rhs)?;
            let r_ty = r.as_type();
            let LexValue::Punctuation(punc) = op.val else {
                unimplemented!();
            };
            match punc {
                Not => r.not(),
                Sub => r.neg(),

                _ => unimplemented!(),
            }
            .map_err(|e| e.unary(source, op, punc, rhs, r_ty))
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
