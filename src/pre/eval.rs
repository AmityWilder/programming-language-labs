use crate::{
    analyzer::types::PrimitiveType,
    error::{Error, ErrorKind},
    lex::token::{Operator, TokenKind, punc::OpSide},
    pre::{StreamSpan, ast},
    src_code::{map::SourceMap, span::FileId},
    types::{byte, sint, text, uint},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreVal<'src> {
    Bool(bool),
    Byte(byte),
    Uint(uint),
    Sint(sint),
    Char(char),
    Text(text),

    Typename(PrimitiveType),
    Macro(&'src str),
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
    Value(PreVal<'src>),
}

impl<'src> PreExpr<'src> {
    fn eval(self, source: &'src SourceMap) -> Result<PreVal<'src>, Error> {
        match self {
            PreExpr::Binary(inner) => {
                let lhs = inner.lhs.eval(source)?;
                let rhs = inner.rhs.eval(source)?;
                match inner.op {
                    _ => todo!("error: incompatible in preprocessing"),
                }
            }

            PreExpr::Unary(inner) => {
                let operand = inner.operand.eval(source)?;
                match inner {
                    _ => todo!("error: unsupported in preprocessing"),
                }
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
) -> Result<bool, Error> {
    if let PreVal::Bool(x) = PreParser::new(tokens.items.list, source)
        .expression()?
        .eval(source)?
    {
        Ok(x)
    } else {
        let open = tokens.open.into_span(file);
        Err(Error::new(
            open.try_super_union(tokens.close.into_span(file))
                .unwrap_or(open),
            ErrorKind::NonBoolPrePredicate {
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
    fn expression(&mut self) -> Result<PreExpr<'src>, Error> {
        self.or()
    }

    /// `or -> xor ( ("|" | "!|") xor )*`
    fn or(&mut self) -> Result<PreExpr<'src>, Error> {
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
    fn xor(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `and -> equality ( ("&" | "!&") equality )*`
    fn and(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `equality -> comparison ( ("!=" | "==") comparison )*`
    fn equality(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `comparison -> shift ( ( ">" | ">=" | "<" | "<=" ) shift )*`
    fn comparison(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `shift -> term ( ( "<<" | ">>" | "[<<]" | "[>>]" ) term )*`
    fn shift(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `term -> factor ( ( "+" | "-" ) factor )*`
    fn term(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `factor -> unary_prefix ( ( "*" | "/" | "%" ) unary_prefix )*`
    fn factor(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `unary_prefix -> "szof" type_expression | ( "!" | "!!" | "-" )* exponent`
    fn unary_prefix(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `exponent -> primary ( "**" primary )*`
    fn exponent(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }

    /// `primary -> group | BOOL | BYTE | UINT | SINT | CHAR | TEXT | IDENT | MACRO`
    fn primary(&mut self) -> Result<PreExpr<'src>, Error> {
        todo!()
    }
}
