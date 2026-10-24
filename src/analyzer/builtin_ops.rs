#![allow(unused, reason = "under construction")]
#![expect(
    clippy::unnecessary_wraps,
    reason = "needs to fit the function pointer shape"
)]

use super::{
    AbstractType, BlockStmt, Failure, FnDef, FunType, OpOverload, OperandTypes, Operator,
    PrimitiveType, StaticType, types::FunShape,
};
use crate::{
    interp::value::{InterpVal, InterpValue, PrimitiveVal},
    spec_ops::{BitRotate, BitShift},
    types::{byte, fail, flpt, nevr, none, sint, text, uint},
};

pub fn interp_exists_def<'brw, 'src>(
    lhs: InterpValue<'brw, 'src>,
) -> std::result::Result<InterpValue<'brw, 'src>, Failure> {
    Ok(InterpValue::Prvalue(InterpVal::Primitive(
        crate::interp::value::PrimitiveVal::Bool(crate::interp::value::BoolObject(
            lhs.into_ref().ensure_ok().is_ok(),
        )),
    )))
}

macro_rules! prim_op {
    (
        $op:ident($lhs:ident, $rhs:ident) -> $output:ident

        as fn($lhs_name:ident: $Lhs:ty, $rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $($def:tt)*
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::Binary(AbstractType::$lhs, AbstractType::$rhs),
                output: AbstractType::$output,
                fun: FunShape {
                    params: vec![
                        StaticType::$lhs,
                        StaticType::$rhs,
                    ],
                    ret: StaticType::$output,
                },
                def: {
                    fn interp<'brw, 'src>(
                        lhs: InterpValue<'brw, 'src>,
                        rhs: InterpValue<'brw, 'src>,
                    ) -> Result<InterpValue<'brw, 'src>, Failure> {
                        let lhs = lhs.into_ref();
                        let rhs = rhs.into_ref();
                        let $lhs_name: &$Lhs = lhs.as_ref().ensure_ok()?.try_into().unwrap_or_else(|_| {
                            panic!(
                                "{}<{}, {}> lhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {lhs:?}",
                                stringify!($op),
                                stringify!($Lhs),
                                stringify!($Rhs),
                                stringify!($Lhs),
                            )
                        });
                        let $rhs_name: &$Rhs = rhs.as_ref().ensure_ok()?.try_into().unwrap_or_else(|_| {
                            panic!(
                                "{}<{}, {}> rhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {rhs:?}",
                                stringify!($op),
                                stringify!($Lhs),
                                stringify!($Rhs),
                                stringify!($Rhs),
                            )
                        });
                        let res: $Out = { $($def)* };
                        Ok(InterpValue::Prvalue(res.into()))
                    }
                    FnDef::LangOpB(interp)
                },
            },
        )
    };

    (
        $op:ident($rhs:ident) -> $output:ident

        as fn($rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $($def:tt)*
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::UnaryPre(AbstractType::$rhs),
                output: AbstractType::$output,
                fun: FunShape {
                    params: vec![StaticType::$rhs],
                    ret: StaticType::$output,
                },
                def: {
                    fn interp<'brw, 'src>(rhs: InterpValue<'brw, 'src>) -> Result<InterpValue<'brw, 'src>, Failure> {
                        let rhs = rhs.into_ref();
                        let $rhs_name: &$Rhs = rhs.as_ref().ensure_ok()?.try_into().unwrap_or_else(|_| {
                            panic!(
                                "{}<{}> rhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {rhs:?}",
                                stringify!($op),
                                stringify!($Rhs),
                                stringify!($Rhs),
                            )
                        });
                        let res: $Out = { $($def)* };
                        Ok(InterpValue::Prvalue(res.into()))
                    }
                    FnDef::LangOpU(interp)
                },
            },
        )
    };

    (
        ($lhs:ident)$op:ident -> $output:ident

        as fn($lhs_name:ident: $Lhs:ty) -> $Out:ty {
            $($def:tt)*
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::UnaryPost(AbstractType::$lhs),
                output: AbstractType::$output,
                fun: FunShape {
                    params: vec![StaticType::$lhs],
                    ret: StaticType::$output,
                },
                def: {
                    fn interp<'brw, 'src>(
                        lhs: InterpValue<'brw, 'src>,
                    ) -> Result<InterpValue<'brw, 'src>, Failure> {
                        let lhs = lhs.to_ref();
                        let $lhs_name: &$Lhs = lhs.as_ref().ensure_ok()?.try_into().unwrap_or_else(|_| {
                            panic!(
                                "{}<{}> lhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {lhs:?}",
                                stringify!($op),
                                stringify!($Lhs),
                                stringify!($Lhs),
                            )
                        });
                        let res: $Out = { $($def)* };
                        Ok(res.into())
                    }
                    FnDef::LangOpU(interp)
                },
            },
        )
    };
}

