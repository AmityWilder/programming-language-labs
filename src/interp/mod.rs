//! Code execution

#![allow(
    clippy::missing_docs_in_private_items,
    dead_code,
    reason = "under construction"
)]

use crate::{
    error::ContextError,
    grammar::{Binary, Expr, Unary},
    scanner::token::{
        punc::Punctuation,
        value::{CharLiteral, StringLiteral, Value as TokenValue},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    UInt(usize),
    SInt(isize),
    Flt(f64),
    Char(char),
    Str(String),
}

// TODO: unsigned should be allowed to convert to signed, but signed is not allowed to convert to unsigned
impl<'src> Value {
    fn eval_rem(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_rem(r)
                .map(Self::UInt)
                .ok_or_else(|| todo!("div by zero")),

            (Self::SInt(l), Self::SInt(r)) => l
                .checked_rem(r)
                .map(Self::SInt)
                .ok_or_else(|| todo!("div by zero")),

            _ => todo!("incompatible"),
        }
    }

    fn eval_and(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l & r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l & r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l & r)),

            _ => todo!("incompatible"),
        }
    }

    fn eval_mul(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_mul(r)
                .map(Self::UInt)
                .ok_or_else(|| todo!("overflow")),

            (Self::SInt(l), Self::SInt(r)) => l
                .checked_mul(r)
                .map(Self::SInt)
                .ok_or_else(|| todo!("overflow")),

            (Self::Flt(l), Self::Flt(r)) => Ok(Self::Flt(l * r)),

            _ => todo!("incompatible"),
        }
    }

    fn eval_add(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_add(r)
                .map(Self::UInt)
                .ok_or_else(|| todo!("overflow")),

            (Self::SInt(l), Self::SInt(r)) => l
                .checked_add(r)
                .map(Self::SInt)
                .ok_or_else(|| todo!("overflow")),

            (Self::Flt(l), Self::Flt(r)) => Ok(Self::Flt(l + r)),

            (Self::Str(l), Self::Char(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Bool(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::UInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::SInt(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Str(l), Self::Flt(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Char(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Bool(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::UInt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::SInt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),
            (Self::Flt(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),

            (Self::Str(l), Self::Str(r)) => Ok(Self::Str(format!("{l}{r}"))),

            _ => todo!("incompatible"),
        }
    }

    fn eval_sub(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_sub(r)
                .map(Self::UInt)
                .ok_or_else(|| todo!("overflow")),

            (Self::SInt(l), Self::SInt(r)) => l
                .checked_sub(r)
                .map(Self::SInt)
                .ok_or_else(|| todo!("overflow")),

            (Self::Flt(l), Self::Flt(r)) => Ok(Self::Flt(l - r)),

            _ => todo!("incompatible"),
        }
    }

    fn eval_div(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => l
                .checked_div(r)
                .map(Self::UInt)
                .ok_or_else(|| todo!("div by zero")),

            (Self::SInt(l), Self::SInt(r)) => l
                .checked_div(r)
                .map(Self::SInt)
                .ok_or_else(|| todo!("div by zero")),

            _ => todo!("incompatible"),
        }
    }

    fn eval_cmp(self, rhs: Self) -> Result<std::cmp::Ordering, ContextError<'src>> {
        // TODO: more things should support cmp
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(l.cmp(&r)),

            (Self::SInt(l), Self::SInt(r)) => Ok(l.cmp(&r)),

            (Self::Flt(l), Self::Flt(r)) => Ok(l.total_cmp(&r)),

            _ => todo!("incompatible"),
        }
    }

    fn eval_xor(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l ^ r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l ^ r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l ^ r)),

            _ => todo!("incompatible"),
        }
    }

    fn eval_or(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(l | r)),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(l | r)),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(l | r)),

            _ => todo!("incompatible"),
        }
    }

    fn eval_nand(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l & r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l & r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l & r))),

            _ => todo!("incompatible"),
        }
    }

    fn eval_nor(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l | r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l | r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l | r))),

            _ => todo!("incompatible"),
        }
    }

    fn eval_xnor(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::Bool(l), Self::Bool(r)) => Ok(Self::Bool(!(l ^ r))),
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(!(l ^ r))),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(!(l ^ r))),

            _ => todo!("incompatible"),
        }
    }

    fn eval_exp(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => {
                Ok(Self::UInt(l.pow(r.try_into().map_err(|e| todo!("{e}"))?)))
            }
            (Self::SInt(l), Self::SInt(r)) => {
                Ok(Self::SInt(l.pow(r.try_into().map_err(|e| todo!("{e}"))?)))
            }
            (Self::Flt(l), Self::UInt(r)) => {
                Ok(Self::Flt(l.powi(r.try_into().map_err(|e| todo!("{e}"))?)))
            }
            (Self::Flt(l), Self::SInt(r)) => {
                Ok(Self::Flt(l.powi(r.try_into().map_err(|e| todo!("{e}"))?)))
            }
            (Self::Flt(l), Self::Flt(r)) => Ok(Self::Flt(l.powf(r))),

            _ => todo!("incompatible"),
        }
    }

    fn eval_shl(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.unbounded_shl(r.try_into().map_err(|e| todo!("{e}"))?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.unbounded_shl(r.try_into().map_err(|e| todo!("{e}"))?),
            )),

            _ => todo!("incompatible"),
        }
    }

    fn eval_shr(self, rhs: Self) -> Result<Self, ContextError<'src>> {
        match (self, rhs) {
            (Self::UInt(l), Self::UInt(r)) => Ok(Self::UInt(
                l.unbounded_shr(r.try_into().map_err(|e| todo!("{e}"))?),
            )),
            (Self::SInt(l), Self::SInt(r)) => Ok(Self::SInt(
                l.unbounded_shr(r.try_into().map_err(|e| todo!("{e}"))?),
            )),

            _ => todo!("incompatible"),
        }
    }

    fn eval_not(self) -> Result<Self, ContextError<'src>> {
        match self {
            Self::Bool(x) => Ok(Self::Bool(!x)),
            Self::UInt(x) => Ok(Self::UInt(!x)),
            Self::SInt(x) => Ok(Self::SInt(!x)),

            _ => todo!("unsupported"),
        }
    }

    fn eval_neg(self) -> Result<Self, ContextError<'src>> {
        match self {
            Self::UInt(x) => Err(todo!("unsigned cannot be negated")),
            Self::SInt(x) => x
                .checked_neg()
                .map(Self::SInt)
                .ok_or_else(|| todo!("overflow")),
            Self::Flt(x) => Ok(Self::Flt(-x)),

            _ => todo!("unsupported"),
        }
    }
}

