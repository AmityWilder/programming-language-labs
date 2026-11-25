use crate::{
    analyzer::types::PrimitiveType,
    error::{Diagnostic, DiagnosticKind},
    lex::token::{Operator, TokenKind, keyword::Keyword, punc::OpSide},
    pre::{StreamSpan, ast},
    spec_ops::{BitRotate as _, BitShift as _},
    src_code::{map::SourceMap, span::FileId},
    types::{byte, flpt, nevr, none, sint, text, uint},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreVal<M> {
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Char(char),
    Text(text),

    Typename(PrimitiveType),
    Macro(M),
}

impl<T: std::fmt::Display> std::fmt::Display for PreVal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(x) => x.fmt(f),
            Self::Byte(x) => x.fmt(f),
            Self::Uint(x) => x.fmt(f),
            Self::Sint(x) => x.fmt(f),
            Self::Char(x) => x.fmt(f),
            Self::Text(x) => x.fmt(f),
            Self::Typename(x) => x.fmt(f),
            Self::Macro(x) => x.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct PreBinary<'src> {
    op_span: StreamSpan,
    op: Operator,
    lhs: PreExpr<'src>,
    rhs: PreExpr<'src>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct PreUnary<'src> {
    op_span: StreamSpan,
    op: Operator,
    side: OpSide,
    operand: PreExpr<'src>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
struct PreGroup<'src> {
    /// `(`
    open: StreamSpan,
    expr: PreExpr<'src>,
    /// `)`
    close: StreamSpan,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
enum PreExpr<'src> {
    Binary(Box<PreBinary<'src>>),
    Unary(Box<PreUnary<'src>>),
    Group(Box<PreGroup<'src>>),
    Value(PreVal<&'src str>),
}

impl<'src> PreExpr<'src> {
    fn eval(self, source: &'src SourceMap) -> Result<PreVal<&'src str>, Diagnostic> {
        match self {
            PreExpr::Binary(inner) => {
                let lhs = inner.lhs.eval(source)?;
                let rhs = inner.rhs.eval(source)?;
                Ok(match inner.op {
                    Operator::And => match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => PreVal::Bool(l & r),
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l & r),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l & r),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l & r),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Or => match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => PreVal::Bool(l | r),
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l | r),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l | r),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l | r),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Xor => match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => PreVal::Bool(l ^ r),
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l ^ r),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l ^ r),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l ^ r),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Nand => match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => PreVal::Bool(!(l & r)),
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(!(l & r)),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(!(l & r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(!(l & r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Nor => match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => PreVal::Bool(!(l | r)),
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(!(l | r)),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(!(l | r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(!(l | r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Xnor => match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => PreVal::Bool(!(l ^ r)),
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(!(l ^ r)),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(!(l ^ r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(!(l ^ r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Add => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l.wrapping_add(r)),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.wrapping_add(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.wrapping_add(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Sub => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l.wrapping_sub(r)),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.wrapping_sub(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.wrapping_sub(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Mul => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l.wrapping_mul(r)),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.wrapping_mul(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.wrapping_mul(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Div => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => {
                            PreVal::Byte(l.checked_div(r).unwrap_or(byte::MAX))
                        }
                        (PreVal::Uint(l), PreVal::Uint(r)) => {
                            PreVal::Uint(l.checked_div(r).unwrap_or(uint::MAX))
                        }
                        (PreVal::Sint(l), PreVal::Sint(r)) => {
                            PreVal::Sint(l.checked_div(r).unwrap_or(sint::MAX))
                        }

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Rem => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => {
                            PreVal::Byte(l.checked_rem(r).unwrap_or(l))
                        }
                        (PreVal::Uint(l), PreVal::Uint(r)) => {
                            PreVal::Uint(l.checked_rem(r).unwrap_or(l))
                        }
                        (PreVal::Sint(l), PreVal::Sint(r)) => {
                            PreVal::Sint(l.checked_rem(r).unwrap_or(l))
                        }

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Shl => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => {
                            PreVal::Byte(l.unbounded_shl(r.into()))
                        }
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.bshl(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.bshl(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Shr => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => {
                            PreVal::Byte(l.unbounded_shr(r.into()))
                        }
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.bshr(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.bshr(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Rotl => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => PreVal::Byte(l.rotate_left(r.into())),
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.rotl(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.rotl(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Rotr => match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => {
                            PreVal::Byte(l.rotate_right(r.into()))
                        }
                        (PreVal::Uint(l), PreVal::Uint(r)) => PreVal::Uint(l.rotr(r)),
                        (PreVal::Sint(l), PreVal::Sint(r)) => PreVal::Sint(l.rotr(r)),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Eq => PreVal::Bool(match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => l == r,
                        (PreVal::Byte(l), PreVal::Byte(r)) => l == r,
                        (PreVal::Uint(l), PreVal::Uint(r)) => l == r,
                        (PreVal::Sint(l), PreVal::Sint(r)) => l == r,
                        (PreVal::Char(l), PreVal::Char(r)) => l == r,
                        (PreVal::Text(l), PreVal::Text(r)) => l == r,

                        _ => todo!("error: incompatible in preprocessing"),
                    }),
                    Operator::Ne => PreVal::Bool(match (lhs, rhs) {
                        (PreVal::Bool(l), PreVal::Bool(r)) => l != r,
                        (PreVal::Byte(l), PreVal::Byte(r)) => l != r,
                        (PreVal::Uint(l), PreVal::Uint(r)) => l != r,
                        (PreVal::Sint(l), PreVal::Sint(r)) => l != r,
                        (PreVal::Char(l), PreVal::Char(r)) => l != r,
                        (PreVal::Text(l), PreVal::Text(r)) => l != r,

                        _ => todo!("error: incompatible in preprocessing"),
                    }),
                    Operator::Lt => PreVal::Bool(match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => l < r,
                        (PreVal::Uint(l), PreVal::Uint(r)) => l < r,
                        (PreVal::Sint(l), PreVal::Sint(r)) => l < r,

                        _ => todo!("error: incompatible in preprocessing"),
                    }),
                    Operator::Gt => PreVal::Bool(match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => l > r,
                        (PreVal::Uint(l), PreVal::Uint(r)) => l > r,
                        (PreVal::Sint(l), PreVal::Sint(r)) => l > r,

                        _ => todo!("error: incompatible in preprocessing"),
                    }),
                    Operator::Le => PreVal::Bool(match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => l <= r,
                        (PreVal::Uint(l), PreVal::Uint(r)) => l <= r,
                        (PreVal::Sint(l), PreVal::Sint(r)) => l <= r,

                        _ => todo!("error: incompatible in preprocessing"),
                    }),
                    Operator::Ge => PreVal::Bool(match (lhs, rhs) {
                        (PreVal::Byte(l), PreVal::Byte(r)) => l >= r,
                        (PreVal::Uint(l), PreVal::Uint(r)) => l >= r,
                        (PreVal::Sint(l), PreVal::Sint(r)) => l >= r,

                        _ => todo!("error: incompatible in preprocessing"),
                    }),

                    Operator::MacroConcat => unimplemented!("should be handled by expansion"),
                    _ => todo!("error: unsupported in preprocessing"),
                })
            }

            PreExpr::Unary(inner) => {
                let operand = inner.operand.eval(source)?;
                Ok(match inner.op {
                    Operator::Not => match operand {
                        PreVal::Bool(x) => PreVal::Bool(!x),
                        PreVal::Byte(x) => PreVal::Byte(!x),
                        PreVal::Uint(x) => PreVal::Uint(!x),
                        PreVal::Sint(x) => PreVal::Sint(!x),

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Exists => PreVal::Bool(match operand {
                        PreVal::Bool(x) => x,
                        PreVal::Byte(x) => x != 0,
                        PreVal::Uint(x) => x != 0,
                        PreVal::Sint(x) => x != 0,
                        PreVal::Char(x) => x != '\0',
                        PreVal::Text(x) => !x.is_empty(),
                        PreVal::Macro(_x) => todo!("macro existence"),

                        _ => todo!("error: incompatible in preprocessing"),
                    }),
                    Operator::Neg => match operand {
                        PreVal::Sint(x) => PreVal::Sint(x.wrapping_neg()), // TBD: how should overflow be handled?

                        _ => todo!("error: incompatible in preprocessing"),
                    },
                    Operator::Szof => PreVal::Uint(match operand {
                        PreVal::Char(x) => x.len_utf8(),
                        PreVal::Text(x) => x.len(),
                        PreVal::Typename(PrimitiveType::None) => std::mem::size_of::<none>(),
                        PreVal::Typename(PrimitiveType::Nevr) => std::mem::size_of::<nevr>(),
                        PreVal::Typename(PrimitiveType::Bool) => std::mem::size_of::<bool>(),
                        PreVal::Typename(PrimitiveType::Byte) => std::mem::size_of::<byte>(),
                        PreVal::Typename(PrimitiveType::Uint) => std::mem::size_of::<uint>(),
                        PreVal::Typename(PrimitiveType::Sint) => std::mem::size_of::<sint>(),
                        PreVal::Typename(PrimitiveType::Flpt) => std::mem::size_of::<flpt>(),
                        PreVal::Typename(PrimitiveType::Char) => std::mem::size_of::<char>(),
                        PreVal::Typename(PrimitiveType::Text) => std::mem::size_of::<text>(),

                        _ => todo!("error: incompatible in preprocessing"),
                    }),

                    // TBD: should this even be handled by eval?
                    Operator::MacroStringify => PreVal::Text(match operand {
                        PreVal::Bool(x) => {
                            (if x { Keyword::True } else { Keyword::Fals }).to_string()
                        }
                        PreVal::Byte(x) => x.to_string(),
                        PreVal::Uint(x) => x.to_string(),
                        PreVal::Sint(x) => x.to_string(),
                        PreVal::Char(x) => x.to_string(),
                        PreVal::Text(x) => x,
                        PreVal::Typename(x) => x.to_string(),
                        PreVal::Macro(x) => x.to_owned(),
                    }),

                    Operator::MacroConst => unimplemented!("should be handled by expansion"),
                    _ => todo!("error: unsupported in preprocessing"),
                })
            }

            PreExpr::Group(PreGroup { expr, .. }) => expr.eval(source),

            PreExpr::Value(inner) => Ok(inner),
        }
    }
}

pub fn eval_predicate(
    tokens: ast::Replacer,
    source: &SourceMap,
    file: FileId,
) -> Result<bool, Diagnostic> {
    if let PreVal::Bool(x) = PreParser::new(tokens.items.list, source)
        .expression()?
        .eval(source)?
    {
        Ok(x)
    } else {
        let open = tokens.open.into_span(file);
        Err(Diagnostic::new(
            open.try_super_union(tokens.close.into_span(file))
                .unwrap_or(open),
            DiagnosticKind::NonBoolPrePredicate {
                actual: todo!("change NonBoolPrePredicate to use PreVal"),
            },
        ))
    }
}

struct PreParser<'src, I: Iterator> {
    source: &'src SourceMap,
    stream: std::iter::Peekable<I>,
}

impl<'src, I> PreParser<'src, I>
where
    I: Iterator<Item = (StreamSpan, TokenKind)>,
{
    fn new<J>(stream: J, source: &'src SourceMap) -> Self
    where
        J: IntoIterator<IntoIter = I>,
    {
        Self {
            source,
            stream: stream.into_iter().peekable(),
        }
    }

    fn pull_if<P>(&mut self, pred: P) -> Option<(StreamSpan, TokenKind)>
    where
        P: FnOnce(&I::Item) -> bool,
    {
        self.stream.next_if(pred)
    }

    fn pull_if_eq<T>(&mut self, kind: T) -> Option<StreamSpan>
    where
        T: Into<TokenKind>,
    {
        self.pull_if(|(_, k)| *k == kind.into())
            .map(|(span, _)| span)
    }

    fn pull_into_if<T, P>(&mut self, pred: P) -> Option<(StreamSpan, T)>
    where
        TokenKind: TryInto<T>,
        P: FnOnce(&T) -> bool,
    {
        self.stream.next_if_map(|(span, kind)| {
            if let Ok(x) = kind.try_into()
                && pred(&x)
            {
                Ok((span, x))
            } else {
                Err((span, kind))
            }
        })
    }
}

impl<'src, I> PreParser<'src, I>
where
    I: Iterator<Item = (StreamSpan, TokenKind)>,
{
    fn expression(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        self.or()
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        let mut expr = self.xor()?;
        while let Some((op_span, op)) =
            self.pull_into_if(|kind| matches!(kind, Operator::Or | Operator::Nor))
        {
            let rhs = self.xor()?;
            expr = PreExpr::Binary(Box::new(PreBinary {
                lhs: expr,
                op,
                op_span,
                rhs,
            }));
        }
        Ok(expr)
    }

    /// `xor -> and ( ("^" | "!^") and )*`
    fn xor(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `unary_prefix -> "szof" type_expression | ( "!" | "!!" | "-" )* exponent`
    fn unary_prefix(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `exponent -> primary ( "**" primary )*`
    fn exponent(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }

    /// `primary -> group | BOOL | BYTE | UINT | SINT | CHAR | TEXT | IDENT | MACRO`
    fn primary(&mut self) -> Result<PreExpr<'src>, Diagnostic> {
        todo!()
    }
}
