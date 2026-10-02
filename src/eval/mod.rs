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
        punc::Punctuation,
        value::{CharLiteral, Value as TokenValue},
    },
};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpError {
    Incompatible,
    FailedConversion(std::num::TryFromIntError),
    Overflow,
    DivByZero,
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
pub enum Value {
    Bool(bool),
    UInt(usize),
    SInt(isize),
    Frac(f64),
    Char(char),
    Str(String),
}

impl Value {
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
            _ => Err(OpError::Incompatible),
        }
    }

    fn add(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_add(r).map(Self::UInt).ok_or(OpError::Overflow)
            }
            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_add(r).map(Self::SInt).ok_or(OpError::Overflow)
            }
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
            _ => Err(OpError::Incompatible),
        }
    }

    fn sub(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_sub(r).map(Self::UInt).ok_or(OpError::Overflow)
            }
            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_sub(r).map(Self::SInt).ok_or(OpError::Overflow)
            }
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l - r)),

            // TODO: coersions?
            // TODO: char arithmetic?
            _ => Err(OpError::Incompatible),
        }
    }

    fn mul(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_mul(r).map(Self::UInt).ok_or(OpError::Overflow)
            }
            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_mul(r).map(Self::SInt).ok_or(OpError::Overflow)
            }
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l * r)),

            // TODO: coersions?
            _ => Err(OpError::Incompatible),
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
            _ => Err(OpError::Incompatible),
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
            _ => Err(OpError::Incompatible),
        }
    }

    fn pow(self, other: Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_pow(r.try_into().map_err(OpError::FailedConversion)?)
                .map(Self::UInt)
                .ok_or(OpError::Overflow),
            (Self::SInt(l), Self::SInt(r)) => l
                .checked_pow(r.try_into().map_err(OpError::FailedConversion)?)
                .map(Self::SInt)
                .ok_or(OpError::Overflow),
            (Self::Frac(l), Self::UInt(r)) => Ok(Self::Frac(
                l.powi(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::Frac(l), Self::SInt(r)) => Ok(Self::Frac(
                l.powi(r.try_into().map_err(OpError::FailedConversion)?),
            )),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l.powf(r))),

            // TODO: coersions?
            _ => Err(OpError::Incompatible),
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

            _ => Err(OpError::Incompatible),
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

            _ => Err(OpError::Incompatible),
        }
    }

    // TODO: Unary
}