macro_rules! primitive_ops {
    ($(
        $($pre_bin_op:ident)?($lhs_unary:ident$(, $rhs:ident)?)$($post_op:ident)? -> $output:ident

        as fn($lhs_unary_name:ident: $Lhs:ty$(, $rhs_name:ident: $Rhs:ty)?) -> $Out:ty {
            $($def:tt)*
        }
    )*) => {
        vec![
            $(
                prim_op!(
                    $($pre_bin_op)?($lhs_unary$(, $rhs)?)$($post_op)? -> $output

                    as fn($lhs_unary_name: $Lhs$(, $rhs_name: $Rhs)?) -> $Out {
                        $($def)*
                    }
                )
            ),*
        ]
    };
}

macro_rules! incr_decr_op {
    (
        ($rhs:ident)$op:ident -> $output:ident

        as fn($rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $($def:tt)*
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::UnaryPost(AbstractType::$rhs),
                output: AbstractType::$output,
                fun: FunShape {
                    params: vec![StaticType::$rhs],
                    ret: StaticType::$output,
                },
                def: {
                    fn interp<'brw, 'src>(rhs: InterpValue<'brw, 'src>) -> Result<InterpValue<'brw, 'src>, Failure> {
                        let mut rhs = rhs.into_mut().unwrap_or_else(|| {
                            panic!(
                                "{}<{}> requires an lvalue operand, should be guaranteed by static analysis",
                                stringify!($op),
                                stringify!($Rhs),
                            );
                        });
                        let InterpVal::Primitive(PrimitiveVal::$rhs(rhs)) = &mut *rhs else {
                            panic!(
                                "{}<{}> rhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {rhs:?}",
                                stringify!($op),
                                stringify!($Rhs),
                                stringify!($Rhs),
                            )
                        };
                        let $rhs_name: &mut $Rhs = &mut rhs.0;
                        let res: $Out = { $($def)* };
                        Ok(InterpValue::Prvalue(res.into()))
                    }
                    FnDef::LangOpU(interp)
                },
            },
        )
    };
}

