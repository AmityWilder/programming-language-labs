use crate::{
    error::{ContextError, ErrorType, OpSide},
    grammar::ast::{AstNodeFinal, Binary, BlockExpr, Expr, Literal, TypenameKeyword, Unary},
    scanner::token::{
        SpecToken,
        value::{LexValue, Operator},
    },
    types::fsize,
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
        op: &SpecToken<'src, Operator>,
        lhs: Option<&Expr<'src>>,
        rhs: Option<&Expr<'src>>,
    ) -> ContextError<'src> {
        match self {
            Self::Incompatible { l, r } => ContextError::token_error(
                source,
                Some(op),
                ErrorType::PreIncompatible {
                    op: op.val,
                    l_range: lhs.expect("binary should have lhs").range(source),
                    r_range: rhs.expect("binary should have rhs").range(source),
                    l_ty: l.as_type(),
                    r_ty: r.as_type(),
                },
            ),
            Self::Unsupported { x } => ContextError::token_error(
                source,
                Some(op),
                ErrorType::PreUnsupported {
                    op: op.val,
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
    Flpt,
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
            Self::Bool | Self::Uint | Self::Sint | Self::Flpt | Self::Char | Self::Text => {
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
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(Self::BoolLiteral(l & r)),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(Self::UIntLiteral(l & r)),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(Self::SIntLiteral(l & r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn or(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(Self::BoolLiteral(l | r)),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(Self::UIntLiteral(l | r)),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(Self::SIntLiteral(l | r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn xor(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(Self::BoolLiteral(l ^ r)),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(Self::UIntLiteral(l ^ r)),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(Self::SIntLiteral(l ^ r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn nand(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(Self::BoolLiteral(!(l & r))),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(Self::UIntLiteral(!(l & r))),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(Self::SIntLiteral(!(l & r))),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn nor(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(Self::BoolLiteral(!(l | r))),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(Self::UIntLiteral(!(l | r))),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(Self::SIntLiteral(!(l | r))),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn xnor(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(Self::BoolLiteral(!(l ^ r))),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(Self::UIntLiteral(!(l ^ r))),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(Self::SIntLiteral(!(l ^ r))),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn add(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => {
                Ok(Self::UIntLiteral(l.saturating_add(r)))
            }
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => {
                Ok(Self::SIntLiteral(l.saturating_add(r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn sub(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => {
                Ok(Self::UIntLiteral(l.saturating_sub(r)))
            }
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => {
                Ok(Self::SIntLiteral(l.saturating_sub(r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn mul(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => {
                Ok(Self::UIntLiteral(l.saturating_mul(r)))
            }
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => {
                Ok(Self::SIntLiteral(l.saturating_mul(r)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    fn div(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => {
                Ok(Self::UIntLiteral(l.checked_div(r).unwrap_or(usize::MAX)))
            }
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => {
                Ok(Self::SIntLiteral(l.checked_div(r).unwrap_or(isize::MAX)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    fn rem(self, rhs: Self) -> Result<Self, PpOpError<'src>> {
        match (self, rhs) {
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => {
                Ok(Self::UIntLiteral(l.checked_rem(r).unwrap_or(0)))
            }
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => {
                Ok(Self::SIntLiteral(l.checked_rem(r).unwrap_or(0)))
            }
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn eq(self, rhs: Self) -> Result<bool, PpOpError<'src>> {
        match (self, rhs) {
            (Self::BoolLiteral(l), Self::BoolLiteral(r)) => Ok(l == r),
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(l == r),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(l == r),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn cmp(self, rhs: Self) -> Result<std::cmp::Ordering, PpOpError<'src>> {
        match (self, rhs) {
            (Self::UIntLiteral(l), Self::UIntLiteral(r)) => Ok(l.cmp(&r)),
            (Self::SIntLiteral(l), Self::SIntLiteral(r)) => Ok(l.cmp(&r)),
            (l, r) => Err(PpOpError::Incompatible { l, r }),
        }
    }

    const fn size(self) -> usize {
        match self {
            // TBD: should literals produce an error?
            Self::BoolLiteral(_) | Self::Bool => std::mem::size_of::<bool>(),
            Self::UIntLiteral(_) | Self::Uint => std::mem::size_of::<usize>(),
            Self::SIntLiteral(_) | Self::Sint => std::mem::size_of::<isize>(),
            Self::Flpt => std::mem::size_of::<fsize>(),
            Self::Char => std::mem::size_of::<char>(),
            Self::Text => std::mem::size_of::<String>(),
            Self::Macro(_) => 0,
        }
    }

    const fn neg(self) -> Result<Self, PpOpError<'src>> {
        match self {
            Self::SIntLiteral(x) => Ok(Self::SIntLiteral(x.saturating_neg())),
            x => Err(PpOpError::Unsupported { x }),
        }
    }

    const fn not(self) -> Result<Self, PpOpError<'src>> {
        match self {
            Self::UIntLiteral(x) => Ok(Self::UIntLiteral(!x)),
            Self::SIntLiteral(x) => Ok(Self::SIntLiteral(!x)),
            Self::BoolLiteral(x) => Ok(Self::BoolLiteral(!x)),
            x => Err(PpOpError::Unsupported { x }),
        }
    }

    fn exists<F>(self, macro_exists: F) -> Result<bool, PpOpError<'src>>
    where
        F: FnOnce(&str) -> bool,
    {
        match self {
            Self::Macro(x) => Ok(macro_exists(x)),
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
    use Operator::{
        Add, And, Div, Eq, Exists, Ge, Gt, Le, Lt, Mul, Nand, Ne, Nor, Not, Or, Rem, SubNeg as Sub,
        SubNeg as Neg, Szof, Xnor, Xor,
    };

    // TODO: Some operations are valid for Expr but not in preprocessing
    match expr {
        Expr::Binary(inner) => {
            let Binary { lhs, op, rhs } = &**inner;

            match op.val {
                And => preproc_eval(source, lhs, mexists)?.and(preproc_eval(source, rhs, mexists)?),

                Or => preproc_eval(source, lhs, mexists)?.or(preproc_eval(source, rhs, mexists)?),

                Xor => preproc_eval(source, lhs, mexists)?.xor(preproc_eval(source, rhs, mexists)?),

                Nand => {
                    preproc_eval(source, lhs, mexists)?.nand(preproc_eval(source, rhs, mexists)?)
                }

                Nor => preproc_eval(source, lhs, mexists)?.nor(preproc_eval(source, rhs, mexists)?),

                Xnor => {
                    preproc_eval(source, lhs, mexists)?.xnor(preproc_eval(source, rhs, mexists)?)
                }

                Add => preproc_eval(source, lhs, mexists)?.add(preproc_eval(source, rhs, mexists)?),

                Sub => preproc_eval(source, lhs, mexists)?.sub(preproc_eval(source, rhs, mexists)?),

                Mul => preproc_eval(source, lhs, mexists)?.mul(preproc_eval(source, rhs, mexists)?),

                Div => preproc_eval(source, lhs, mexists)?.div(preproc_eval(source, rhs, mexists)?),

                Rem => preproc_eval(source, lhs, mexists)?.rem(preproc_eval(source, rhs, mexists)?),

                Eq | Ne => preproc_eval(source, lhs, mexists)?
                    .eq(preproc_eval(source, rhs, mexists)?)
                    .map(|is_eq| {
                        PpValue::BoolLiteral(match op.val {
                            Eq => is_eq,
                            Ne => !is_eq,
                            _ => unreachable!("guarded by outer match arm"),
                        })
                    }),

                Lt | Le | Gt | Ge => preproc_eval(source, lhs, mexists)?
                    .cmp(preproc_eval(source, rhs, mexists)?)
                    .map(|ord| {
                        PpValue::BoolLiteral(match op.val {
                            Ne => ord.is_ne(),
                            Lt => ord.is_lt(),
                            Le => ord.is_le(),
                            Gt => ord.is_gt(),
                            Ge => ord.is_ge(),
                            _ => unreachable!("guarded by outer match arm"),
                        })
                    }),

                _ => {
                    return Err(ContextError::token_error(
                        source,
                        Some(op),
                        ErrorType::InvalidPreOperator,
                    ));
                }
            }
            .map_err(|e| e.contextualize(source, op, Some(lhs), Some(rhs)))
        }

        Expr::Unary(inner) => {
            let Unary { op, operand, side } = &**inner;

            match side {
                OpSide::Postfix => {
                    return Err(ContextError::token_error(
                        source,
                        Some(op),
                        ErrorType::InvalidPreOperator,
                    ));
                }

                OpSide::Prefix => match op.val {
                    Szof => Ok(PpValue::UIntLiteral(
                        preproc_eval(source, operand, mexists)?.size(),
                    )),

                    Neg => preproc_eval(source, operand, mexists)?.neg(),

                    Not => preproc_eval(source, operand, mexists)?.not(),

                    Exists => preproc_eval(source, operand, mexists)?
                        .exists(mexists)
                        .map(PpValue::BoolLiteral),

                    _ => {
                        return Err(ContextError::token_error(
                            source,
                            Some(op),
                            ErrorType::InvalidPreOperator,
                        ));
                    }
                },
            }
            .map_err(|e| e.contextualize(source, op, None, Some(operand)))
        }

        Expr::Literal(inner) => {
            let Literal { value } = inner;
            match value.val {
                LexValue::UIntLiteral(x) => Ok(PpValue::UIntLiteral(x)),
                LexValue::SIntLiteral(x) => Ok(PpValue::SIntLiteral(x)),
                LexValue::BoolLiteral(x) => Ok(PpValue::BoolLiteral(x)),
                LexValue::Macro => Ok(PpValue::Macro(value.lex)),

                _ => Err(ContextError::token_error(
                    source,
                    Some(value),
                    ErrorType::InvalidPreLiteral,
                )),
            }
        }

        Expr::Directive(inner) => Err(ContextError::token_error(
            source,
            Some(&inner.kw),
            ErrorType::InvalidPreOperator, // TODO: this isn't an operator
        )),

        Expr::Block(inner) => Err(ContextError::token_error(
            source,
            Some(match inner {
                BlockExpr::If(inner) => &inner.if_kw,
                BlockExpr::Loop(inner) => &inner.kw,
                BlockExpr::Block(inner) => &inner.open,
            }),
            ErrorType::InvalidPreOperator, // TODO: this isn't an operator
        )),

        // TBD: should `pvt` variables be allowed?
        Expr::Variable(inner) => {
            return Err(ContextError::node_error(
                source,
                inner,
                ErrorType::InvalidPreLiteral, // TODO: this might need its own error
            ));
        }

        Expr::Grouping(inner) => preproc_eval(source, &inner.expr, mexists),

        Expr::Type(inner) => {
            if let Some(or_ty) = inner.or_ty.as_ref() {
                return Err(ContextError::node_error(
                    source,
                    or_ty,
                    ErrorType::PreOrType,
                ));
            }
            if let Some(x) = inner.name.name.val {
                if !inner.name.path.is_empty() {
                    todo!("error: primitives are global scope");
                }
                if inner.name.args.is_some() {
                    todo!("error: primitives do not have generic parameters");
                }
                match x {
                    TypenameKeyword::Bool => Ok(PpValue::Bool),
                    TypenameKeyword::Sint => Ok(PpValue::Sint),
                    TypenameKeyword::Uint => Ok(PpValue::Uint),
                    TypenameKeyword::Flpt => Ok(PpValue::Flpt),
                    TypenameKeyword::Char => Ok(PpValue::Char),
                    TypenameKeyword::Text => Ok(PpValue::Text),

                    TypenameKeyword::None | TypenameKeyword::Nevr | TypenameKeyword::Fail => {
                        todo!("preprocessor does not support `{x}`")
                    }
                }
            } else {
                todo!("error: custom types do not exist during preprocessing")
            }
        }
    }
}
