#![allow(unused, reason = "under construction")]
use super::{
    AbstractType, BlockStmt, Environment, Failure, FnDef, FunType, OpOverload, OperandTypes,
    Operator, PrimitiveType, StaticType, types::FunShape,
};
use crate::{
    exec::value::{Value, ValueMut},
    interp::value::InterpVal,
    spec_ops::{BitRotate, BitShift},
    types::*,
};

// TBD: how does Batscript handle overflow?

macro_rules! prim_op {
    (
        $op:ident($lhs:ident, $rhs:ident) -> $output:ident

        as fn($lhs_name:ident: $Lhs:ty, $rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $def:expr
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::Binary(
                    AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$lhs)),
                    AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$rhs)),
                ),
                output: AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$output)),
                fun: FunShape {
                    params: vec![
                        StaticType::Primitive(PrimitiveType::$lhs),
                        StaticType::Primitive(PrimitiveType::$rhs),
                    ],
                    ret: StaticType::Primitive(PrimitiveType::$output),
                },
                def: {
                    fn interp<'src>(
                        lhs: &InterpVal<'src>,
                        rhs: &InterpVal<'src>,
                    ) -> Result<InterpVal<'src>, Failure> {
                        let $lhs_name: &$Lhs = lhs.try_into().unwrap_or_else(|_| {
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
                        let $rhs_name: &$Rhs = rhs.try_into().unwrap_or_else(|_| {
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
                        let res: $Out = $def;
                        Ok(res.into())
                    }
                    fn runtime<'src, 'val>(
                        out: ValueMut<'val, 'src>,
                        lhs: Value<'val, 'src>,
                        rhs: Value<'val, 'src>,
                    ) -> Result<(), Failure> {
                        // SAFETY: static analyzer must ensure soundness
                        let $lhs_name = &unsafe { Value::cast_into::<$Lhs>(lhs).assume_init() };
                        // SAFETY: static analyzer must ensure soundness
                        let $rhs_name = &unsafe { Value::cast_into::<$Rhs>(rhs).assume_init() };
                        let res = $def;
                        // SAFETY: static analyzer must ensure soundness
                        unsafe { ValueMut::cast_into::<$Out>(out) }.data.write(res);
                        Ok(())
                    }
                    FnDef::LangOpB(runtime, interp)
                },
            },
        )
    };

    (
        $op:ident($rhs:ident) -> $output:ident

        as fn($rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $def:expr
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::UnaryPre(AbstractType::Concrete(StaticType::Primitive(
                    PrimitiveType::$rhs,
                ))),
                output: AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$output)),
                fun: FunShape {
                    params: vec![StaticType::Primitive(PrimitiveType::$rhs)],
                    ret: StaticType::Primitive(PrimitiveType::$output),
                },
                def: {
                    fn interp<'src>(rhs: &InterpVal<'src>) -> Result<InterpVal<'src>, Failure> {
                        let $rhs_name: &$Rhs = rhs.try_into().unwrap_or_else(|_| {
                            panic!(
                                "{}<{}> rhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {rhs:?}",
                                stringify!($op),
                                stringify!($Lhs),
                                stringify!($Lhs),
                            )
                        });
                        let res: $Out = $def;
                        Ok(res.into())
                    }
                    fn runtime<'src, 'val>(
                        out: ValueMut<'val, 'src>,
                        rhs: Value<'val, 'src>,
                    ) -> Result<(), Failure> {
                        // SAFETY: static analyzer must ensure soundness
                        let $rhs_name = &unsafe { Value::cast_into::<$Rhs>(rhs).assume_init() };
                        let res = $def;
                        // SAFETY: static analyzer must ensure soundness
                        unsafe { ValueMut::cast_into::<$Out>(out) }.data.write(res);
                        Ok(())
                    }
                    FnDef::LangOpU(runtime, interp)
                },
            },
        )
    };

    (
        ($lhs:ident)$op:ident -> $output:ident

        as fn($lhs_name:ident: $Lhs:ty) -> $Out:ty {
            $def:expr
        }
    ) => {
        (
            Operator::$op,
            OpOverload {
                operands: OperandTypes::UnaryPost(AbstractType::Concrete(StaticType::Primitive(
                    PrimitiveType::$lhs,
                ))),
                output: AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$output)),
                fun: FunShape {
                    params: vec![StaticType::Primitive(PrimitiveType::$lhs)],
                    ret: StaticType::Primitive(PrimitiveType::$output),
                },
                def: {
                    fn interp<'inr>(
                        lhs: &InterpVal<'src>,
                    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
                        let $lhs_name: &$Lhs = lhs.try_into().unwrap_or_else(|_| {
                            panic!(
                                "{}<{}> lhs type mismatch should be caught by static analysis\
                                \n expected: {}\
                                \n    found: {lhs:?}",
                                stringify!($op),
                                stringify!($Lhs),
                                stringify!($Lhs),
                            )
                        });
                        let res: $Out = $def;
                        Ok(res.into())
                    }
                    fn runtime<'src, 'val>(
                        $env: &'val Environment,
                        out: ValueMut<'val, 'src>,
                        lhs: Value<'val, 'src>,
                    ) -> Result<(), Failure> {
                        // SAFETY: static analyzer must ensure soundness
                        let $lhs_name = &unsafe { Value::cast_into::<$Lhs>(lhs).assume_init() };
                        let res = $def;
                        // SAFETY: static analyzer must ensure soundness
                        unsafe { ValueMut::cast_into::<$Out>(out) }.data.write(res);
                        Ok(())
                    }
                    FnDef::LangOpU(runtime, interp)
                },
            },
        )
    };
}