pub fn evaluate<'src>(source: &'src str, ast: &Expr<'src>) -> Result<Value, ContextError<'src>> {
    use Punctuation::*;
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;

            macro_rules! operate {
                ($operation:ident($l:expr, $r:expr) -> $Value:ident else DivByZero) => {
                    $l.$operation($r).map(Value::$Value).ok_or_else(|| {
                        ContextError::token_error(
                            source,
                            Some(*op),
                            ErrorType::DivByZero {
                                zero: rhs.range(source),
                            },
                        )
                    })
                };

                ($operation:ident($l:expr, $r:expr) -> $Value:ident else $Overflow:ident) => {
                    $l.$operation($r).map(Value::$Value).ok_or_else(|| {
                        ContextError::token_error(
                            source,
                            Some(*op),
                            ErrorType::Overflow(OverflowError::$Overflow {
                                lhs: ($l, lhs.range(source)),
                                rhs: ($r, rhs.range(source)),
                            }),
                        )
                    })
                };
            }

            match (op.val, evaluate(source, lhs)?, evaluate(source, rhs)?) {
                // Addition
                (TokenValue::Punctuation(Add), Value::UInt(l), Value::UInt(r)) => {
                    operate!(checked_add(l, r) -> UInt else UAdd)
                }
                (TokenValue::Punctuation(Add), Value::SInt(l), Value::SInt(r)) => {
                    operate!(checked_add(l, r) -> SInt else SAdd)
                }

                // Subtraction
                (TokenValue::Punctuation(Sub), Value::UInt(l), Value::UInt(r)) => {
                    operate!(checked_sub(l, r) -> UInt else USub)
                }
                (TokenValue::Punctuation(Sub), Value::SInt(l), Value::SInt(r)) => {
                    operate!(checked_sub(l, r) -> SInt else SSub)
                }

                // Multiplication
                (TokenValue::Punctuation(Mul), Value::UInt(l), Value::UInt(r)) => {
                    operate!(checked_mul(l, r) -> UInt else USub)
                }

                // Division
                (TokenValue::Punctuation(Div), Value::UInt(l), Value::UInt(r)) => {
                    operate!(checked_div(l, r) -> UInt else DivByZero)
                }

                // Remainder
                (TokenValue::Punctuation(Rem), Value::UInt(l), Value::UInt(r)) => {
                    operate!(checked_rem(l, r) -> UInt else DivByZero)
                }

                // Power
                (TokenValue::Punctuation(Pow), Value::UInt(l), Value::UInt(r)) => l
                    .checked_pow(r.try_into().map_err(|e| {
                        ContextError::error(
                            source,
                            Some(rhs.range(source)),
                            rhs.macro_range(source),
                            ErrorType::FailedConvert(e),
                        )
                    })?)
                    .map(Value::UInt)
                    .ok_or_else(|| {
                        ContextError::error(
                            source,
                            Some(rhs.range(source)),
                            rhs.macro_range(source),
                            ErrorType::Overflow(OverflowError::UPow {
                                lhs: (l, lhs.range(source)),
                                rhs: (r, rhs.range(source)),
                            }),
                        )
                    }),

                // Comparison
                (TokenValue::Punctuation(cmp @ (Ne | Eq | Gt | Ge | Lt | Le)), l, r) => {
                    let ord = l.cmp(&r).map_err(|e| {
                        debug_assert_eq!(e, OpError::Incompatible, "assumption");
                        ContextError::token_error(
                            source,
                            Some(*op),
                            ErrorType::Incompatible {
                                op: cmp,
                                lhs: (l.as_type(), lhs.range(source)),
                                rhs: (r.as_type(), rhs.range(source)),
                            },
                        )
                    })?;
                    Ok(Value::Bool(match cmp {
                        Ne => ord.is_none_or(Ordering::is_ne),
                        Eq => ord.is_some_and(Ordering::is_eq),
                        Gt => ord.is_some_and(Ordering::is_gt),
                        Ge => ord.is_some_and(Ordering::is_ge),
                        Lt => ord.is_some_and(Ordering::is_lt),
                        Le => ord.is_some_and(Ordering::is_le),
                        _ => unreachable!(),
                    }))
                }

                // Shift left
                (TokenValue::Punctuation(Shl), Value::UInt(l), Value::UInt(r)) => todo!(),

                // Shift right
                (TokenValue::Punctuation(Shr), Value::UInt(l), Value::UInt(r)) => todo!(),

                // Supported operators, but not for these operands
                (
                    TokenValue::Punctuation(punc @ (Sub | Mul | Div | Rem | Pow | Shl | Shr)),
                    l,
                    r,
                ) => Err(ContextError::token_error(
                    source,
                    Some(*op),
                    ErrorType::Incompatible {
                        op: punc,
                        lhs: (l.as_type(), lhs.range(source)),
                        rhs: (r.as_type(), rhs.range(source)),
                    },
                )),

                _ => unimplemented!(),
            }
        }

        Expr::Unary(inner) => {
            let Unary { op, rhs } = &**inner;
            match (op.val, evaluate(source, rhs)?) {
                // Not
                (TokenValue::Punctuation(Not), Value::UInt(r)) => todo!(),

                // Negation
                (TokenValue::Punctuation(Sub), Value::UInt(r)) => todo!(),

                (TokenValue::Punctuation(punc @ (Not | Sub)), r) => Err(ContextError::token_error(
                    source,
                    Some(*op),
                    ErrorType::Unsupported {
                        op: punc,
                        rhs: (r.as_type(), rhs.range(source)),
                    },
                )),

                _ => unimplemented!(),
            }
        }

        Expr::Literal(token) => match token.val {
            TokenValue::BoolLiteral(b) => Ok(Value::Bool(b)),
            TokenValue::UIntLiteral(n) => Ok(Value::UInt(n)),
            TokenValue::SIntLiteral(n) => Ok(Value::SInt(n)),
            TokenValue::FltLiteral(x) => Ok(Value::Frac(x)),
            TokenValue::CharLiteral(CharLiteral { ch, .. }) => Ok(Value::Char(ch)),
            TokenValue::StringLiteral(s) => s
                .process()
                .map(|s| Value::Str(s.text))
                .map_err(|e| ContextError::token_error(source, Some(*token), e)),

            _ => unimplemented!(),
        },

        Expr::Grouping(group) => Ok(evaluate(source, &group.expr)?),
    }
}
