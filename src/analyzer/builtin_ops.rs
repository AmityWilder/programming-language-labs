#![allow(unused, reason = "under construction")]
use super::{
    AbstractType, BlockStmt, Environment, FnDef, FunType, OpOverload, OperandTypes, Operator,
    PrimitiveType, StaticType, Value, ValueMut, types::FunShape,
};
use crate::types::*;

macro_rules! prim_op {
    (
        $op:ident($lhs:ident, $rhs:ident) -> $output:ident

        as fn($env:ident, $lhs_name:ident: $Lhs:ty, $rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $def:expr
        }
    ) => {
        (
            OpOverload {
                op: Operator::$op,
                operands: OperandTypes::Binary(
                    AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$lhs)),
                    AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$rhs)),
                ),
                output: AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$output)),
                def: FunType {
                    name: Operator::$op.as_str(),
                    shape: FunShape {
                        params: vec![
                            StaticType::Primitive(PrimitiveType::$lhs),
                            StaticType::Primitive(PrimitiveType::$rhs),
                        ],
                        ret: StaticType::Primitive(PrimitiveType::$output),
                    },
                },
            },
            {
                fn fun<'src, 'val>(
                    $env: &'val mut Environment,
                    lhs: Value<'val, 'src>,
                    rhs: Value<'val, 'src>,
                    out: ValueMut<'val, 'src>,
                ) {
                    // SAFETY: static analyzer must ensure soundness
                    let $lhs_name = unsafe { Value::cast_into::<$Lhs>(lhs) };
                    // SAFETY: static analyzer must ensure soundness
                    let $rhs_name = unsafe { Value::cast_into::<$Rhs>(rhs) };
                    let res = $def;
                    // SAFETY: static analyzer must ensure soundness
                    *unsafe { ValueMut::cast_into::<$Out>(out) } = res;
                }
                FnDef::LangOpB(fun)
            },
        )
    };

    (
        $op:ident($rhs:ident) -> $output:ident

        as fn($env:ident, $rhs_name:ident: $Rhs:ty) -> $Out:ty {
            $def:expr
        }
    ) => {
        (
            OpOverload {
                op: Operator::$op,
                operands: OperandTypes::UnaryPre(AbstractType::Concrete(StaticType::Primitive(
                    PrimitiveType::$rhs,
                ))),
                output: AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$output)),
                def: FunType {
                    name: Operator::$op.as_str(),
                    shape: FunShape {
                        params: vec![StaticType::Primitive(PrimitiveType::$rhs)],
                        ret: StaticType::Primitive(PrimitiveType::$output),
                    },
                },
            },
            {
                fn fun<'src, 'val>(
                    $env: &'val mut Environment,
                    rhs: Value<'val, 'src>,
                    out: ValueMut<'val, 'src>,
                ) {
                    // SAFETY: static analyzer must ensure soundness
                    let $rhs_name = unsafe { Value::cast_into::<$Rhs>(rhs) };
                    let res = $def;
                    // SAFETY: static analyzer must ensure soundness
                    *unsafe { ValueMut::cast_into::<$Out>(out) } = res;
                }
                FnDef::LangOpU(fun)
            },
        )
    };

    (
        ($lhs:ident)$op:ident -> $output:ident

        as fn($env:ident, $lhs_name:ident: $Lhs:ty) -> $Out:ty {
            $def:expr
        }
    ) => {
        (
            OpOverload {
                op: Operator::$op,
                operands: OperandTypes::UnaryPost(AbstractType::Concrete(StaticType::Primitive(
                    PrimitiveType::$lhs,
                ))),
                output: AbstractType::Concrete(StaticType::Primitive(PrimitiveType::$output)),
                def: FunType {
                    name: Operator::$op.as_str(),
                    shape: FunShape {
                        params: vec![StaticType::Primitive(PrimitiveType::$lhs)],
                        ret: StaticType::Primitive(PrimitiveType::$output),
                    },
                },
            },
            {
                fn fun<'src, 'val>(
                    $env: &'val mut Environment,
                    lhs: Value<'val, 'src>,
                    out: ValueMut<'val, 'src>,
                ) {
                    // SAFETY: static analyzer must ensure soundness
                    let $lhs_name = unsafe { Value::cast_into::<$Lhs>(lhs) };
                    let res = $def;
                    // SAFETY: static analyzer must ensure soundness
                    *unsafe { ValueMut::cast_into::<$Out>(out) } = res;
                }
                FnDef::LangOpU(fun)
            },
        )
    };
}

