use crate::{
    error::{ContextError, ErrorType, OpSide},
    eval::ValueType,
    grammar::ast::{AstNodeFinal, Binary, Expr, Unary},
    scanner::token::{
        Token,
        punc::Punctuation,
        value::{Keyword, LexValue, TokenEq},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PpOpError<'src> {
    Incompatible { l: PpValue<'src>, r: PpValue<'src> },
    Unsupported { x: PpValue<'src> },
}

impl<'src> PpOpError<'src> {
    fn contextualize(
        self,
        source: &'src str,
        op: &Token<'src>,
        lhs: Option<&Expr<'src>>,
        rhs: Option<&Expr<'src>>,
    ) -> ContextError<'src> {
        match self {
            Self::Incompatible { l, r } => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::PreIncompatible {
                    op: op
                        .val
                        .to_token_eq()
                        .expect("only TokenEq operators supported"),
                    l_range: lhs.expect("binary should have lhs").range(source),
                    r_range: rhs.expect("binary should have rhs").range(source),
                    l_ty: l.as_type(),
                    r_ty: r.as_type(),
                },
            ),
            Self::Unsupported { x } => ContextError::token_error(
                source,
                Some(*op),
                ErrorType::PreUnsupported {
                    op: op
                        .val
                        .to_token_eq()
                        .expect("only TokenEq operators supported"),
                    r_ty: x.as_type(),
                    r_range: rhs.expect("unary prefix should have rhs").range(source),
                },
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PpType {
    Typename,
    Bool,
    UInt,
    SInt,
    Macro,
}

impl PpType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Typename => "typename",
            Self::Bool => "bool",
            Self::UInt => "uint",
            Self::SInt => "sint",
            Self::Macro => "macro identifier",
        }
    }

    pub const fn to_value_type(self) -> ValueType {
        match self {
            Self::Typename => ValueType::Type,
            Self::Bool => ValueType::Bool,
            Self::UInt => ValueType::UInt,
            Self::SInt => ValueType::SInt,
            Self::Macro => ValueType::Macro,
        }
    }
}

impl std::fmt::Display for PpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PpValue<'src> {
    // types (for size operations)
    Bool,
    Uint,
    Sint,
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

impl<'src> PpValue<'src> {
    const fn as_type(self) -> PpType {
        match self {
            Self::Bool | Self::Uint | Self::Sint | Self::Frac | Self::Char | Self::Text => {
                PpType::Typename
            }
            Self::BoolLiteral(_) => PpType::Bool,
            Self::UIntLiteral(_) => PpType::UInt,
            Self::SIntLiteral(_) => PpType::SInt,
            Self::Macro(_) => PpType::Macro,
        }
    }

    const fn and(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => Ok(PpValue::BoolLiteral(l & r)),
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => Ok(PpValue::UIntLiteral(l & r)),
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => Ok(PpValue::SIntLiteral(l & r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn or(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => Ok(PpValue::BoolLiteral(l | r)),
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => Ok(PpValue::UIntLiteral(l | r)),
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => Ok(PpValue::SIntLiteral(l | r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn xor(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => Ok(PpValue::BoolLiteral(l ^ r)),
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => Ok(PpValue::UIntLiteral(l ^ r)),
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => Ok(PpValue::SIntLiteral(l ^ r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn nand(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => {
                Ok(PpValue::BoolLiteral(!(l & r)))
            }
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(!(l & r)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(!(l & r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn nor(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => {
                Ok(PpValue::BoolLiteral(!(l | r)))
            }
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(!(l | r)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(!(l | r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn xnor(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => {
                Ok(PpValue::BoolLiteral(!(l ^ r)))
            }
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(!(l ^ r)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(!(l ^ r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn add(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(l.saturating_add(r)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(l.saturating_add(r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn sub(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(l.saturating_sub(r)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(l.saturating_sub(r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn mul(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(l.saturating_mul(r)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(l.saturating_mul(r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    fn div(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(l.checked_div(r).unwrap_or(usize::MAX)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(l.checked_div(r).unwrap_or(isize::MAX)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    fn rem(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => {
                Ok(PpValue::UIntLiteral(l.checked_rem(r).unwrap_or(0)))
            }
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => {
                Ok(PpValue::SIntLiteral(l.checked_rem(r).unwrap_or(0)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn eq(self, rhs: Self) -> Result<bool, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::BoolLiteral(l), PpValue::BoolLiteral(r)) => Ok(l == r),
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => Ok(l == r),
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => Ok(l == r),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn cmp(self, rhs: Self) -> Result<std::cmp::Ordering, PpOpError<'src>> {
        match (self, rhs) {
            (PpValue::UIntLiteral(l), PpValue::UIntLiteral(r)) => Ok(l.cmp(&r)),
            (PpValue::SIntLiteral(l), PpValue::SIntLiteral(r)) => Ok(l.cmp(&r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn size(self) -> usize {
        match self {
            // TBD: should literals produce an error?
            PpValue::BoolLiteral(_) | PpValue::Bool => std::mem::size_of::<bool>(),
            PpValue::UIntLiteral(_) | PpValue::Uint => std::mem::size_of::<usize>(),
            PpValue::SIntLiteral(_) | PpValue::Sint => std::mem::size_of::<isize>(),
            PpValue::Frac => std::mem::size_of::<f64>(),
            PpValue::Char => std::mem::size_of::<char>(),
            PpValue::Text => std::mem::size_of::<String>(),
            PpValue::Macro(_) => 0,
        }
    }

    const fn neg(self) -> Result<Self, PpOpError<'src>> {
        match self {
            PpValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(x.saturating_neg())),
            x => Err(PpOpError::Unsupported { x }),
        }
    }

    const fn not(self) -> Result<Self, PpOpError<'src>> {
        match self {
            PpValue::UIntLiteral(x) => Ok(PpValue::UIntLiteral(!x)),
            PpValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(!x)),
            PpValue::BoolLiteral(x) => Ok(PpValue::BoolLiteral(!x)),
            x => Err(PpOpError::Unsupported { x }),
        }
    }

    fn exists<F>(self, macro_exists: F) -> Result<bool, PpOpError<'src>>
    where
        F: FnOnce(&str) -> bool,
    {
        match self {
            PpValue::Macro(x) => Ok(macro_exists(x)),
            x => Err(PpOpError::Unsupported { x }),
        }
    }
}

pub fn preproc_eval<'src, F>(
    source: &'src str,
    expr: &Expr<'src>,
    mexists: &mut F,
) -> Result<PpValue<'src>, ContextError<'src>>
where
    F: FnMut(&str) -> bool,
{
    // TODO: Some operations are valid for Expr but not in preprocessing
    match expr {
        Expr::Ternary(inner) => Err(ContextError::node_error(
            source,
            &**inner,
            ErrorType::InvalidPreOperator,
        )),

        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;

            let punc = op.val.to_token_eq().expect("only TokenEq ops supported");
            match punc {
                TokenEq::Keyword(_keyword) => Err(ContextError::token_error(
                    source,
                    Some(*op),
                    ErrorType::InvalidPreOperator,
                )),

                TokenEq::Punctuation(punctuation) => {
                    match punctuation {
                        Punctuation::And => preproc_eval(source, lhs, mexists)?
                            .and(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Or => preproc_eval(source, lhs, mexists)?
                            .or(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Xor => preproc_eval(source, lhs, mexists)?
                            .xor(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Nand => preproc_eval(source, lhs, mexists)?
                            .nand(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Nor => preproc_eval(source, lhs, mexists)?
                            .nor(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Xnor => preproc_eval(source, lhs, mexists)?
                            .xnor(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Add => preproc_eval(source, lhs, mexists)?
                            .add(preproc_eval(source, rhs, mexists)?),

                        Punctuation::SubNeg => preproc_eval(source, lhs, mexists)?
                            .sub(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Mul => preproc_eval(source, lhs, mexists)?
                            .mul(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Div => preproc_eval(source, lhs, mexists)?
                            .div(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Rem => preproc_eval(source, lhs, mexists)?
                            .rem(preproc_eval(source, rhs, mexists)?),

                        Punctuation::Eq | Punctuation::Ne => preproc_eval(source, lhs, mexists)?
                            .eq(preproc_eval(source, rhs, mexists)?)
                            .map(|is_eq| {
                                PpValue::BoolLiteral(match punctuation {
                                    Punctuation::Eq => is_eq,
                                    Punctuation::Ne => !is_eq,
                                    _ => unreachable!("guarded by outer match arm"),
                                })
                            }),

                        Punctuation::Lt | Punctuation::Le | Punctuation::Gt | Punctuation::Ge => {
                            preproc_eval(source, lhs, mexists)?
                                .cmp(preproc_eval(source, rhs, mexists)?)
                                .map(|ord| {
                                    PpValue::BoolLiteral(match punctuation {
                                        Punctuation::Ne => ord.is_ne(),
                                        Punctuation::Lt => ord.is_lt(),
                                        Punctuation::Le => ord.is_le(),
                                        Punctuation::Gt => ord.is_gt(),
                                        Punctuation::Ge => ord.is_ge(),
                                        _ => unreachable!("guarded by outer match arm"),
                                    })
                                })
                        }

                        _ => {
                            return Err(ContextError::token_error(
                                source,
                                Some(*op),
                                ErrorType::InvalidPreOperator,
                            ));
                        }
                    }
                    .map_err(|e| e.contextualize(source, op, Some(lhs), Some(rhs)))
                }
            }
        }

        Expr::Unary(inner) => {
            let Unary { op, operand, side } = &**inner;

            let punc = op.val.to_token_eq().expect("only TokenEq ops supported");

            match side {
                OpSide::Left => {
                    return Err(ContextError::token_error(
                        source,
                        Some(*op),
                        ErrorType::InvalidPreOperator,
                    ));
                }

                OpSide::Right => match punc {
                    TokenEq::Keyword(keyword) => match keyword {
                        Keyword::Szof => Ok(PpValue::UIntLiteral(
                            preproc_eval(source, operand, mexists)?.size(),
                        )),

                        _ => {
                            return Err(ContextError::token_error(
                                source,
                                Some(*op),
                                ErrorType::InvalidPreOperator,
                            ));
                        }
                    },
                    TokenEq::Punctuation(punctuation) => match punctuation {
                        Punctuation::SubNeg => preproc_eval(source, operand, mexists)?.neg(),

                        Punctuation::Not => preproc_eval(source, operand, mexists)?.not(),

                        Punctuation::Exists => preproc_eval(source, operand, mexists)?
                            .exists(mexists)
                            .map(PpValue::BoolLiteral),

                        _ => {
                            return Err(ContextError::token_error(
                                source,
                                Some(*op),
                                ErrorType::InvalidPreOperator,
                            ));
                        }
                    },
                },
            }
            .map_err(|e| e.contextualize(source, op, None, Some(operand)))
        }

        Expr::Literal(inner) => match inner.val {
            LexValue::UIntLiteral(x) => Ok(PpValue::UIntLiteral(x)),
            LexValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(x)),
            LexValue::BoolLiteral(x) => Ok(PpValue::BoolLiteral(x)),
            LexValue::Macro => Ok(PpValue::Macro(inner.lex)),

            _ => Err(ContextError::token_error(
                source,
                Some(*inner),
                ErrorType::InvalidPreLiteral,
            )),
        },

        Expr::Grouping(inner) => preproc_eval(source, &inner.expr, mexists),

        Expr::Type(inner) => {
            if let Some(or_ty) = inner.or_ty.as_ref() {
                return Err(ContextError::node_error(
                    source,
                    or_ty,
                    ErrorType::PreOrType,
                ));
            }
            match inner.name.val {
                LexValue::Keyword(Keyword::Bool) => Ok(PpValue::Bool),
                LexValue::Keyword(Keyword::Sint) => Ok(PpValue::Sint),
                LexValue::Keyword(Keyword::Uint) => Ok(PpValue::Uint),
                LexValue::Keyword(Keyword::Frac) => Ok(PpValue::Frac),
                LexValue::Keyword(Keyword::Char) => Ok(PpValue::Char),
                LexValue::Keyword(Keyword::Text) => Ok(PpValue::Text),
                _ => Err(ContextError::token_error(
                    source,
                    Some(inner.name),
                    ErrorType::PreOrType,
                )),
            }
        }
    }
}