macro_rules! primitive_ops {
    ($(
        $($pre_bin_op:ident)?($lhs_unary:ident$(, $rhs:ident)?)$($post_op:ident)? -> $output:ident

        as fn($lhs_unary_name:ident: $Lhs:ty$(, $rhs_name:ident: $Rhs:ty)?) -> $Out:ty {
            $def:expr
        }
    )*) => {
        pub fn builtin_ops<'src>() -> impl IntoIterator<IntoIter: ExactSizeIterator + DoubleEndedIterator, Item = (Operator, OpOverload<'src>)> {
            vec![$(
                prim_op!(
                    $($pre_bin_op)?($lhs_unary$(, $rhs)?)$($post_op)? -> $output

                    as fn($lhs_unary_name: $Lhs$(, $rhs_name: $Rhs)?) -> $Out {
                        $def
                    }
                )
            ),*]
        }
    };
}

primitive_ops![
    // bool
    Exists(None) -> Bool as fn(r: none) -> bool { false }
    Exists(Fail) -> Bool as fn(r: fail) -> bool { false }
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

    // uint
    And (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l & r }
    Or  (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l | r }
    Xor (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l ^ r }
    Nand(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { !(l & r) }
    Nor (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { !(l | r) }
    Xnor(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { !(l ^ r) }

    Add   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l + r } // TBD: should overflow result in a failure?
    SubNeg(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l - r } // TBD: should overflow result in a failure?
    Mul   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l * r } // TBD: should overflow result in a failure?
    Pow   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.pow(*r as u32) } // TBD: should overflow result in a failure?
    Div   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.checked_div(*r).ok_or(Failure::DivByZero)? }
    Rem   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.checked_rem(*r).ok_or(Failure::DivByZero)? }

    Shl (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.bshl(*r) }
    Shr (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.bshr(*r) }
    Rotl(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.rotl(*r) }
    Rotr(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.rotr(*r) }

    Shl (Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.bshl(*r) }
    Shr (Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.bshr(*r) }
    Rotl(Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.rotl(*r) }
    Rotr(Uint, Sint) -> Uint as fn(l: uint, r: sint) -> uint { l.rotr(*r) }

    // sint
    And (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l & r }
    Or  (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l | r }
    Xor (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l ^ r }
    Nand(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { !(l & r) }
    Nor (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { !(l | r) }
    Xnor(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { !(l ^ r) }

    Add   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l + r } // TBD: should overflow result in a failure?
    SubNeg(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l - r } // TBD: should overflow result in a failure?
    Mul   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l * r } // TBD: should overflow result in a failure?
    Pow   (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.pow(*r as u32) } // TBD: should overflow result in a failure?
    Div   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.checked_div(*r).ok_or(Failure::DivByZero)? }
    Rem   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.checked_rem(*r).ok_or(Failure::DivByZero)? }

    Shl (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.bshl(*r) }
    Shr (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.bshr(*r) }
    Rotl(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.rotl(*r) }
    Rotr(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.rotr(*r) }

    Shl (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.bshl(*r) }
    Shr (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.bshr(*r) }
    Rotl(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.rotl(*r) }
    Rotr(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.rotr(*r) }

    SubNeg(Sint) -> Sint as fn(r: sint) -> sint { -r } // TBD: should overflow result in a failure?

    // flpt
    Add   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l + r }
    SubNeg(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l - r }
    Mul   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l * r }
    Pow   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l.powf(*r) }
    Div   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l / r }
    Rem   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l % r }

    SubNeg(Flpt) -> Flpt as fn(r: flpt) -> flpt { -r }

    // char
    Add(Char, Uint) -> Char as fn(l: char, r: uint) -> char {
        u32::try_from(*r)
            .ok()
            .and_then(|r| l.to_u32().checked_add(r))
            .and_then(char::from_u32)
            .ok_or(Failure::Utf8Error)?
    }
    SubNeg(Char, Uint) -> Char as fn(l: char, r: uint) -> char {
        u32::try_from(*r)
            .ok()
            .and_then(|r| l.to_u32().checked_add(r))
            .and_then(char::from_u32)
            .ok_or(Failure::Utf8Error)?
    }

    // text
    Add(Text, Text) -> Text as fn(l: text, r: text) -> text { format!("{l}{r}") }

    Add(Text, Bool) -> Text as fn(l: text, r: bool) -> text { format!("{l}{r}") }
    Add(Text, Uint) -> Text as fn(l: text, r: uint) -> text { format!("{l}{r}") }
    Add(Text, Sint) -> Text as fn(l: text, r: sint) -> text { format!("{l}{r}") }
    Add(Text, Flpt) -> Text as fn(l: text, r: flpt) -> text { format!("{l}{r}") }
    Add(Text, Char) -> Text as fn(l: text, r: char) -> text { format!("{l}{r}") }
    Add(Text, Text) -> Text as fn(l: text, r: text) -> text { format!("{l}{r}") }

    Add(Bool, Text) -> Text as fn(l: bool, r: text) -> text { format!("{l}{r}") }
    Add(Uint, Text) -> Text as fn(l: uint, r: text) -> text { format!("{l}{r}") }
    Add(Sint, Text) -> Text as fn(l: sint, r: text) -> text { format!("{l}{r}") }
    Add(Flpt, Text) -> Text as fn(l: flpt, r: text) -> text { format!("{l}{r}") }
    Add(Char, Text) -> Text as fn(l: char, r: text) -> text { format!("{l}{r}") }
    Add(Text, Text) -> Text as fn(l: text, r: text) -> text { format!("{l}{r}") }
];