macro_rules! primitive_ops {
    ($(
        $($pre_bin_op:ident)?($lhs_unary:ident$(, $rhs:ident)?)$($post_op:ident)? -> $output:ident

        as fn($env:ident, $lhs_unary_name:ident: $Lhs:ty$(, $rhs_name:ident: $Rhs:ty)?) -> $Out:ty {
            $def:expr
        }
    )*) => {
        pub fn builtin_ops() -> impl IntoIterator<IntoIter: ExactSizeIterator + DoubleEndedIterator, Item = (OpOverload<'static>, FnDef<'static>)> {
            vec![$(
                prim_op!(
                    $($pre_bin_op)?($lhs_unary$(, $rhs)?)$($post_op)? -> $output

                    as fn($env, $lhs_unary_name: $Lhs$(, $rhs_name: $Rhs)?) -> $Out {
                        $def
                    }
                )
            ),*]
        }
    };
}

// TBD: should assignment be a special case, since it should be defined for *all* types?
primitive_ops![
    // direct assignment
    Assign(None, None) -> None as fn(env, l: none, r: none) -> none { todo!() }
    Assign(Fail, Fail) -> None as fn(env, l: fail, r: fail) -> none { todo!() }
    Assign(Bool, Bool) -> None as fn(env, l: bool, r: bool) -> none { todo!() }
    Assign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    Assign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!() }
    Assign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }
    Assign(Char, Char) -> None as fn(env, l: char, r: char) -> none { todo!() }
    Assign(Text, Text) -> None as fn(env, l: text, r: text) -> none { todo!() }

    // bool-assign
    AndAssign (Bool, Bool) -> None as fn (env, l: bool, r: bool) -> none { todo!() }
    OrAssign  (Bool, Bool) -> None as fn (env, l: bool, r: bool) -> none { todo!() }
    XorAssign (Bool, Bool) -> None as fn (env, l: bool, r: bool) -> none { todo!() }
    NandAssign(Bool, Bool) -> None as fn (env, l: bool, r: bool) -> none { todo!() }
    NorAssign (Bool, Bool) -> None as fn (env, l: bool, r: bool) -> none { todo!() }
    XnorAssign(Bool, Bool) -> None as fn (env, l: bool, r: bool) -> none { todo!() }

    // uint-assign
    AndAssign (Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    OrAssign  (Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    XorAssign (Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    NandAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    NorAssign (Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    XnorAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }

    AddAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    SubAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    MulAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    PowAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    DivAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    RemAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }

    ShlAssign (Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    ShrAssign (Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    RotlAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }
    RotrAssign(Uint, Uint) -> None as fn(env, l: uint, r: uint) -> none { todo!() }

    ShlAssign (Uint, Sint) -> None as fn(env, l: uint, r: sint) -> none { todo!() }
    ShrAssign (Uint, Sint) -> None as fn(env, l: uint, r: sint) -> none { todo!() }
    RotlAssign(Uint, Sint) -> None as fn(env, l: uint, r: sint) -> none { todo!() }
    RotrAssign(Uint, Sint) -> None as fn(env, l: uint, r: sint) -> none { todo!() }

    // sint-assign
    AndAssign (Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l &= r") }
    OrAssign  (Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l |= r") }
    XorAssign (Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l ^= r") }
    NandAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = !(l & r)") }
    NorAssign (Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = !(l | r)") }
    XnorAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = !(l ^ r)") }

    AddAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l += r") }
    SubAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l -= r") }
    MulAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l *= r") }
    PowAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = l.pow(r)") }
    DivAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l /= r") }
    RemAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l %= r") }

    ShlAssign (Sint, Uint) -> None as fn(env, l: sint, r: uint) -> none { todo!("l = l.bshl(r)") }
    ShrAssign (Sint, Uint) -> None as fn(env, l: sint, r: uint) -> none { todo!("l = l.bshr(r)") }
    RotlAssign(Sint, Uint) -> None as fn(env, l: sint, r: uint) -> none { todo!("l = l.rotl(r)") }
    RotrAssign(Sint, Uint) -> None as fn(env, l: sint, r: uint) -> none { todo!("l = l.rotr(r)") }

    ShlAssign (Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = l.bshl(r)") }
    ShrAssign (Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = l.bshr(r)") }
    RotlAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = l.rotl(r)") }
    RotrAssign(Sint, Sint) -> None as fn(env, l: sint, r: sint) -> none { todo!("l = l.rotr(r)") }

    // flpt-assign
    AddAssign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }
    SubAssign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }
    MulAssign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }
    PowAssign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }
    DivAssign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }
    RemAssign(Flpt, Flpt) -> None as fn(env, l: flpt, r: flpt) -> none { todo!() }

    // char-assign
    AddAssign(Char, Uint) -> None as fn(env, l: char, r: uint) -> none { todo!() }
    SubAssign(Char, Uint) -> None as fn(env, l: char, r: uint) -> none { todo!() }

    // text-assign
    AddAssign(Text, Text) -> None as fn(env, l: text, r: text) -> none { todo!() }

    AddAssign(Text, Bool) -> None as fn(env, l: text, r: bool) -> none { todo!() }
    AddAssign(Text, Uint) -> None as fn(env, l: text, r: uint) -> none { todo!() }
    AddAssign(Text, Sint) -> None as fn(env, l: text, r: sint) -> none { todo!() }
    AddAssign(Text, Flpt) -> None as fn(env, l: text, r: flpt) -> none { todo!() }
    AddAssign(Text, Char) -> None as fn(env, l: text, r: char) -> none { todo!() }
    AddAssign(Text, Text) -> None as fn(env, l: text, r: text) -> none { todo!() }

    AddAssign(Bool, Text) -> None as fn(env, l: bool, r: text) -> none { todo!() }
    AddAssign(Uint, Text) -> None as fn(env, l: uint, r: text) -> none { todo!() }
    AddAssign(Sint, Text) -> None as fn(env, l: sint, r: text) -> none { todo!() }
    AddAssign(Flpt, Text) -> None as fn(env, l: flpt, r: text) -> none { todo!() }
    AddAssign(Char, Text) -> None as fn(env, l: char, r: text) -> none { todo!() }
    AddAssign(Text, Text) -> None as fn(env, l: text, r: text) -> none { todo!() }

    // bool
    Exists(None) -> Bool as fn(env, r: none) -> none { todo!() }
    Exists(Fail) -> Bool as fn(env, r: fail) -> none { todo!() }
    Exists(Bool) -> Bool as fn(env, r: bool) -> none { todo!() }
    Exists(Uint) -> Bool as fn(env, r: uint) -> none { todo!() }
    Exists(Sint) -> Bool as fn(env, r: sint) -> none { todo!() }
    Exists(Flpt) -> Bool as fn(env, r: flpt) -> none { todo!() }
    Exists(Char) -> Bool as fn(env, r: char) -> none { todo!() }
    Exists(Text) -> Bool as fn(env, r: text) -> none { todo!() }

    And (Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { l & r }
    Or  (Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!("l | r") }
    Xor (Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!("l ^ r") }
    Nand(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!("!(l & r)") }
    Nor (Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!("!(l | r)") }
    Xnor(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!("!(l ^ r)") }

    Eq(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!() }
    Eq(Uint, Uint) -> Bool as fn(env, l: uint, r: uint) -> bool { todo!() }
    Eq(Sint, Sint) -> Bool as fn(env, l: sint, r: sint) -> bool { todo!() }
    Eq(Flpt, Flpt) -> Bool as fn(env, l: flpt, r: flpt) -> bool { todo!() }
    Eq(Char, Char) -> Bool as fn(env, l: char, r: char) -> bool { todo!() }
    Eq(Text, Text) -> Bool as fn(env, l: text, r: text) -> bool { todo!() }

    Ne(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!() }
    Ne(Uint, Uint) -> Bool as fn(env, l: uint, r: uint) -> bool { todo!() }
    Ne(Sint, Sint) -> Bool as fn(env, l: sint, r: sint) -> bool { todo!() }
    Ne(Flpt, Flpt) -> Bool as fn(env, l: flpt, r: flpt) -> bool { todo!() }
    Ne(Char, Char) -> Bool as fn(env, l: char, r: char) -> bool { todo!() }
    Ne(Text, Text) -> Bool as fn(env, l: text, r: text) -> bool { todo!() }

    Gt(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!() }
    Gt(Uint, Uint) -> Bool as fn(env, l: uint, r: uint) -> bool { todo!() }
    Gt(Sint, Sint) -> Bool as fn(env, l: sint, r: sint) -> bool { todo!() }
    Gt(Flpt, Flpt) -> Bool as fn(env, l: flpt, r: flpt) -> bool { todo!() }
    Gt(Char, Char) -> Bool as fn(env, l: char, r: char) -> bool { todo!() }
    Gt(Text, Text) -> Bool as fn(env, l: text, r: text) -> bool { todo!() }

    Lt(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!() }
    Lt(Uint, Uint) -> Bool as fn(env, l: uint, r: uint) -> bool { todo!() }
    Lt(Sint, Sint) -> Bool as fn(env, l: sint, r: sint) -> bool { todo!() }
    Lt(Flpt, Flpt) -> Bool as fn(env, l: flpt, r: flpt) -> bool { todo!() }
    Lt(Char, Char) -> Bool as fn(env, l: char, r: char) -> bool { todo!() }
    Lt(Text, Text) -> Bool as fn(env, l: text, r: text) -> bool { todo!() }

    Ge(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!() }
    Ge(Uint, Uint) -> Bool as fn(env, l: uint, r: uint) -> bool { todo!() }
    Ge(Sint, Sint) -> Bool as fn(env, l: sint, r: sint) -> bool { todo!() }
    Ge(Flpt, Flpt) -> Bool as fn(env, l: flpt, r: flpt) -> bool { todo!() }
    Ge(Char, Char) -> Bool as fn(env, l: char, r: char) -> bool { todo!() }
    Ge(Text, Text) -> Bool as fn(env, l: text, r: text) -> bool { todo!() }

    Le(Bool, Bool) -> Bool as fn(env, l: bool, r: bool) -> bool { todo!() }
    Le(Uint, Uint) -> Bool as fn(env, l: uint, r: uint) -> bool { todo!() }
    Le(Sint, Sint) -> Bool as fn(env, l: sint, r: sint) -> bool { todo!() }
    Le(Flpt, Flpt) -> Bool as fn(env, l: flpt, r: flpt) -> bool { todo!() }
    Le(Char, Char) -> Bool as fn(env, l: char, r: char) -> bool { todo!() }
    Le(Text, Text) -> Bool as fn(env, l: text, r: text) -> bool { todo!() }

    Not(Bool) -> Bool as fn(env, r: bool) -> bool { todo!() }

    // uint
    And (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Or  (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Xor (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Nand(Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Nor (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Xnor(Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }

    Add   (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    SubNeg(Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Mul   (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Pow   (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Div   (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Rem   (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }

    Shl (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Shr (Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Rotl(Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }
    Rotr(Uint, Uint) -> Uint as fn(env, l: uint, r: uint) -> uint { todo!() }

    Shl (Uint, Sint) -> Uint as fn(env, l: uint, r: sint) -> uint { todo!() }
    Shr (Uint, Sint) -> Uint as fn(env, l: uint, r: sint) -> uint { todo!() }
    Rotl(Uint, Sint) -> Uint as fn(env, l: uint, r: sint) -> uint { todo!() }
    Rotr(Uint, Sint) -> Uint as fn(env, l: uint, r: sint) -> uint { todo!() }

    // sint
    And (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Or  (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Xor (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Nand(Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Nor (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Xnor(Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }

    Add   (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    SubNeg(Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Mul   (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Pow   (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Div   (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Rem   (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }

    Shl (Sint, Uint) -> Sint as fn(env, l: sint, r: uint) -> sint { todo!() }
    Shr (Sint, Uint) -> Sint as fn(env, l: sint, r: uint) -> sint { todo!() }
    Rotl(Sint, Uint) -> Sint as fn(env, l: sint, r: uint) -> sint { todo!() }
    Rotr(Sint, Uint) -> Sint as fn(env, l: sint, r: uint) -> sint { todo!() }

    Shl (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Shr (Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Rotl(Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }
    Rotr(Sint, Sint) -> Sint as fn(env, l: sint, r: sint) -> sint { todo!() }

    SubNeg(Sint) -> Sint as fn(env, r: sint) -> sint { todo!() }

    // flpt
    Add   (Flpt, Flpt) -> Flpt as fn(env, l: flpt, r: flpt) -> flpt { todo!() }
    SubNeg(Flpt, Flpt) -> Flpt as fn(env, l: flpt, r: flpt) -> flpt { todo!() }
    Mul   (Flpt, Flpt) -> Flpt as fn(env, l: flpt, r: flpt) -> flpt { todo!() }
    Pow   (Flpt, Flpt) -> Flpt as fn(env, l: flpt, r: flpt) -> flpt { todo!() }
    Div   (Flpt, Flpt) -> Flpt as fn(env, l: flpt, r: flpt) -> flpt { todo!() }
    Rem   (Flpt, Flpt) -> Flpt as fn(env, l: flpt, r: flpt) -> flpt { todo!() }

    SubNeg(Flpt) -> Flpt as fn(env, r: flpt) -> flpt { todo!() }

    // char
    Add   (Char, Uint) -> Char as fn(env, l: char, r: uint) -> char { todo!() }
    SubNeg(Char, Uint) -> Char as fn(env, l: char, r: uint) -> char { todo!() }

    // text
    Add(Text, Text) -> Text as fn(env, l: text, r: text) -> text { todo!() }

    Add(Text, Bool) -> Text as fn(env, l: text, r: bool) -> text { todo!() }
    Add(Text, Uint) -> Text as fn(env, l: text, r: uint) -> text { todo!() }
    Add(Text, Sint) -> Text as fn(env, l: text, r: sint) -> text { todo!() }
    Add(Text, Flpt) -> Text as fn(env, l: text, r: flpt) -> text { todo!() }
    Add(Text, Char) -> Text as fn(env, l: text, r: char) -> text { todo!() }
    Add(Text, Text) -> Text as fn(env, l: text, r: text) -> text { todo!() }

    Add(Bool, Text) -> Text as fn(env, l: bool, r: text) -> text { todo!() }
    Add(Uint, Text) -> Text as fn(env, l: uint, r: text) -> text { todo!() }
    Add(Sint, Text) -> Text as fn(env, l: sint, r: text) -> text { todo!() }
    Add(Flpt, Text) -> Text as fn(env, l: flpt, r: text) -> text { todo!() }
    Add(Char, Text) -> Text as fn(env, l: char, r: text) -> text { todo!() }
    Add(Text, Text) -> Text as fn(env, l: text, r: text) -> text { todo!() }
];
