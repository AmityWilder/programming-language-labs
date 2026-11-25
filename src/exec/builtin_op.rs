use crate::{
    error::fail::Failure,
    exec::{Value, env::Environment, tree::Expr},
};

// Temporary dynamic type solution
#[cfg(true)]
mod inner {
    use super::*;
    use crate::exec::ControlFlow;
    use std::ops::FromResidual as _;

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum BuiltinOp {
        Assign,
        Inpt,
        Exists,
        /// TODO: how will other views be indexed?
        Index,
        Slice,
        Not,
        And,
        Or,
        Xor,
        Nand,
        Nor,
        Xnor,
        Eq,
        Ne,
        Lt,
        Gt,
        Le,
        Ge,
        Neg,
        Add,
        Sub,
        Mul,
        Div,
        Rem,
        Pow,
        Shl,
        Shr,
        Rotl,
        Rotr,
    }

    impl BuiltinOp {
        pub fn eval(self, env: &mut Environment, args: &[Expr]) -> ControlFlow<Failure, Value> {
            ControlFlow::Ok(match self {
                // mutate reference
                Self::Assign => {
                    let [a, b] = args else { unimplemented!() };
                    // TODO: change this to eval_var() when the static analyzer can determine types statically
                    let Some(var) = a.eval(env)?.var() else {
                        return ControlFlow::Err(Failure::ImmutableAssignment);
                    };
                    let value = b.eval(env)?;
                    if let Some(var) = env.get_mut(var) {
                        // existing
                        *var = value;
                    } else {
                        // initializer
                        env.insert(var, value);
                    }
                    Value::None
                }

                // short circuiting
                Self::And | Self::Or | Self::Nand | Self::Nor => {
                    let [a, b] = args else { unimplemented!() };
                    match match a.eval(env) {
                        ControlFlow::Ok(x) => x,
                        res => return res,
                    } {
                        Value::Bool(a) => {
                            let res = match self {
                                Self::And | Self::Nand => a && b.eval_bool(env)?,
                                Self::Or | Self::Nor => a || b.eval_bool(env)?,
                                _ => unreachable!(),
                            };
                            let is_neg = matches!(self, Self::Nand | Self::Nor);
                            Value::Bool(res ^ is_neg)
                        }

                        Value::Byte(a) => Value::Byte(match self {
                            Self::And | Self::Nand => a & b.eval_byte(env)?,
                            Self::Or | Self::Nor => a | b.eval_byte(env)?,
                            _ => unreachable!(),
                        }),
                        Value::Uint(a) => Value::Uint(match self {
                            Self::And | Self::Nand => a & b.eval_uint(env)?,
                            Self::Or | Self::Nor => a | b.eval_uint(env)?,
                            _ => unreachable!(),
                        }),
                        Value::Sint(a) => Value::Sint(match self {
                            Self::And | Self::Nand => a & b.eval_sint(env)?,
                            Self::Or | Self::Nor => a | b.eval_sint(env)?,
                            _ => unreachable!(),
                        }),

                        _ => todo!("error: incompatible"),
                    }
                }

                _ => {
                    // TODO: collecting is expensive, replace with fixed size
                    let args = match args
                        .iter()
                        .map(|x| {
                            x.eval(env).ok().map(|val| {
                                if matches!(val, Value::Var(_)) {
                                    let mut val = &val;
                                    while let Value::Var(id) = *val {
                                        // TODO: what if there's a cycle?
                                        val = env.get(id).expect("invalid reference");
                                    }
                                    val.clone()
                                } else {
                                    val
                                }
                            })
                        })
                        .collect::<Result<Vec<Value>, ControlFlow<Failure, !, Value>>>()
                    {
                        Ok(list) => list,
                        Err(res) => return ControlFlow::from_residual(res),
                    };
                    match self {
                        Self::Assign | Self::And | Self::Or | Self::Nand | Self::Nor => {
                            unreachable!()
                        }

                        Self::Inpt => {
                            let mut buf = String::new();
                            if std::io::stdin().read_line(&mut buf).is_err() {
                                return ControlFlow::Err(Failure::FailedRead);
                            }
                            crate::pop_trailing_newline(&mut buf);
                            Value::Text(buf)
                        }

                        Self::Index => todo!("exec Index"),
                        Self::Slice => todo!("exec Slice"),

                        Self::Xor => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Bool(a), Value::Bool(b)) => Value::Bool(a ^ b),
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a ^ b),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(a ^ b),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(a ^ b),

                                (a, b) => todo!("error: incompatible: {a:?} ^ {b:?}"),
                            }
                        }
                        Self::Xnor => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Bool(a), Value::Bool(b)) => Value::Bool(!(a ^ b)),
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(!(a ^ b)),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(!(a ^ b)),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(!(a ^ b)),