pub fn evaluate<'src>(ast: &Expr<'src>) -> Result<Value, ContextError<'src>> {
    match ast {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;
            let lhs = evaluate(lhs)?;
            let rhs = evaluate(rhs)?;
            match op.val {
                TokenValue::Punctuation(punc) => match punc {
                    Punctuation::Rem => lhs.eval_rem(rhs),
                    Punctuation::And => lhs.eval_and(rhs),
                    Punctuation::Mul => lhs.eval_mul(rhs),
                    Punctuation::Add => lhs.eval_add(rhs),
                    Punctuation::Sub => lhs.eval_sub(rhs),
                    Punctuation::Div => lhs.eval_div(rhs),
                    Punctuation::Lt => lhs.eval_cmp(rhs).map(|ord| Value::Bool(ord.is_lt())),
                    Punctuation::Gt => lhs.eval_cmp(rhs).map(|ord| Value::Bool(ord.is_gt())),
                    Punctuation::Xor => lhs.eval_xor(rhs),
                    Punctuation::Or => lhs.eval_or(rhs),
                    Punctuation::Neq => lhs.eval_cmp(rhs).map(|ord| Value::Bool(ord.is_ne())),
                    Punctuation::Nand => lhs.eval_nand(rhs),
                    Punctuation::Nor => lhs.eval_nor(rhs),
                    Punctuation::Xnor => lhs.eval_xnor(rhs),
                    Punctuation::Exp => lhs.eval_exp(rhs),
                    Punctuation::Le => lhs.eval_cmp(rhs).map(|ord| Value::Bool(ord.is_le())),
                    Punctuation::Shl => lhs.eval_shl(rhs),
                    Punctuation::Eq => lhs.eval_cmp(rhs).map(|ord| Value::Bool(ord.is_eq())),
                    Punctuation::Ge => lhs.eval_cmp(rhs).map(|ord| Value::Bool(ord.is_ge())),
                    Punctuation::Shr => lhs.eval_shr(rhs),
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }

        Expr::Unary(inner) => {
            let Unary { op, rhs } = &**inner;
            let rhs = evaluate(rhs)?;
            match op.val {
                TokenValue::Punctuation(punc) => match punc {
                    Punctuation::Not => rhs.eval_not(),
                    Punctuation::Sub => rhs.eval_neg(),
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }

        Expr::Literal(token) => match token.val {
            TokenValue::BoolLiteral(b) => Ok(Value::Bool(b)),
            TokenValue::UIntLiteral(n) => Ok(Value::UInt(n)),
            TokenValue::SIntLiteral(n) => Ok(Value::SInt(n)),
            TokenValue::FltLiteral(x) => Ok(Value::Flt(x)),
            TokenValue::CharLiteral(CharLiteral { ch, .. }) => Ok(Value::Char(ch)),
            TokenValue::StringLiteral(s) => s
                .process()
                .map(|StringLiteral { text, .. }| Value::Str(text))
                .map_err(|e| todo!("{e}")),
            _ => unimplemented!("not a literal"),
        },

        Expr::Grouping(inner) => evaluate(inner),
    }
}