pub fn builtin_ops<'src>()
-> impl IntoIterator<IntoIter: DoubleEndedIterator, Item = (Operator, OpOverload<'src>)> {
    primitive_ops![
        // bool
        Exists(Bool) -> Bool as fn(r: bool) -> bool { *r } // TBD: should !!bool be equivalent to !(!bool)? or does it definitively represent "success"?
        Exists(Uint) -> Bool as fn(r: uint) -> bool { true }
        Exists(Sint) -> Bool as fn(r: sint) -> bool { true }
        Exists(Flpt) -> Bool as fn(r: flpt) -> bool { !r.is_nan() }
        Exists(Char) -> Bool as fn(r: char) -> bool { *r != '\0' }
        Exists(Text) -> Bool as fn(r: text) -> bool { !r.is_empty() }

        And (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l & r }
        Or  (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l | r }
        Xor (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l ^ r }
        Nand(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !(l & r) }
        Nor (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !(l | r) }
        Xnor(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !(l ^ r) }

        // 0 + 0 = 0
        // 0 + 1 = 1
        // 1 + 0 = 1
        // 1 + 1 = 1 (clamped)
        Add(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l | r }
        // 0 - 0 = 0
        // 0 - 1 = 0 (clamped)
        // 1 - 0 = 1
        // 1 - 1 = 0
        Sub(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !r & l }
        // 0 * 0 = 0
        // 0 * 1 = 0
        // 1 * 0 = 0
        // 1 * 1 = 1
        Mul(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l & r }
        // 0^0 = 1
        // 0^1 = 0
        // 1^0 = 1
        // 1^1 = 1
        Pow(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !r | l }
        // 0 / 0 = 1 (clamped infinity)
        // 0 / 1 = 0
        // 1 / 0 = 1 (clamped infinity)
        // 1 / 1 = 1
        Div(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !r | l }
        // 0 % 0 = 1 (clamped infinity)
        // 0 % 1 = 0
        // 1 % 0 = 1 (clamped infinity)
        // 1 % 1 = 1
        Rem(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !r | l }

        Eq(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l == r }
        Eq(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l == r }
        Eq(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l == r }
        Eq(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l == r } // TBD: should float equality be allowed?
        Eq(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l == r }
        Eq(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l == r }

        Ne(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l != r }
        Ne(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l != r }
        Ne(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l != r }
        Ne(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l != r } // TBD: should float equality be allowed?
        Ne(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l != r }
        Ne(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l != r }

        Gt(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l > r }
        Gt(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l > r }
        Gt(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l > r }
        Gt(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l > r }
        Gt(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l > r }
        Gt(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l > r }

        Lt(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l < r }
        Lt(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l < r }
        Lt(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l < r }
        Lt(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l < r }
        Lt(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l < r }
        Lt(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l < r }

        Ge(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l >= r }
        Ge(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l >= r }
        Ge(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l >= r }
        Ge(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l >= r }
        Ge(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l >= r }
        Ge(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l >= r }

        Le(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l <= r }
        Le(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l <= r }
        Le(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l <= r }
        Le(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l <= r }
        Le(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l <= r }
        Le(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l <= r }

        Not(Bool) -> Bool as fn(r: bool) -> bool { !r }

        // byte
        And (Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l & r }
        Or  (Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l | r }
        Xor (Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l ^ r }
        Nand(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { !(l & r) }
        Nor (Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { !(l | r) }
        Xnor(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { !(l ^ r) }

        Add(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l + r } // TBD: should overflow result in a failure?
        Sub(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l - r } // TBD: should overflow result in a failure?
        Mul(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l * r } // TBD: should overflow result in a failure?
        Pow(Byte, Uint) -> Byte as fn(l: byte, r: uint) -> byte { l.pow(*r as u32) } // TBD: should overflow result in a failure?
        Div(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l.checked_div(*r).ok_or(Failure::DivByZero)? }
        Rem(Byte, Byte) -> Byte as fn(l: byte, r: byte) -> byte { l.checked_rem(*r).ok_or(Failure::DivByZero)? }

        Shl (Byte, Uint) -> Byte as fn(l: byte, r: uint) -> byte { l.bshl(*r) }
        Shr (Byte, Uint) -> Byte as fn(l: byte, r: uint) -> byte { l.bshr(*r) }
        Rotl(Byte, Uint) -> Byte as fn(l: byte, r: uint) -> byte { l.rotl(*r) }
        Rotr(Byte, Uint) -> Byte as fn(l: byte, r: uint) -> byte { l.rotr(*r) }

        Shl (Byte, Sint) -> Byte as fn(l: byte, r: sint) -> byte { l.bshl(*r) }
        Shr (Byte, Sint) -> Byte as fn(l: byte, r: sint) -> byte { l.bshr(*r) }
        Rotl(Byte, Sint) -> Byte as fn(l: byte, r: sint) -> byte { l.rotl(*r) }
        Rotr(Byte, Sint) -> Byte as fn(l: byte, r: sint) -> byte { l.rotr(*r) }

        Not(Uint) -> Uint as fn(r: uint) -> uint { !r }

        // uint
        And (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l & r }
        Or  (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l | r }
        Xor (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l ^ r }
        Nand(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { !(l & r) }
        Nor (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { !(l | r) }
        Xnor(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { !(l ^ r) }

        Add(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l + r } // TBD: should overflow result in a failure?
        Sub(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l - r } // TBD: should overflow result in a failure?
        Mul(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l * r } // TBD: should overflow result in a failure?
        Pow(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.pow(*r as u32) } // TBD: should overflow result in a failure?
        Div(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.checked_div(*r).ok_or(Failure::DivByZero)? }
        Rem(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.checked_rem(*r).ok_or(Failure::DivByZero)? }

        Shl (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.bshl(*r) }
        Shr (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.bshr(*r) }
        Rotl(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.rotl(*r) }
        Rotr(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.rotr(*r) }

        Shl (Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.bshl(*r) }
        Shr (Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.bshr(*r) }
        Rotl(Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.rotl(*r) }
        Rotr(Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.rotr(*r) }

        Not(Uint) -> Uint as fn(r: uint) -> uint { !r }

        // sint
        And (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l & r }
        Or  (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l | r }
        Xor (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l ^ r }
        Nand(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { !(l & r) }
        Nor (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { !(l | r) }
        Xnor(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { !(l ^ r) }

        Add(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l + r } // TBD: should overflow result in a failure?
        Sub(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l - r } // TBD: should overflow result in a failure?
        Mul(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l * r } // TBD: should overflow result in a failure?
        Pow(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.pow(*r as u32) } // TBD: should overflow result in a failure?
        Div(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.checked_div(*r).ok_or(Failure::DivByZero)? }
        Rem(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.checked_rem(*r).ok_or(Failure::DivByZero)? }

        Shl (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.bshl(*r) }
        Shr (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.bshr(*r) }
        Rotl(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.rotl(*r) }
        Rotr(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.rotr(*r) }

        Shl (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.bshl(*r) }
        Shr (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.bshr(*r) }
        Rotl(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.rotl(*r) }
        Rotr(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.rotr(*r) }

        Neg(Sint) -> Sint as fn(r: sint) -> sint { -r } // TBD: should overflow result in a failure?
        Not(Sint) -> Bool as fn(r: sint) -> sint { !r }

        // flpt
        Add(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l + r }
        Sub(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l - r }
        Mul(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l * r }
        Pow(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l.powf(*r) }
        Div(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l / r }
        Rem(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l % r }

        Neg(Flpt) -> Flpt as fn(r: flpt) -> flpt { -r }

        // char
        Add(Char, Uint) -> Char as fn(l: char, r: uint) -> char {
            u32::try_from(*r)
                .ok()
                .and_then(|r| l.to_u32().checked_add(r))
                .and_then(char::from_u32)
                .ok_or(Failure::Utf8Error)?
        }
        Sub(Char, Uint) -> Char as fn(l: char, r: uint) -> char {
            u32::try_from(*r)
                .ok()
                .and_then(|r| l.to_u32().checked_add(r))
                .and_then(char::from_u32)
                .ok_or(Failure::Utf8Error)?
        }

        // text
        Add(Text, Text) -> Text as fn(l: text, r: text) -> text { format!("{l}{r}") }

        Add(Text, Bool) -> Text as fn(l: text, r: bool) -> text { format!("{l}{r}") }
        Add(Text, Byte) -> Text as fn(l: text, r: byte) -> text { format!("{l}{r}") }
        Add(Text, Uint) -> Text as fn(l: text, r: uint) -> text { format!("{l}{r}") }
        Add(Text, Sint) -> Text as fn(l: text, r: sint) -> text { format!("{l}{r}") }
        Add(Text, Flpt) -> Text as fn(l: text, r: flpt) -> text { format!("{l}{r}") }
        Add(Text, Char) -> Text as fn(l: text, r: char) -> text { format!("{l}{r}") }
        Add(Text, Text) -> Text as fn(l: text, r: text) -> text { format!("{l}{r}") }

        Add(Bool, Text) -> Text as fn(l: bool, r: text) -> text { format!("{l}{r}") }
        Add(Byte, Text) -> Text as fn(l: byte, r: text) -> text { format!("{l}{r}") }
        Add(Uint, Text) -> Text as fn(l: uint, r: text) -> text { format!("{l}{r}") }
        Add(Sint, Text) -> Text as fn(l: sint, r: text) -> text { format!("{l}{r}") }
        Add(Flpt, Text) -> Text as fn(l: flpt, r: text) -> text { format!("{l}{r}") }
        Add(Char, Text) -> Text as fn(l: char, r: text) -> text { format!("{l}{r}") }
        Add(Text, Text) -> Text as fn(l: text, r: text) -> text { format!("{l}{r}") }
    ].into_iter().chain([
        incr_decr_op!((Bool)Incr -> Bool as fn(rhs: bool) -> bool {
            std::mem::replace(rhs, if *rhs { todo!("iterator finality") } else { true })
        }),
        incr_decr_op!((Bool)Decr -> Bool as fn(rhs: bool) -> bool {
            std::mem::replace(rhs, if *rhs { false } else { todo!("iterator finality") })
        }),
        incr_decr_op!((Byte)Incr -> Byte as fn(rhs: byte) -> byte {
            std::mem::replace(rhs, rhs.checked_add(1).unwrap_or_else(|| todo!("iterator finality")))
        }),
        incr_decr_op!((Byte)Decr -> Byte as fn(rhs: byte) -> byte {
            std::mem::replace(rhs, rhs.checked_sub(1).unwrap_or_else(|| todo!("iterator finality")))
        }),
        incr_decr_op!((Uint)Incr -> Uint as fn(rhs: uint) -> uint {
            std::mem::replace(rhs, rhs.checked_add(1).unwrap_or_else(|| todo!("iterator finality")))
        }),
        incr_decr_op!((Uint)Decr -> Uint as fn(rhs: uint) -> uint {
            std::mem::replace(rhs, rhs.checked_sub(1).unwrap_or_else(|| todo!("iterator finality")))
        }),
        incr_decr_op!((Sint)Incr -> Sint as fn(rhs: sint) -> sint {
            std::mem::replace(rhs, rhs.checked_add(1).unwrap_or_else(|| todo!("iterator finality")))
        }),
        incr_decr_op!((Sint)Decr -> Sint as fn(rhs: sint) -> sint {
            std::mem::replace(rhs, rhs.checked_sub(1).unwrap_or_else(|| todo!("iterator finality")))
        }),
        incr_decr_op!((Char)Incr -> Char as fn(rhs: char) -> char {
            rhs.to_u32().checked_add(1).and_then(char::from_u32).unwrap_or_else(|| todo!("iterator finality"))
        }),
        incr_decr_op!((Char)Decr -> Char as fn(rhs: char) -> char {
            rhs.to_u32().checked_sub(1).and_then(char::from_u32).unwrap_or_else(|| todo!("iterator finality"))
        }),
        incr_decr_op!((Text)Incr -> Char as fn(rhs: text) -> char {
            if rhs.is_empty() { todo!("iterator finality") } else { rhs.remove(0) }
        }),
    ])
}