                                (a, b) => todo!("error: incompatible: {a:?} !^ {b:?}"),
                            }
                        }
                        Self::Not => {
                            let [x] = <[_; 1]>::try_from(args).expect("bad argcount");
                            match x {
                                Value::Bool(x) => Value::Bool(!x),
                                Value::Byte(x) => Value::Byte(!x),
                                Value::Uint(x) => Value::Uint(!x),
                                Value::Sint(x) => Value::Sint(!x),

                                _ => todo!("error: unsupported"),
                            }
                        }
                        Self::Exists => {
                            let [x] = <[_; 1]>::try_from(args).expect("bad argcount");
                            Value::Bool(match x {
                                Value::None | Value::Fail(_) => false,

                                Value::Bool(x) => x,
                                Value::Byte(x) => x != 0,
                                Value::Uint(x) => x != 0,
                                Value::Sint(x) => x != 0,
                                Value::Flpt(x) => !x.is_nan(),
                                Value::Char(x) => x != '\0',
                                Value::Text(x) => !x.is_empty(),

                                Value::Var(_) => todo!(),
                                Value::View { .. } => todo!(),
                            })
                        }
                        Self::Add => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a + b),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(a + b),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(a + b),
                                (Value::Flpt(a), Value::Flpt(b)) => Value::Flpt(a + b),

                                (Value::Text(a), Value::None) => Value::Text(format!("{a}none")),
                                (Value::Text(a), Value::Bool(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Text(a), Value::Byte(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Text(a), Value::Uint(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Text(a), Value::Sint(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Text(a), Value::Flpt(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Text(a), Value::Char(b)) => Value::Text(format!("{a}{b}")),

                                (Value::None, Value::Text(b)) => Value::Text(format!("none{b}")),
                                (Value::Bool(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Byte(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Uint(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Sint(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Flpt(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),
                                (Value::Char(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),

                                (Value::Text(a), Value::Text(b)) => Value::Text(format!("{a}{b}")),

                                (a, b) => todo!("error: incompatible: {a:?} + {b:?}"),
                            }
                        }
                        Self::Sub => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a - b),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(a - b),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(a - b),
                                (Value::Flpt(a), Value::Flpt(b)) => Value::Flpt(a - b),

                                (a, b) => todo!("error: incompatible: {a:?} - {b:?}"),
                            }
                        }
                        Self::Neg => {
                            let [x] = <[_; 1]>::try_from(args).expect("bad argcount");
                            match x {
                                Value::Sint(x) => Value::Sint(-x),
                                Value::Flpt(x) => Value::Flpt(-x),

                                x => todo!("error: unsupported: -{x:?}"),
                            }
                        }
                        Self::Mul => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a * b),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(a * b),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(a * b),
                                (Value::Flpt(a), Value::Flpt(b)) => Value::Flpt(a * b),

                                (a, b) => todo!("error: incompatible: {a:?} * {b:?}"),
                            }
                        }
                        Self::Pow => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a.pow(b.into())),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(
                                    b.try_into()
                                        .map_or_else(|_| todo!("overflow"), |b| a.pow(b)),
                                ),
                                (Value::Sint(a), Value::Uint(b)) => Value::Sint(
                                    b.try_into()
                                        .map_or_else(|_| todo!("overflow"), |b| a.pow(b)),
                                ),
                                (Value::Flpt(a), Value::Flpt(b)) => Value::Flpt(a.powf(b)),

                                (a, b) => todo!("error: incompatible: {a:?} ** {b:?}"),
                            }
                        }
                        Self::Div => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a / b),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(a / b),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(a / b),
                                (Value::Flpt(a), Value::Flpt(b)) => Value::Flpt(a / b),

                                (a, b) => todo!("error: incompatible: {a:?} / {b:?}"),
                            }
                        }
                        Self::Rem => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => Value::Byte(a % b),
                                (Value::Uint(a), Value::Uint(b)) => Value::Uint(a % b),
                                (Value::Sint(a), Value::Sint(b)) => Value::Sint(a % b),
                                (Value::Flpt(a), Value::Flpt(b)) => Value::Flpt(a % b),

                                (a, b) => todo!("error: incompatible: {a:?} % {b:?}"),
                            }
                        }
                        Self::Shl => todo!("exec Shl"),
                        Self::Shr => todo!("exec Shr"),
                        Self::Rotl => todo!("exec Rotl"),
                        Self::Rotr => todo!("exec Rotr"),

                        Self::Eq => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            Value::Bool(match (a, b) {
                                (Value::None, Value::None) => todo!("none equality"),
                                (Value::Fail(_), Value::Fail(_)) => todo!("failure equality"),
                                (Value::Bool(a), Value::Bool(b)) => a == b,
                                (Value::Byte(a), Value::Byte(b)) => a == b,
                                (Value::Uint(a), Value::Uint(b)) => a == b,
                                (Value::Sint(a), Value::Sint(b)) => a == b,
                                (Value::Flpt(a), Value::Flpt(b)) => a == b,
                                (Value::Char(a), Value::Char(b)) => a == b,
                                (Value::Text(a), Value::Text(b)) => a == b,

                                (a, b) => todo!("error: incompatible: {a:?} == {b:?}"),
                            })
                        }
                        Self::Ne => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            Value::Bool(match (a, b) {
                                (Value::None, Value::None) => todo!("none equality"),
                                (Value::Fail(_), Value::Fail(_)) => todo!("failure equality"),
                                (Value::Bool(a), Value::Bool(b)) => a != b,
                                (Value::Byte(a), Value::Byte(b)) => a != b,
                                (Value::Uint(a), Value::Uint(b)) => a != b,
                                (Value::Sint(a), Value::Sint(b)) => a != b,
                                (Value::Flpt(a), Value::Flpt(b)) => a != b,
                                (Value::Char(a), Value::Char(b)) => a != b,
                                (Value::Text(a), Value::Text(b)) => a != b,

                                (a, b) => todo!("error: incompatible: {a:?} != {b:?}"),
                            })
                        }

                        Self::Lt => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            Value::Bool(match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => a < b,
                                (Value::Uint(a), Value::Uint(b)) => a < b,
                                (Value::Sint(a), Value::Sint(b)) => a < b,
                                (Value::Flpt(a), Value::Flpt(b)) => a < b,
                                (Value::Char(a), Value::Char(b)) => a < b,
                                (Value::Text(a), Value::Text(b)) => a < b,

                                (a, b) => todo!("error: incompatible: {a:?} < {b:?}"),
                            })
                        }
                        Self::Gt => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            Value::Bool(match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => a > b,
                                (Value::Uint(a), Value::Uint(b)) => a > b,
                                (Value::Sint(a), Value::Sint(b)) => a > b,
                                (Value::Flpt(a), Value::Flpt(b)) => a > b,
                                (Value::Char(a), Value::Char(b)) => a > b,
                                (Value::Text(a), Value::Text(b)) => a > b,

                                (a, b) => todo!("error: incompatible: {a:?} > {b:?}"),
                            })
                        }
                        Self::Le => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            Value::Bool(match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => a <= b,
                                (Value::Uint(a), Value::Uint(b)) => a <= b,
                                (Value::Sint(a), Value::Sint(b)) => a <= b,
                                (Value::Flpt(a), Value::Flpt(b)) => a <= b,
                                (Value::Char(a), Value::Char(b)) => a <= b,
                                (Value::Text(a), Value::Text(b)) => a <= b,

                                (a, b) => todo!("error: incompatible: {a:?} <= {b:?}"),
                            })
                        }
                        Self::Ge => {
                            let [a, b] = <[_; 2]>::try_from(args).expect("bad argcount");
                            Value::Bool(match (a, b) {
                                (Value::Byte(a), Value::Byte(b)) => a >= b,
                                (Value::Uint(a), Value::Uint(b)) => a >= b,
                                (Value::Sint(a), Value::Sint(b)) => a >= b,
                                (Value::Flpt(a), Value::Flpt(b)) => a >= b,
                                (Value::Char(a), Value::Char(b)) => a >= b,
                                (Value::Text(a), Value::Text(b)) => a >= b,

                                (a, b) => todo!("error: incompatible: {a:?} >= {b:?}"),
                            })
                        }
                    }
                }
            })
        }
    }
}

