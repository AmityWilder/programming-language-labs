//! Code execution

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::{ContextError, ErrorType},
    grammar::{Binary, Expr, Unary},
    scanner::token::{
        punc::Punctuation,
        value::{CharLiteral, StringLiteral, Value as TokenValue},
    },
};

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
}

// TODO: unsigned should be allowed to convert to signed, but signed is not allowed to convert to unsigned
impl<'src> Value {
    fn eval_rem(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_rem(r)
                    .map(Self::UInt)
                    .ok_or_else(|| ErrorType::DivByZero {
                        zero: rhs_expr.range(source),
                    })
            }

            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_rem(r)
                    .map(Self::SInt)
                    .ok_or_else(|| ErrorType::DivByZero {
                        zero: rhs_expr.range(source),
                    })
            }

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Rem,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_and(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l & r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l & r)),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::And,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_mul(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_mul(r).map(Self::UInt).ok_or(ErrorType::Overflow)
            }

            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_mul(r).map(Self::SInt).ok_or(ErrorType::Overflow)
            }

            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l * r)),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Mul,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_add(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_add(r).map(Self::UInt).ok_or(ErrorType::Overflow)
            }

            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_add(r).map(Self::SInt).ok_or(ErrorType::Overflow)
            }

            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l + r)),

            (Self::Str(l), Self::Char(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Bool(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::UInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::SInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Frac(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Char(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Bool(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::UInt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::SInt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Frac(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Str(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Add,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_sub(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_sub(r).map(Self::UInt).ok_or(ErrorType::Overflow)
            }

            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_sub(r).map(Self::SInt).ok_or(ErrorType::Overflow)
            }

            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l - r)),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Sub,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_div(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => {
                l.checked_div(r)
                    .map(Self::UInt)
                    .ok_or_else(|| ErrorType::DivByZero {
                        zero: rhs_expr.range(source),
                    })
            }

            (Self::SInt(l), Self::SInt(r)) => {
                l.checked_div(r)
                    .map(Self::SInt)
                    .ok_or_else(|| ErrorType::DivByZero {
                        zero: rhs_expr.range(source),
                    })
            }

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Div,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_cmp(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
        op: Punctuation,
    ) -> Result<std::cmp::Ordering, ErrorType<'src>> {
        // TODO: more things should support cmp
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(l.cmp(&r)),

            (Self::SInt(l), Self::SInt(r)) => Ok(l.cmp(&r)),

            (Self::Frac(l), Self::Frac(r)) => Ok(l.total_cmp(&r)),

            (l, r) => Err(ErrorType::Incompatible {
                op,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_xor(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l ^ r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l ^ r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l ^ r)),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Xor,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_or(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l | r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l | r)),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Or,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_nand(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l & r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l & r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l & r))),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Nand,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_nor(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l | r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l | r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l | r))),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Nor,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_xnor(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l ^ r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l ^ r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l ^ r))),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Xnor,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_exp(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.pow(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.pow(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),
            (Self::Frac(l), Self::UInt(r)) => Ok(Self::Frac(
                l.powi(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),
            (Self::Frac(l), Self::SInt(r)) => Ok(Self::Frac(
                l.powi(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),
            (Self::Frac(l), Self::Frac(r)) => Ok(Self::Frac(l.powf(r))),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Exp,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_shl(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.unbounded_shl(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.unbounded_shl(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Shl,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_shr(
        self,
        rhs: Self,
        source: &'src str,
        lhs_expr: &Expr<'src>,
        rhs_expr: &Expr<'src>,
    ) -> Result<Self, ErrorType<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.unbounded_shr(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.unbounded_shr(r.try_into().map_err(ErrorType::FailedConvert)?),
            )),

            (l, r) => Err(ErrorType::Incompatible {
                op: Punctuation::Shr,
                lhs: (l.as_type(), lhs_expr.range(source)),
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_not(self, source: &'src str, rhs_expr: &Expr<'src>) -> Result<Self, ErrorType<'src>> {
        match self {
            Self::Bool(x) => Ok(Self::Bool(!x)),
            Self::UInt(x) => Ok(Self::UInt(!x)),
            Self::SInt(x) => Ok(Self::SInt(!x)),

            r => Err(ErrorType::Unsupported {
                op: Punctuation::Not,
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }

    fn eval_neg(self, source: &'src str, rhs_expr: &Expr<'src>) -> Result<Self, ErrorType<'src>> {
        match self {
            Self::UInt(x) => Ok(Self::SInt(
                isize::try_from(x).map_err(ErrorType::FailedConvert)?,
            )),

            Self::SInt(x) => x.checked_neg().map(Self::SInt).ok_or(ErrorType::Overflow),

            Self::Frac(x) => Ok(Self::Frac(-x)),

            r => Err(ErrorType::Unsupported {
                op: Punctuation::Sub,
                rhs: (r.as_type(), rhs_expr.range(source)),
            }),
        }
    }
}

pub fn evaluate<'src>(source: &'src str, ast: &Expr<'src>) -> Result<Value, ContextError<'src>> {
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            match op.val {
                TokenValue::Punctuation(punc) => match punc {
                    Punctuation::Rem => {
                        evaluate(source, lhs)?.eval_rem(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::And => {
                        evaluate(source, lhs)?.eval_and(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Mul => {
                        evaluate(source, lhs)?.eval_mul(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Add => {
                        evaluate(source, lhs)?.eval_add(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Sub => {
                        evaluate(source, lhs)?.eval_sub(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Div => {
                        evaluate(source, lhs)?.eval_div(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Lt => evaluate(source, lhs)?
                        .eval_cmp(evaluate(source, rhs)?, source, lhs, rhs, punc)
                        .map(|ord| Value::Bool(ord.is_lt())),
                    Punctuation::Gt => evaluate(source, lhs)?
                        .eval_cmp(evaluate(source, rhs)?, source, lhs, rhs, punc)
                        .map(|ord| Value::Bool(ord.is_gt())),
                    Punctuation::Xor => {
                        evaluate(source, lhs)?.eval_xor(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Or => {
                        evaluate(source, lhs)?.eval_or(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Neq => evaluate(source, lhs)?
                        .eval_cmp(evaluate(source, rhs)?, source, lhs, rhs, punc)
                        .map(|ord| Value::Bool(ord.is_ne())),
                    Punctuation::Nand => {
                        evaluate(source, lhs)?.eval_nand(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Nor => {
                        evaluate(source, lhs)?.eval_nor(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Xnor => {
                        evaluate(source, lhs)?.eval_xnor(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Exp => {
                        evaluate(source, lhs)?.eval_exp(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Le => evaluate(source, lhs)?
                        .eval_cmp(evaluate(source, rhs)?, source, lhs, rhs, punc)
                        .map(|ord| Value::Bool(ord.is_le())),
                    Punctuation::Shl => {
                        evaluate(source, lhs)?.eval_shl(evaluate(source, rhs)?, source, lhs, rhs)
                    }
                    Punctuation::Eq => evaluate(source, lhs)?
                        .eval_cmp(evaluate(source, rhs)?, source, lhs, rhs, punc)
                        .map(|ord| Value::Bool(ord.is_eq())),
                    Punctuation::Ge => evaluate(source, lhs)?
                        .eval_cmp(evaluate(source, rhs)?, source, lhs, rhs, punc)
                        .map(|ord| Value::Bool(ord.is_ge())),
                    Punctuation::Shr => {
                        evaluate(source, lhs)?.eval_shr(evaluate(source, rhs)?, source, lhs, rhs)
                    }

                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
            .map_err(|e| (op, e))
        }

        Expr::Unary(inner) => {
            let Unary { op, rhs } = &**inner;
            match op.val {
                TokenValue::Punctuation(punc) => match punc {
                    Punctuation::Not => evaluate(source, rhs)?.eval_not(source, rhs),
                    Punctuation::Sub => evaluate(source, rhs)?.eval_neg(source, rhs),
                    Punctuation::MacroStringify => Ok(Value::Str(rhs.to_string())),

                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
            .map_err(|e| (op, e))
        }

        Expr::Literal(token) => match token.val {
            TokenValue::BoolLiteral(b) => Ok(Value::Bool(b)),
            TokenValue::UIntLiteral(n) => Ok(Value::UInt(n)),
            TokenValue::SIntLiteral(n) => Ok(Value::SInt(n)),
            TokenValue::FltLiteral(x) => Ok(Value::Frac(x)),
            TokenValue::CharLiteral(CharLiteral { ch, .. }) => Ok(Value::Char(ch)),
            TokenValue::StringLiteral(s) => s
                .process()
                .map(|StringLiteral { text, .. }| Value::Str(text)),

            _ => unimplemented!(),
        }
        .map_err(|e| (token, e)),

        Expr::Grouping(group) => Ok(evaluate(source, &group.expr)?),
    }
    .map_err(|(token, err)| ContextError::token_error(source, Some(*token), err))
}
