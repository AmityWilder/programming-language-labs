//! Code execution

use super::{env::UntypedPointer, error::Failure};
use crate::{
    error::{
        ContextError, ErrorType, IntConversionFailure, IntValue, OpSide, OverflowError, TargetTy,
    },
    exec::env::Environment,
    grammar::ast::{AstNodeFinal, Binary, Expr, Literal, Unary},
    scanner::token::{
        Token,
        keyword::Keyword,
        punc::Punctuation,
        value::{CharLiteral, LexValue, TokenEq},
    },
    types::fsize,
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
                Some(op),
                ErrorType::Incompatible {
                    op: op
                        .val
                        .to_token_eq()
                        .expect("only TokenEq supported for ops"),
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
            } => ContextError::node_error(
                source,
                rhs,
                ErrorType::FailedConvert {
                    op: op
                        .val
                        .to_token_eq()
                        .expect("only TokenEq supported for ops"),
                    is_binary,
                    op_range: op.lex_range(source),
                    failure: IntConversionFailure::new(value, target_ty)
                        .expect("should not produce an error on infallible conversion"),
                },
            ),
            OpError::DivByZero => ContextError::token_error(
                source,
                Some(op),
                ErrorType::DivByZero {
                    zero: rhs.range(source),
                },
            ),
            OpError::Overflow(e) => ContextError::token_error(
                source,
                Some(op),
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
        punc: TokenEq,
        rhs: &Expr<'src>,
    ) -> ContextError<'src> {
        match self {
            Self::Unsupported(r_ty) => ContextError::token_error(
                source,
                Some(op),
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
            } => ContextError::node_error(
                source,
                rhs,
                ErrorType::FailedConvert {
                    op: op
                        .val
                        .to_token_eq()
                        .expect("only TokenEq supported for ops"),
                    is_binary,
                    op_range: op.lex_range(source),
                    failure: IntConversionFailure::new(value, target_ty)
                        .expect("should not produce an error on infallible conversion"),
                },
            ),
            Self::Overflow(OverflowKind::SNeg { r }) => ContextError::token_error(
                source,
                Some(op),
                ErrorType::Overflow(OverflowError::SNeg {
                    r_range: rhs.range(source),
                    r_value: r,
                }),
            ),
            Self::UNeg => ContextError::token_error(source, Some(op), ErrorType::UnsignedNeg),

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
    Ptr,
    /// Only valid in preprocessing
    Macro,
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
            Self::Ptr => "pointer",
            // TBD: where does this appear?
            Self::Type => "typename",
            Self::Macro => "macro identifier",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum RunValue<'src> {
    #[default]
    None,
    /// `none` that also outputs `none` as a result of every operation, instead of erroring
    // TBD: is a variant a good way of handling this?
    CoalesceNone,
    // TODO: would love if this could use a ContexError (for nicer display) and not just any error
    Fail(Failure),
    Bool(bool),
    UInt(usize),
    SInt(isize),
    Frac(fsize),
    Char(char),
    Text(Cow<'src, str>),
    Type(ValueType),
    UPtr(UntypedPointer),
}

impl<'src> RunValue<'src> {
    pub fn typename(&self) -> ValueType {
        match self {
            Self::None | Self::CoalesceNone => ValueType::None,
            Self::Fail(_) => ValueType::Fail,
            Self::Bool(_) => ValueType::Bool,
            Self::UInt(_) => ValueType::UInt,
            Self::SInt(_) => ValueType::SInt,
            Self::Frac(_) => ValueType::Frac,
            Self::Char(_) => ValueType::Char,
            Self::Text(_) => ValueType::Text,
            Self::UPtr(_) => ValueType::Ptr,
            // TBD: is this a good idea?
            Self::Type(t) => *t,
        }
    }

    pub fn print(&self) {
        match self {
            RunValue::None | RunValue::CoalesceNone => println!("null"), // TODO: color?
            RunValue::Fail(e) => println!("{e}"),                        // TODO: color?
            RunValue::Bool(x) => println!("{x}"),
            RunValue::UInt(x) => println!("{x}"),
            RunValue::SInt(x) => println!("{x}"),
            RunValue::Frac(x) => println!("{x}"),
            RunValue::Char(x) => println!("{x}"),
            RunValue::Text(x) => println!("{x}"),
            RunValue::UPtr(_) => todo!(),
            RunValue::Type(_) => unimplemented!("typename does not exist at runtime"),
        }
    }

    // Binary

    /// Remember that [`PartialEq`] is separate from [`PartialOrd`].
    /// Some types can be tested for equality while not supporting ordering.
    fn eq(&self, other: &Self) -> Result<bool, OpError> {
        match (self, other) {
            (Self::None, Self::None) => Ok(true),
            (Self::None, _) | (_, Self::None) => Ok(false), // TBD: should this be consistent with "exists"?
            (Self::Bool(l), Self::Bool(r)) => Ok(l.eq(r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(l.eq(r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(l.eq(r)),
            // remember that equality is symmetric
            (Self::UInt(u), Self::SInt(s)) | (Self::SInt(s), Self::UInt(u)) => Ok({
                // if conversion fails, they're definitely not equal
                isize::try_from(*u).is_ok_and(|u| u.eq(s))
            }),
            // frac should have to convert in order to compare with integers
            (Self::Frac(l), Self::Frac(r)) => Ok(l.eq(r)),
            (Self::Char(l), Self::Char(r)) => Ok(l.eq(r)),
            (Self::Text(l), Self::Text(r)) => Ok(l.eq(r)),
            (Self::Char(c), Self::Text(t)) | (Self::Text(t), Self::Char(c)) => {
                Ok(t.eq(c.encode_utf8(&mut [0; char::MAX_LEN_UTF8])))
            }

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    /// Returns [`None`] if compatible but incomparable
    /// (i.e. a non-existent `NotEqual` variant of [`std::cmp::Ordering`]).
    fn cmp(&self, other: &Self) -> Result<Option<std::cmp::Ordering>, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Some(l.cmp(r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Some(l.cmp(r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Some(l.cmp(r))),
            (Self::SInt(l), Self::UInt(r)) => Ok(Some(
                // failure to convert sint to uint implies the sint is negative,
                // and therefore certainly less than a uint
                usize::try_from(*l).map_or(std::cmp::Ordering::Less, |l| l.cmp(r)),
            )),
            (Self::UInt(l), Self::SInt(r)) => Ok(Some(
                // failure to convert sint to uint implies the sint is negative,
                // and therefore certainly less than a uint
                usize::try_from(*r).map_or(std::cmp::Ordering::Greater, |r| l.cmp(&r)),
            )),
            (Self::Frac(l), Self::Frac(r)) => Ok(l.partial_cmp(r)),
            (Self::Char(l), Self::Char(r)) => Ok(Some(l.cmp(r))),
            (Self::Text(l), Self::Text(r)) => Ok(Some(l.cmp(r))),
            (Self::Char(l), Self::Text(r)) => Ok(Some(
                (*l.encode_utf8(&mut [0; char::MAX_LEN_UTF8])).cmp(r.as_ref()),
            )),
            (Self::Text(l), Self::Char(r)) => Ok(Some(
                l.as_ref().cmp(r.encode_utf8(&mut [0; char::MAX_LEN_UTF8])),
            )),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn and(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l & r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l & r)),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn or(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l | r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l | r)),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn xor(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l ^ r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l ^ r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l ^ r)),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn nand(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l & r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l & r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l & r))),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn nor(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l | r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l | r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l | r))),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn xnor(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l ^ r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l ^ r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l ^ r))),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn add(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::Bool(l), &Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (&Self::UInt(l), &Self::UInt(r)) => l
                .checked_add(r)
                .map(Self::UInt)
                .ok_or(OpError::Overflow(OverflowKind::UAdd { l, r })),
            (&Self::SInt(l), &Self::SInt(r)) => l
                .checked_add(r)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SAdd { l, r })),
            (&Self::Frac(l), &Self::Frac(r)) => Ok(Self::Frac(l + r)),

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

            (Self::Text(l), Self::Text(r)) => {
                Ok(Self::Text(Cow::Owned(l.as_ref().to_string() + &*r)))
            }

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn sub(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => l
                .checked_sub(r)
                .map(Self::UInt)
                .ok_or(OpError::Overflow(OverflowKind::USub { l, r })),
            (&Self::SInt(l), &Self::SInt(r)) => l
                .checked_sub(r)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SSub { l, r })),
            (&Self::Frac(l), &Self::Frac(r)) => Ok(Self::Frac(l - r)),

            // TODO: char arithmetic? (needs errors)
            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn mul(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::Bool(l), &Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (&Self::UInt(l), &Self::UInt(r)) => l
                .checked_mul(r)
                .map(Self::UInt)
                .ok_or(OpError::Overflow(OverflowKind::UMul { l, r })),
            (&Self::SInt(l), &Self::SInt(r)) => l
                .checked_mul(r)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SMul { l, r })),
            (&Self::Frac(l), &Self::Frac(r)) => Ok(Self::Frac(l * r)),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn div(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => {
                l.checked_div(r).map(Self::UInt).ok_or(OpError::DivByZero)
            }
            (&Self::SInt(l), &Self::SInt(r)) => {
                l.checked_div(r).map(Self::SInt).ok_or(OpError::DivByZero)
            }
            (&Self::Frac(l), &Self::Frac(r)) => Ok(Self::Frac(l / r)),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn rem(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => {
                l.checked_rem(r).map(Self::UInt).ok_or(OpError::DivByZero)
            }
            (&Self::SInt(l), &Self::SInt(r)) => {
                l.checked_rem(r).map(Self::SInt).ok_or(OpError::DivByZero)
            }
            (&Self::Frac(l), &Self::Frac(r)) => Ok(Self::Frac(l / r)),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn pow(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => l
                .checked_pow(u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?)
                .map(Self::UInt)
                .ok_or(OpError::Overflow(OverflowKind::UPow { l, r })),
            (&Self::SInt(l), &Self::SInt(r)) => l
                .checked_pow(u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?)
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SPow { l, r })),
            (&Self::Frac(l), &Self::UInt(r)) => {
                Ok(Self::Frac(l.powi(i32::try_from(r).map_err(|_| {
                    OpError::FailedConversion {
                        is_binary: true,
                        target_ty: TargetTy::S32,
                        value: IntValue::UInt(r),
                    }
                })?)))
            }
            (&Self::Frac(l), &Self::SInt(r)) => {
                Ok(Self::Frac(l.powi(i32::try_from(r).map_err(|_| {
                    OpError::FailedConversion {
                        is_binary: true,
                        target_ty: TargetTy::S32,
                        value: IntValue::SInt(r),
                    }
                })?)))
            }
            (&Self::Frac(l), &Self::Frac(r)) => Ok(Self::Frac(l.powf(r))),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn shl(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => Ok(Self::UInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::UInt(l), &Self::SInt(r)) => Ok(Self::UInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::UInt(r)) => Ok(Self::SInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::SInt(r)) => Ok(Self::SInt(l.unbounded_shl(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn shr(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => Ok(Self::UInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::UInt(l), &Self::SInt(r)) => Ok(Self::UInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::UInt(r)) => Ok(Self::SInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::SInt(r)) => Ok(Self::SInt(l.unbounded_shr(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn rotl(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => Ok(Self::UInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::UInt(l), &Self::SInt(r)) => Ok(Self::UInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::UInt(r)) => Ok(Self::SInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::SInt(r)) => Ok(Self::SInt(l.rotate_left(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    fn rotr(&self, other: &Self) -> Result<Self, OpError> {
        match (self, other) {
            (&Self::UInt(l), &Self::UInt(r)) => Ok(Self::UInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::UInt(l), &Self::SInt(r)) => Ok(Self::UInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::UInt(r)) => Ok(Self::SInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::UInt(r),
                })?,
            ))),
            (&Self::SInt(l), &Self::SInt(r)) => Ok(Self::SInt(l.rotate_right(
                u32::try_from(r).map_err(|_| OpError::FailedConversion {
                    is_binary: true,
                    target_ty: TargetTy::U32,
                    value: IntValue::SInt(r),
                })?,
            ))),

            (l, r) => Err(OpError::Incompatible(l.typename(), r.typename())),
        }
    }

    /// The equivalent of [`Into`]/[`TryInto`]
    fn convert(&self, into_ty: ValueType) -> Result<Self, OpError> {
        match (self, into_ty) {
            (Self::Type(_), _) => unimplemented!("should be caught by grammar"),
            (_, ValueType::Type) => unimplemented!("`{into_ty}` isn't a type"),

            // TODO: add a warning about converting into self(?)
            // TBD: what is the value of `nevr`?
            (Self::None | Self::CoalesceNone, ValueType::None)
            | (Self::Bool(_), ValueType::Bool)
            | (Self::UInt(_), ValueType::UInt)
            | (Self::SInt(_), ValueType::SInt)
            | (Self::Frac(_), ValueType::Frac)
            | (Self::Char(_), ValueType::Char)
            | (Self::Text(_), ValueType::Text) => Ok(self.clone()),

            (x, t @ (ValueType::None | ValueType::Nevr | ValueType::Fail))
            | (x @ (Self::None | Self::CoalesceNone), t) => {
                Err(OpError::Incompatible(x.typename(), t))
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
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            (Self::Text(s), ValueType::UInt) => Ok(match s.parse() {
                Ok(x) => Self::UInt(x),
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            (Self::Text(s), ValueType::SInt) => Ok(match s.parse() {
                Ok(x) => Self::SInt(x),
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            (Self::Text(s), ValueType::Frac) => Ok(match s.parse() {
                Ok(x) => Self::Frac(x),
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            (Self::Text(s), ValueType::Char) => Ok(match s.parse() {
                Ok(x) => Self::Char(x),
                Err(e) => Self::Fail(Failure::from(e)),
            }),

            // TBD: is this even a good idea?
            (x, ValueType::Bool) => Ok(x.exists()),

            (&Self::Bool(x), ValueType::UInt) => Ok(Self::UInt(x.into())),
            (&Self::Bool(x), ValueType::SInt) => Ok(Self::SInt(x.into())),
            (&Self::Bool(x), ValueType::Frac) => Ok(Self::Frac(x.into())),
            (&Self::Bool(x), ValueType::Char) => Ok(Self::Char(if x { '1' } else { '0' })),

            (&Self::UInt(x), ValueType::SInt) => Ok(match x.try_into() {
                Ok(x) => Self::SInt(x),
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            (&Self::SInt(x), ValueType::UInt) => Ok(match x.try_into() {
                Ok(x) => Self::UInt(x),
                Err(e) => Self::Fail(Failure::from(e)),
            }),

            // TODO: need more `FailedConversion` errors for things besides integers!
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "no other way to convert int to float while ignoring errors"
            )]
            (&Self::UInt(x), ValueType::Frac) => Ok(Self::Frac(x as fsize)), // TBD: error (or warning) for loss of data?
            (&Self::UInt(x), ValueType::Char) => Ok(match u8::try_from(x) {
                // TODO: what about unicode?
                Ok(x) => Self::Char(char::from(x)),
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "no other way to convert int to float while ignoring errors"
            )]
            (&Self::SInt(x), ValueType::Frac) => Ok(Self::Frac(x as fsize)), // TBD: error (or warning) for loss of data?
            (&Self::SInt(x), ValueType::Char) => Ok(match u8::try_from(x) {
                // TODO: what about unicode?
                Ok(x) => Self::Char(char::from(x)),
                Err(e) => Self::Fail(Failure::from(e)),
            }),
            #[expect(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "no other way to convert int to float while ignoring errors"
            )]
            (&Self::Frac(x), ValueType::UInt) => Ok(Self::UInt(x as usize)), // TBD: should truncation be an error/warning?
            #[expect(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                reason = "no other way to convert int to float while ignoring errors"
            )]
            (&Self::Frac(x), ValueType::SInt) => Ok(Self::SInt(x as isize)), // TBD: should truncation be an error/warning?

            #[cfg(not(target_pointer_width = "16"))]
            #[expect(
                clippy::as_conversions,
                reason = "usize has no into impl even if wide enough to store the value"
            )]
            (&Self::Char(ch), ValueType::UInt) => Ok(Self::UInt(ch.to_u32() as usize)),

            // TODO: give this its own error
            (l, r) => Err(OpError::Incompatible(l.typename(), r)),
        }
    }

    /// The equivalent of [`std::mem::transmute`]
    fn transmute(&self, into_ty: ValueType) -> Result<Self, OpError> {
        match (self, into_ty) {
            (Self::Type(_), _) => unimplemented!("should be caught by grammar"),
            (_, ValueType::Type) => unimplemented!("`{into_ty}` isn't a type"),

            // TODO: add a warning about transmuting into self(?)
            (Self::None | Self::CoalesceNone, ValueType::None | ValueType::Nevr)
            | (Self::Bool(_), ValueType::Bool)
            | (Self::UInt(_), ValueType::UInt)
            | (Self::SInt(_), ValueType::SInt)
            | (Self::Frac(_), ValueType::Frac)
            | (Self::Char(_), ValueType::Char)
            | (Self::Text(_), ValueType::Text) => Ok(self.clone()),

            // TBD: what about custom types?
            (Self::None | Self::CoalesceNone, _)
            | (_, ValueType::None | ValueType::Nevr | ValueType::Fail) => {
                todo!("'transmute involving none/nevr/fail' error")
            }

            (&Self::UInt(x), ValueType::SInt) => Ok(Self::SInt(x.cast_signed())),
            (&Self::SInt(x), ValueType::UInt) => Ok(Self::UInt(x.cast_unsigned())),

            // TODO: specialize for pointer widths
            (&Self::UInt(x), ValueType::Frac) => Ok(Self::Frac(fsize::from_bits(x as u64))),
            (&Self::Frac(x), ValueType::UInt) => Ok(Self::UInt(x.to_bits() as usize)),

            (Self::Text(_), ValueType::UInt) => todo!("text pointer?"),

            // TBD: what about custom types? how will we measure their sizes?
            _ => todo!("incompatible layout error"),
        }
    }

    // Unary

    fn not(&self) -> Result<Self, OpError> {
        match self {
            &Self::Bool(r) => Ok(Self::Bool(!r)),
            &Self::UInt(r) => Ok(Self::UInt(!r)),
            &Self::SInt(r) => Ok(Self::SInt(!r)),

            r => Err(OpError::Unsupported(r.typename())),
        }
    }

    fn neg(&self) -> Result<Self, OpError> {
        match self {
            Self::UInt(_) => Err(OpError::UNeg),
            &Self::SInt(r) => r
                .checked_neg()
                .map(Self::SInt)
                .ok_or(OpError::Overflow(OverflowKind::SNeg { r })),

            r => Err(OpError::Unsupported(r.typename())),
        }
    }

    fn exists(&self) -> Self {
        Self::Bool(match self {
            Self::None => false,
            Self::Bool(r) => *r,
            Self::Frac(r) => !r.is_nan(),
            Self::Char(r) => *r != '\0',
            Self::Text(r) => !r.is_empty(),
            _ => true,
        })
    }

    pub fn sizeof(&self) -> usize {
        // very important that this is the ACTUAL SIZE of the value! it will be used to store it in the environment!!
        match self {
            RunValue::None | RunValue::CoalesceNone => std::mem::size_of_val(&()),
            RunValue::Bool(x) => std::mem::size_of_val(x),
            RunValue::UInt(x) => std::mem::size_of_val(x),
            RunValue::SInt(x) => std::mem::size_of_val(x),
            RunValue::Frac(x) => std::mem::size_of_val(x),
            RunValue::Char(x) => std::mem::size_of_val(x),
            RunValue::Text(x) => std::mem::size_of_val(&**x), // TODO: depends on literal/const vs runtime
            RunValue::Fail(e) => std::mem::size_of_val(e),
            RunValue::UPtr(ptr) => std::mem::size_of_val(ptr),
            RunValue::Type(t) => match t {
                ValueType::None | ValueType::Nevr => std::mem::size_of::<()>(),
                ValueType::Bool => std::mem::size_of::<bool>(),
                ValueType::UInt => std::mem::size_of::<usize>(),
                ValueType::SInt => std::mem::size_of::<isize>(),
                ValueType::Frac => std::mem::size_of::<fsize>(),
                ValueType::Char => std::mem::size_of::<char>(),
                ValueType::Text => todo!("multiple representations/dynamically sized"),
                ValueType::Fail => std::mem::size_of::<Failure>(),
                ValueType::Ptr => std::mem::size_of::<UntypedPointer>(),
                ValueType::Type | ValueType::Macro => unimplemented!("{t} is not a runtime type"),
            },
        }
    }

    pub fn alignof(&self) -> usize {
        // very important that this is the ACTUAL ALIGNMENT of the type! it will be used to store it in the environment!!
        match self {
            RunValue::None | RunValue::CoalesceNone | RunValue::Type(ValueType::None) => 0,
            RunValue::Bool(_) | RunValue::Type(ValueType::Bool) => std::mem::align_of::<bool>(),
            RunValue::UInt(_) | RunValue::Type(ValueType::UInt) => std::mem::align_of::<usize>(),
            RunValue::SInt(_) | RunValue::Type(ValueType::SInt) => std::mem::align_of::<isize>(),
            RunValue::Frac(_) | RunValue::Type(ValueType::Frac) => std::mem::align_of::<fsize>(),
            RunValue::Char(_) | RunValue::Type(ValueType::Char) => std::mem::align_of::<char>(),
            RunValue::Text(_) | RunValue::Type(ValueType::Text) => std::mem::align_of::<u8>(), // i thiiiink?..
            RunValue::Fail(_) | RunValue::Type(ValueType::Fail) => std::mem::align_of::<Failure>(),
            RunValue::UPtr(_) | RunValue::Type(ValueType::Ptr) => {
                std::mem::align_of::<UntypedPointer>()
            }

            RunValue::Type(ValueType::Nevr) => todo!("shouldn't be stored"),
            RunValue::Type(t @ (ValueType::Type | ValueType::Macro)) => {
                unimplemented!("{t} is not a runtime type")
            }
        }
    }
}

pub fn evaluate<'src>(
    source: &'src str,
    env: &Environment<'src>,
    ast: &Expr<'src>,
) -> Result<RunValue<'src>, ContextError<'src>> {
    use Keyword::Szof;
    use Punctuation::{
        Add, And, Coalesce, Convert, Div, Eq, Exists, Ge, Gt, Le, Lt, Mul, Nand, Ne, Nor, Not, Or,
        Pow, Rem, Rotl, Rotr, Shl, Shr, SubNeg, Transmute, Xnor, Xor,
    };
    match ast {
        Expr::Ternary(_) => todo!("deprecated"),

        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            let l = evaluate(source, env, lhs)?;
            let r = evaluate(source, env, rhs)?;

            if matches!(
                (&l, &r),
                (RunValue::CoalesceNone, _) | (_, RunValue::CoalesceNone)
            ) {
                return Ok(RunValue::CoalesceNone);
            }

            let punc = op
                .val
                .to_token_eq()
                .expect("only TokenEq supported for ops");
            match punc {
                TokenEq::Keyword(_keyword) => unimplemented!("no binary keywords yet"),

                TokenEq::Punctuation(punctuation) => match punctuation {
                    And => l.and(&r),
                    Xor => l.xor(&r),
                    Or => l.or(&r),
                    Nand => l.nand(&r),
                    Xnor => l.xnor(&r),
                    Nor => l.nor(&r),
                    Add => l.add(&r),
                    SubNeg => l.sub(&r),
                    Mul => l.mul(&r),
                    Div => l.div(&r),
                    Rem => l.rem(&r),
                    Pow => l.pow(&r),
                    Shl => l.shl(&r),
                    Shr => l.shr(&r),
                    Rotl => l.rotl(&r),
                    Rotr => l.rotr(&r),
                    Convert if let RunValue::Type(r) = r => l.convert(r),
                    Transmute if let RunValue::Type(r) = r => l.transmute(r),

                    Eq | Ne => l.eq(&r).map(|is_eq| {
                        RunValue::Bool(match punctuation {
                            Eq => is_eq,
                            Ne => !is_eq,
                            _ => unreachable!("guarded by outer match arm"),
                        })
                    }),

                    Lt | Gt | Le | Ge => l.cmp(&r).map(|ord| {
                        RunValue::Bool(match punctuation {
                            Lt => ord.is_some_and(Ordering::is_lt),
                            Gt => ord.is_some_and(Ordering::is_gt),
                            Le => ord.is_some_and(Ordering::is_le),
                            Ge => ord.is_some_and(Ordering::is_ge),
                            _ => unreachable!("guarded by outer match arm"),
                        })
                    }),

                    _ => unimplemented!(),
                },
            }
            .map_err(|e| e.binary(source, op, lhs, rhs))
        }

        Expr::Unary(inner) => {
            let Unary { op, operand, side } = &**inner;
            let x = evaluate(source, env, operand)?;

            if matches!(&x, RunValue::CoalesceNone) {
                return Ok(RunValue::CoalesceNone);
            }

            let punc = op
                .val
                .to_token_eq()
                .expect("only TokenEq supported for ops");
            match side {
                // postfix
                OpSide::Left => match punc {
                    TokenEq::Keyword(_keyword) => unimplemented!("no postfix keywords yet"),
                    TokenEq::Punctuation(punctuation) => match punctuation {
                        Coalesce => return Ok(RunValue::CoalesceNone),
                        _ => unimplemented!(),
                    },
                },

                // prefix
                OpSide::Right => match punc {
                    TokenEq::Keyword(keyword) => match keyword {
                        Szof => Ok(RunValue::UInt(x.sizeof())),
                        _ => unimplemented!(),
                    },
                    TokenEq::Punctuation(punctuation) => match punctuation {
                        Not => x.not(),
                        SubNeg => x.neg(),
                        Exists => Ok(x.exists()),
                        _ => unimplemented!(),
                    },
                },
            }
            .map_err(|e| e.unary(source, op, punc, operand))
        }

        Expr::Literal(literal) => {
            let Literal { value } = literal;
            match value.val {
                LexValue::BoolLiteral(b) => Ok(RunValue::Bool(b)),
                LexValue::UIntLiteral(n) => Ok(RunValue::UInt(n)),
                LexValue::SIntLiteral(n) => Ok(RunValue::SInt(n)),
                LexValue::FracLiteral(x) => Ok(RunValue::Frac(x)),
                LexValue::CharLiteral(CharLiteral { ch, .. }) => Ok(RunValue::Char(ch)),
                LexValue::TextLiteral(s) => s
                    .process()
                    .map(RunValue::Text)
                    .map_err(|e| ContextError::token_error(source, Some(value), e)),
                LexValue::Keyword(Keyword::None) => Ok(RunValue::None),

                _ => unimplemented!(),
            }
        }

        // Assumes we are accessing by value, not reference
        // TODO: how do we take references to store them in variables?
        Expr::Variable(var) => env
            .stack()
            .ulookup(var.ident.lex)
            .map(|ptr| {
                debug_assert_ne!(
                    env.stack().get_raw(ptr),
                    None,
                    "lookup should always give a valid pointer"
                );
                RunValue::UPtr(ptr)
            })
            .ok_or_else(|| {
                ContextError::token_error(source, Some(&var.ident), ErrorType::UnknownVar)
            }),

        Expr::Grouping(group) => evaluate(source, env, &group.expr),

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
                | Keyword::Rot
                | Keyword::Fun
                | Keyword::Mem
                | Keyword::Let
                | Keyword::Uni
                | Keyword::Pvt
                | Keyword::Has
                | Keyword::If
                | Keyword::Or
                | Keyword::Tor
                | Keyword::Pick
                | Keyword::Rep
                | Keyword::For
                | Keyword::In
                | Keyword::Loop
                | Keyword::Cord
                | Keyword::Halt
                | Keyword::Skip
                | Keyword::Give
                | Keyword::Emit
                | Keyword::Szof
                | Keyword::Prnt => unimplemented!("not a typename"),
            })),

            LexValue::Identifier | LexValue::Callable => unimplemented!("custom types"),

            _ => unimplemented!(),
        },
    }
}