#[cfg(false)]
mod inner {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum BuiltinOp {
        InptB,
        InptC,
        InptT,

        ExistsN,
        ExistsF,
        ExistsL,
        ExistsB,
        ExistsU,
        ExistsS,
        ExistsC,
        ExistsT,

        // TODO: how will other views be indexed?
        IndexT,
        SliceT,

        NotL,
        NotB,
        NotU,
        NotS,

        AndL,
        AndB,
        AndU,
        AndS,

        OrL,
        OrB,
        OrU,
        OrS,

        XorL,
        XorB,
        XorU,
        XorS,

        NandL,
        NandB,
        NandU,
        NandS,

        NorL,
        NorB,
        NorU,
        NorS,

        XnorL,
        XnorB,
        XnorU,
        XnorS,

        EqL,
        EqB,
        EqU,
        EqS,
        EqC,
        EqT,

        NeL,
        NeB,
        NeU,
        NeS,
        NeC,
        NeT,

        LtL,
        LtB,
        LtU,
        LtS,
        LtC,
        LtT,

        GtL,
        GtB,
        GtU,
        GtS,
        GtC,
        GtT,

        LeL,
        LeB,
        LeU,
        LeS,
        LeC,
        LeT,

        GeL,
        GeB,
        GeU,
        GeS,
        GeC,
        GeT,

