#![allow(unused, reason = "under construction")]
use super::{
    AbstractType, BlockStmt, Environment, Failure, FnDef, FunType, OpOverload, OperandTypes,
    Operator, PrimitiveType, StaticType, types::FunShape,
};
use crate::{
    exec::value::{Value, ValueMut},
    interp::value::{LangVar, LangVarCast},
    spec_ops::{BitRotate, BitShift},
    types::*,
};

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
                    fn interp<'inr>(
                        lhs: &(dyn 'inr + LangVar),
                        rhs: &(dyn 'inr + LangVar),
                    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
                        let $lhs_name = lhs
                            .downcast_to::<$Lhs>()
                            .expect("should be guaranteed by static analysis");
                        let $rhs_name = rhs
                            .downcast_to::<$Rhs>()
                            .expect("should be guaranteed by static analysis");
                        let res: $Out = $def;
                        Ok(Box::new(res.cast_into()))
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
                    fn interp<'inr>(
                        rhs: &(dyn 'inr + LangVar),
                    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
                        let $rhs_name = rhs
                            .downcast_to::<$Rhs>()
                            .expect("should be guaranteed by static analysis");
                        let res: $Out = $def;
                        Ok(Box::new(res.cast_into()))
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
                        lhs: &(dyn 'inr + LangVar),
                    ) -> Result<Box<dyn 'inr + LangVar>, Failure> {
                        let $lhs_name = lhs
                            .downcast_to::<$Lhs>()
                            .expect("should be guaranteed by static analysis");
                        let res: $Out = $def;
                        Ok(Box::new(res.cast_into()))
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
        pub fn builtin_ops() -> impl IntoIterator<IntoIter: ExactSizeIterator + DoubleEndedIterator, Item = (Operator, OpOverload<'static>)> {
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
    // bool-assign
    AndAssign (Bool, Bool) -> None as fn(l: bool, r: bool) -> none { todo!() }
    OrAssign  (Bool, Bool) -> None as fn(l: bool, r: bool) -> none { todo!() }
    XorAssign (Bool, Bool) -> None as fn(l: bool, r: bool) -> none { todo!() }
    NandAssign(Bool, Bool) -> None as fn(l: bool, r: bool) -> none { todo!() }
    NorAssign (Bool, Bool) -> None as fn(l: bool, r: bool) -> none { todo!() }
    XnorAssign(Bool, Bool) -> None as fn(l: bool, r: bool) -> none { todo!() }

    // uint-assign
    AndAssign (Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    OrAssign  (Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    XorAssign (Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    NandAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    NorAssign (Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    XnorAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }

    AddAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    SubAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    MulAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    PowAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    DivAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    RemAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }

    ShlAssign (Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    ShrAssign (Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    RotlAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }
    RotrAssign(Uint, Uint) -> None as fn(l: uint, r: uint) -> none { todo!() }

    ShlAssign (Uint, Sint) -> None as fn(l: uint, r: sint) -> none { todo!() }
    ShrAssign (Uint, Sint) -> None as fn(l: uint, r: sint) -> none { todo!() }
    RotlAssign(Uint, Sint) -> None as fn(l: uint, r: sint) -> none { todo!() }
    RotrAssign(Uint, Sint) -> None as fn(l: uint, r: sint) -> none { todo!() }

    // sint-assign
    AndAssign (Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l &= r") }
    OrAssign  (Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l |= r") }
    XorAssign (Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l ^= r") }
    NandAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = !(l & r)") }
    NorAssign (Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = !(l | r)") }
    XnorAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = !(l ^ r)") }

    AddAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l += r") }
    SubAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l -= r") }
    MulAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l *= r") }
    PowAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = l.pow(r)") }
    DivAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l /= r") }
    RemAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l %= r") }

    ShlAssign (Sint, Uint) -> None as fn(l: sint, r: uint) -> none { todo!("l = l.bshl(r)") }
    ShrAssign (Sint, Uint) -> None as fn(l: sint, r: uint) -> none { todo!("l = l.bshr(r)") }
    RotlAssign(Sint, Uint) -> None as fn(l: sint, r: uint) -> none { todo!("l = l.rotl(r)") }
    RotrAssign(Sint, Uint) -> None as fn(l: sint, r: uint) -> none { todo!("l = l.rotr(r)") }

    ShlAssign (Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = l.bshl(r)") }
    ShrAssign (Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = l.bshr(r)") }
    RotlAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = l.rotl(r)") }
    RotrAssign(Sint, Sint) -> None as fn(l: sint, r: sint) -> none { todo!("l = l.rotr(r)") }

    // flpt-assign
    AddAssign(Flpt, Flpt) -> None as fn(l: flpt, r: flpt) -> none { todo!() }
    SubAssign(Flpt, Flpt) -> None as fn(l: flpt, r: flpt) -> none { todo!() }
    MulAssign(Flpt, Flpt) -> None as fn(l: flpt, r: flpt) -> none { todo!() }
    PowAssign(Flpt, Flpt) -> None as fn(l: flpt, r: flpt) -> none { todo!() }
    DivAssign(Flpt, Flpt) -> None as fn(l: flpt, r: flpt) -> none { todo!() }
    RemAssign(Flpt, Flpt) -> None as fn(l: flpt, r: flpt) -> none { todo!() }

    // char-assign
    AddAssign(Char, Uint) -> None as fn(l: char, r: uint) -> none { todo!() }
    SubAssign(Char, Uint) -> None as fn(l: char, r: uint) -> none { todo!() }

    // text-assign
    AddAssign(Text, Text) -> None as fn(l: text, r: text) -> none { todo!() }

    AddAssign(Text, Bool) -> None as fn(l: text, r: bool) -> none { todo!() }
    AddAssign(Text, Uint) -> None as fn(l: text, r: uint) -> none { todo!() }
    AddAssign(Text, Sint) -> None as fn(l: text, r: sint) -> none { todo!() }
    AddAssign(Text, Flpt) -> None as fn(l: text, r: flpt) -> none { todo!() }
    AddAssign(Text, Char) -> None as fn(l: text, r: char) -> none { todo!() }
    AddAssign(Text, Text) -> None as fn(l: text, r: text) -> none { todo!() }

    AddAssign(Bool, Text) -> None as fn(l: bool, r: text) -> none { todo!() }
    AddAssign(Uint, Text) -> None as fn(l: uint, r: text) -> none { todo!() }
    AddAssign(Sint, Text) -> None as fn(l: sint, r: text) -> none { todo!() }
    AddAssign(Flpt, Text) -> None as fn(l: flpt, r: text) -> none { todo!() }
    AddAssign(Char, Text) -> None as fn(l: char, r: text) -> none { todo!() }
    AddAssign(Text, Text) -> None as fn(l: text, r: text) -> none { todo!() }

    // bool
    Exists(None) -> Bool as fn(r: none) -> none { todo!() }
    Exists(Fail) -> Bool as fn(r: fail) -> none { todo!() }
    Exists(Bool) -> Bool as fn(r: bool) -> none { todo!() }
    Exists(Uint) -> Bool as fn(r: uint) -> none { todo!() }
    Exists(Sint) -> Bool as fn(r: sint) -> none { todo!() }
    Exists(Flpt) -> Bool as fn(r: flpt) -> none { todo!() }
    Exists(Char) -> Bool as fn(r: char) -> none { todo!() }
    Exists(Text) -> Bool as fn(r: text) -> none { todo!() }

    And (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l & r }
    Or  (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l | r }
    Xor (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l ^ r }
    Nand(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !(l & r) }
    Nor (Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !(l | r) }
    Xnor(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { !(l ^ r) }

    Eq(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l == r }
    Eq(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l == r }
    Eq(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l == r }
    Eq(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l == r }
    Eq(Char, Char) -> Bool as fn(l: char, r: char) -> bool { l == r }
    Eq(Text, Text) -> Bool as fn(l: text, r: text) -> bool { l == r }

    Ne(Bool, Bool) -> Bool as fn(l: bool, r: bool) -> bool { l != r }
    Ne(Uint, Uint) -> Bool as fn(l: uint, r: uint) -> bool { l != r }
    Ne(Sint, Sint) -> Bool as fn(l: sint, r: sint) -> bool { l != r }
    Ne(Flpt, Flpt) -> Bool as fn(l: flpt, r: flpt) -> bool { l != r }
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

    Add   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l + r }
    SubNeg(Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l - r }
    Mul   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l * r }
    Pow   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l.pow(*r as u32) }
    Div   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l / r }
    Rem   (Uint, Uint) -> Uint as fn(l: uint, r: uint) -> uint { l % r }

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

    Add   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l + r }
    SubNeg(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l - r }
    Mul   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l * r }
    Pow   (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.pow(*r as u32) }
    Div   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l / r }
    Rem   (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l % r }

    Shl (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.bshl(*r) }
    Shr (Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.bshr(*r) }
    Rotl(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.rotl(*r) }
    Rotr(Sint, Uint) -> Sint as fn(l: sint, r: uint) -> sint { l.rotr(*r) }

    Shl (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.bshl(*r) }
    Shr (Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.bshr(*r) }
    Rotl(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.rotl(*r) }
    Rotr(Sint, Sint) -> Sint as fn(l: sint, r: sint) -> sint { l.rotr(*r) }

    SubNeg(Sint) -> Sint as fn(r: sint) -> sint { -r }

    // flpt
    Add   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l + r }
    SubNeg(Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l - r }
    Mul   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l * r }
    Pow   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l.powf(*r) }
    Div   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l / r }
    Rem   (Flpt, Flpt) -> Flpt as fn(l: flpt, r: flpt) -> flpt { l % r }

    SubNeg(Flpt) -> Flpt as fn(r: flpt) -> flpt { -r }

    // char
    Add   (Char, Uint) -> Char as fn(l: char, r: uint) -> char { todo!() }
    SubNeg(Char, Uint) -> Char as fn(l: char, r: uint) -> char { todo!() }

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