        NegS,
        NegF,

        AddB,
        AddU,
        AddS,
        AddF,

        SubB,
        SubU,
        SubS,
        SubF,

        MulB,
        MulU,
        MulS,
        MulF,

        DivB,
        DivU,
        DivS,

        RemB,
        RemU,
        RemS,

        PowBB,
        PowBU,
        PowUB,
        PowUU,
        PowSB,
        PowSU,
        PowFB,
        PowFU,
        PowFF,

        ShlBB,
        ShlBU,
        ShlBS,
        ShlUB,
        ShlUU,
        ShlUS,
        ShlSB,
        ShlSU,
        ShlSS,

        ShrBB,
        ShrBU,
        ShrBS,
        ShrUB,
        ShrUU,
        ShrUS,
        ShrSB,
        ShrSU,
        ShrSS,

        RotlBB,
        RotlBU,
        RotlBS,
        RotlUB,
        RotlUU,
        RotlUS,
        RotlSB,
        RotlSU,
        RotlSS,

        RotrBB,
        RotrBU,
        RotrBS,
        RotrUB,
        RotrUU,
        RotrUS,
        RotrSB,
        RotrSU,
        RotrSS,
    }

    impl BuiltinOp {
        pub fn eval(self, env: &mut Environment, args: &[Expr]) -> Result<Value, Failure> {
            use BuiltinOp::*;
            Ok(match (self, args) {
                (InptB, []) => todo!("byte input"),
                (InptC, []) => todo!("char input"),
                (InptT, []) => todo!("text input"),

                (ExistsN | ExistsF, [_]) => Value::Bool(false),
                (ExistsL, [x]) => Value::Bool(x.eval_bool(env)?),
                (ExistsB, [x]) => Value::Bool(x.eval_byte(env)? != 0),
                (ExistsU, [x]) => Value::Bool(x.eval_uint(env)? != 0),
                (ExistsS, [x]) => Value::Bool(x.eval_sint(env)? != 0),
                (ExistsC, [x]) => Value::Bool(x.eval_char(env)? != '\0'),
                (ExistsT, [x]) => Value::Bool(!x.eval_text(env)?.is_empty()),

                // TBD: is x a character index or a byte index?
                (IndexT, [x]) => todo!(),
                // TBD: range?
                (SliceT, [x]) => todo!(),

                (NotL, [x]) => Value::Bool(!x.eval_bool(env)?),
                (NotB, [x]) => Value::Byte(!x.eval_byte(env)?),
                (NotU, [x]) => Value::Uint(!x.eval_uint(env)?),
                (NotS, [x]) => Value::Sint(!x.eval_sint(env)?),

                // short-circuiting
                (AndL | OrL | XorL | NandL | NorL | XnorL, [a, b]) => {
                    let res = match self {
                        AndL | NandL => a.eval_bool(env)? && b.eval_bool(env)?,
                        OrL | NorL => a.eval_bool(env)? || b.eval_bool(env)?,
                        XorL | XnorL => a.eval_bool(env)? != b.eval_bool(env)?,

                        _ => unreachable!(),
                    };
                    let is_neg = matches!(self, NandL | NorL | XnorL);
                    Value::Bool(res ^ is_neg)
                }

                (AndB, [a, b]) => Value::Byte(a.eval_byte(env)? & b.eval_byte(env)?),
                (AndU, [a, b]) => Value::Uint(a.eval_uint(env)? & b.eval_uint(env)?),
                (AndS, [a, b]) => Value::Sint(a.eval_sint(env)? & b.eval_sint(env)?),

                (OrB, [a, b]) => Value::Byte(a.eval_byte(env)? | b.eval_byte(env)?),
                (OrU, [a, b]) => Value::Uint(a.eval_uint(env)? | b.eval_uint(env)?),
                (OrS, [a, b]) => Value::Sint(a.eval_sint(env)? | b.eval_sint(env)?),

                (XorB, [a, b]) => Value::Byte(a.eval_byte(env)? ^ b.eval_byte(env)?),
                (XorU, [a, b]) => Value::Uint(a.eval_uint(env)? ^ b.eval_uint(env)?),
                (XorS, [a, b]) => Value::Sint(a.eval_sint(env)? ^ b.eval_sint(env)?),

                (NandB, [a, b]) => Value::Byte(!(a.eval_byte(env)? & b.eval_byte(env)?)),
                (NandU, [a, b]) => Value::Uint(!(a.eval_uint(env)? & b.eval_uint(env)?)),
                (NandS, [a, b]) => Value::Sint(!(a.eval_sint(env)? & b.eval_sint(env)?)),

                (NorB, [a, b]) => Value::Byte(!(a.eval_byte(env)? | b.eval_byte(env)?)),
                (NorU, [a, b]) => Value::Uint(!(a.eval_uint(env)? | b.eval_uint(env)?)),
                (NorS, [a, b]) => Value::Sint(!(a.eval_sint(env)? | b.eval_sint(env)?)),

                (XnorB, [a, b]) => Value::Byte(!(a.eval_byte(env)? ^ b.eval_byte(env)?)),
                (XnorU, [a, b]) => Value::Uint(!(a.eval_uint(env)? ^ b.eval_uint(env)?)),
                (XnorS, [a, b]) => Value::Sint(!(a.eval_sint(env)? ^ b.eval_sint(env)?)),

                (EqL, [a, b]) => Value::Bool(a.eval_bool(env)? == b.eval_bool(env)?),
                (EqB, [a, b]) => Value::Bool(a.eval_byte(env)? == b.eval_byte(env)?),
                (EqU, [a, b]) => Value::Bool(a.eval_uint(env)? == b.eval_uint(env)?),
                (EqS, [a, b]) => Value::Bool(a.eval_sint(env)? == b.eval_sint(env)?),
                (EqC, [a, b]) => Value::Bool(a.eval_char(env)? == b.eval_char(env)?),
                (EqT, [a, b]) => Value::Bool(a.eval_text(env)? == b.eval_text(env)?),

                (NeL, [a, b]) => Value::Bool(a.eval_bool(env)? != b.eval_bool(env)?),
                (NeB, [a, b]) => Value::Bool(a.eval_byte(env)? != b.eval_byte(env)?),
                (NeU, [a, b]) => Value::Bool(a.eval_uint(env)? != b.eval_uint(env)?),
                (NeS, [a, b]) => Value::Bool(a.eval_sint(env)? != b.eval_sint(env)?),
                (NeC, [a, b]) => Value::Bool(a.eval_char(env)? != b.eval_char(env)?),
                (NeT, [a, b]) => Value::Bool(a.eval_text(env)? != b.eval_text(env)?),

                (LtB, [a, b]) => Value::Bool(a.eval_byte(env)? < b.eval_byte(env)?),
                (LtU, [a, b]) => Value::Bool(a.eval_uint(env)? < b.eval_uint(env)?),
                (LtS, [a, b]) => Value::Bool(a.eval_sint(env)? < b.eval_sint(env)?),
                (LtC, [a, b]) => Value::Bool(a.eval_char(env)? < b.eval_char(env)?),
                (LtT, [a, b]) => Value::Bool(a.eval_text(env)? < b.eval_text(env)?),

                (GtB, [a, b]) => Value::Bool(a.eval_byte(env)? > b.eval_byte(env)?),
                (GtU, [a, b]) => Value::Bool(a.eval_uint(env)? > b.eval_uint(env)?),
                (GtS, [a, b]) => Value::Bool(a.eval_sint(env)? > b.eval_sint(env)?),
                (GtC, [a, b]) => Value::Bool(a.eval_char(env)? > b.eval_char(env)?),
                (GtT, [a, b]) => Value::Bool(a.eval_text(env)? > b.eval_text(env)?),

                (LeB, [a, b]) => Value::Bool(a.eval_byte(env)? <= b.eval_byte(env)?),
                (LeU, [a, b]) => Value::Bool(a.eval_uint(env)? <= b.eval_uint(env)?),
                (LeS, [a, b]) => Value::Bool(a.eval_sint(env)? <= b.eval_sint(env)?),
                (LeC, [a, b]) => Value::Bool(a.eval_char(env)? <= b.eval_char(env)?),
                (LeT, [a, b]) => Value::Bool(a.eval_text(env)? <= b.eval_text(env)?),

                (GeB, [a, b]) => Value::Bool(a.eval_byte(env)? >= b.eval_byte(env)?),
                (GeU, [a, b]) => Value::Bool(a.eval_uint(env)? >= b.eval_uint(env)?),
                (GeS, [a, b]) => Value::Bool(a.eval_sint(env)? >= b.eval_sint(env)?),
                (GeC, [a, b]) => Value::Bool(a.eval_char(env)? >= b.eval_char(env)?),
                (GeT, [a, b]) => Value::Bool(a.eval_text(env)? >= b.eval_text(env)?),

                (NegS, [x]) => Value::Sint(-x.eval_sint(env)?),
                (NegF, [x]) => Value::Flpt(-x.eval_flpt(env)?),

                (AddB, [a, b]) => Value::Byte(a.eval_byte(env)? + b.eval_byte(env)?),
                (AddU, [a, b]) => Value::Uint(a.eval_uint(env)? + b.eval_uint(env)?),
                (AddS, [a, b]) => Value::Sint(a.eval_sint(env)? + b.eval_sint(env)?),
                (AddF, [a, b]) => Value::Flpt(a.eval_flpt(env)? + b.eval_flpt(env)?),

                (SubB, [a, b]) => Value::Byte(a.eval_byte(env)? - b.eval_byte(env)?),
                (SubU, [a, b]) => Value::Uint(a.eval_uint(env)? - b.eval_uint(env)?),
                (SubS, [a, b]) => Value::Sint(a.eval_sint(env)? - b.eval_sint(env)?),
                (SubF, [a, b]) => Value::Flpt(a.eval_flpt(env)? - b.eval_flpt(env)?),

                (MulB, [a, b]) => Value::Byte(a.eval_byte(env)? * b.eval_byte(env)?),
                (MulU, [a, b]) => Value::Uint(a.eval_uint(env)? * b.eval_uint(env)?),
                (MulS, [a, b]) => Value::Sint(a.eval_sint(env)? * b.eval_sint(env)?),
                (MulF, [a, b]) => Value::Flpt(a.eval_flpt(env)? * b.eval_flpt(env)?),

                (DivB, [a, b]) => Value::Byte(a.eval_byte(env)? / b.eval_byte(env)?),
                (DivU, [a, b]) => Value::Uint(a.eval_uint(env)? / b.eval_uint(env)?),
                (DivS, [a, b]) => Value::Sint(a.eval_sint(env)? / b.eval_sint(env)?),

                (RemB, [a, b]) => Value::Byte(a.eval_byte(env)? % b.eval_byte(env)?),
                (RemU, [a, b]) => Value::Uint(a.eval_uint(env)? % b.eval_uint(env)?),
                (RemS, [a, b]) => Value::Sint(a.eval_sint(env)? % b.eval_sint(env)?),

                (PowBB, [a, b]) => todo!(),
                (PowBU, [a, b]) => todo!(),
                (PowUB, [a, b]) => todo!(),
                (PowUU, [a, b]) => todo!(),
                (PowSB, [a, b]) => todo!(),
                (PowSU, [a, b]) => todo!(),
                (PowFB, [a, b]) => todo!(),
                (PowFU, [a, b]) => todo!(),
                (PowFF, [a, b]) => todo!(),

                (ShlBB, [a, b]) => todo!(),
                (ShlBU, [a, b]) => todo!(),
                (ShlBS, [a, b]) => todo!(),
                (ShlUB, [a, b]) => todo!(),
                (ShlUU, [a, b]) => todo!(),
                (ShlUS, [a, b]) => todo!(),
                (ShlSB, [a, b]) => todo!(),
                (ShlSU, [a, b]) => todo!(),
                (ShlSS, [a, b]) => todo!(),

                (ShrBB, [a, b]) => todo!(),
                (ShrBU, [a, b]) => todo!(),
                (ShrBS, [a, b]) => todo!(),
                (ShrUB, [a, b]) => todo!(),
                (ShrUU, [a, b]) => todo!(),
                (ShrUS, [a, b]) => todo!(),
                (ShrSB, [a, b]) => todo!(),
                (ShrSU, [a, b]) => todo!(),
                (ShrSS, [a, b]) => todo!(),

                (RotlBB, [a, b]) => todo!(),
                (RotlBU, [a, b]) => todo!(),
                (RotlBS, [a, b]) => todo!(),
                (RotlUB, [a, b]) => todo!(),
                (RotlUU, [a, b]) => todo!(),
                (RotlUS, [a, b]) => todo!(),
                (RotlSB, [a, b]) => todo!(),
                (RotlSU, [a, b]) => todo!(),
                (RotlSS, [a, b]) => todo!(),

                (RotrBB, [a, b]) => todo!(),
                (RotrBU, [a, b]) => todo!(),
                (RotrBS, [a, b]) => todo!(),
                (RotrUB, [a, b]) => todo!(),
                (RotrUU, [a, b]) => todo!(),
                (RotrUS, [a, b]) => todo!(),
                (RotrSB, [a, b]) => todo!(),
                (RotrSU, [a, b]) => todo!(),
                (RotrSS, [a, b]) => todo!(),

                // TODO: this will also match any new operators added. thats bad.
                _ => panic!("invalid operands; {STATIC_GUARANTEE}"),
            })
        }
    }
}
pub use inner::*;
