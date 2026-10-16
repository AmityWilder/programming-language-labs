//! Assembly-style runtime for Batscript

use crate::{
    exec::{
        asm::{
            arch::{VirtualArch, read_word, write_word},
            reg::{Reg, TReg},
        },
        error::Failure,
    },
    spec_ops::BitRotate,
    types::{frac, sint, uint},
};

pub mod arch;
pub mod reg;

macro_rules! instructions {
    (
        arch: $arch:ident;
        pc: $pc:ident;

        $(#[$meta:meta])*
        $vis:vis enum $Enum:ident {$(
            $(#[doc = $doc:expr])*
            #[def($def:expr)]
            $Variant:ident $({$(
                $(#[$fmeta:meta])*
                $field:ident: $Type:ty
            ),* $(,)?})?
        ),* $(,)?}
    ) => {
        $(#[$meta])*
        $vis enum $Enum {$(
            $(#[doc = $doc])*
            #[allow(dead_code, reason = "under construction")]
            $Variant $({$(
                $(#[$fmeta])*
                $field: $Type
            ),*})?
        ),*}

        impl $Enum {
            fn step(self, $pc: &mut usize, $arch: &mut VirtualArch) -> Result<(), Failure> {
                #[allow(clippy::arithmetic_side_effects)]
                match self {
                    $(Self::$Variant $({$(#[allow(unused_variables)] $field),*})? => $def,)*
                }
                Ok(())
            }
        }
    };
}

instructions! {
    arch: arch;
    pc: pc;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Instruction {
        // ---------------------------------------------
        // Register
        // ---------------------------------------------

        /// And bools
        #[def(arch.reg[d] = arch.reg[s] & arch.reg[t])]
        LAnd { d: TReg<bool>, s: TReg<bool>, t: TReg<bool> },
        /// And uints
        #[def(arch.reg[d] = arch.reg[s] & arch.reg[t])]
        UAnd { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// And sints
        #[def(arch.reg[d] = arch.reg[s] & arch.reg[t])]
        SAnd { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Or bools
        #[def(arch.reg[d] = arch.reg[s] | arch.reg[t])]
        LOr { d: TReg<bool>, s: TReg<bool>, t: TReg<bool> },
        /// Or uints
        #[def(arch.reg[d] = arch.reg[s] | arch.reg[t])]
        UOr { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Or sints
        #[def(arch.reg[d] = arch.reg[s] | arch.reg[t])]
        SOr { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Xor bools
        #[def(arch.reg[d] = arch.reg[s] ^ arch.reg[t])]
        LXor { d: TReg<bool>, s: TReg<bool>, t: TReg<bool> },
        /// Xor uints
        #[def(arch.reg[d] = arch.reg[s] ^ arch.reg[t])]
        UXor { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Xor sints
        #[def(arch.reg[d] = arch.reg[s] ^ arch.reg[t])]
        SXor { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Nand bools
        #[def(arch.reg[d] = !(arch.reg[s] & arch.reg[t]))]
        LNand { d: TReg<bool>, s: TReg<bool>, t: TReg<bool> },
        /// Nand uints
        #[def(arch.reg[d] = !(arch.reg[s] & arch.reg[t]))]
        UNand { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Nand sints
        #[def(arch.reg[d] = !(arch.reg[s] & arch.reg[t]))]
        SNand { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Nor bools
        #[def(arch.reg[d] = !(arch.reg[s] | arch.reg[t]))]
        LNor { d: TReg<bool>, s: TReg<bool>, t: TReg<bool> },
        /// Nor uints
        #[def(arch.reg[d] = !(arch.reg[s] | arch.reg[t]))]
        UNor { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Nor sints
        #[def(arch.reg[d] = !(arch.reg[s] | arch.reg[t]))]
        SNor { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Xnor bools
        #[def(arch.reg[d] = !(arch.reg[s] ^ arch.reg[t]))]
        LXnor { d: TReg<bool>, s: TReg<bool>, t: TReg<bool> },
        /// Xnor uints
        #[def(arch.reg[d] = !(arch.reg[s] ^ arch.reg[t]))]
        UXnor { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Xnor sints
        #[def(arch.reg[d] = !(arch.reg[s] ^ arch.reg[t]))]
        SXnor { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Not bool
        #[def(arch.reg[d] = !arch.reg[s])]
        LNot { d: TReg<bool>, s: TReg<bool> },
        /// Not uint
        #[def(arch.reg[d] = !arch.reg[s])]
        UNot { d: TReg<uint>, s: TReg<uint> },
        /// Not sint
        #[def(arch.reg[d] = !arch.reg[s])]
        SNot { d: TReg<sint>, s: TReg<sint> },

        /// Add uints
        #[def(arch.reg[d] = arch.reg[s].saturating_add(arch.reg[t]))]
        UAdd { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Add sints
        #[def(arch.reg[d] = arch.reg[s].saturating_add(arch.reg[t]))]
        SAdd { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },
        /// Add fracs
        #[def(arch.reg[d] = arch.reg[s] + arch.reg[t])]
        FAdd { d: TReg<frac>, s: TReg<frac>, t: TReg<frac> },

        /// Subtract uints
        #[def(arch.reg[d] = arch.reg[s].saturating_sub(arch.reg[t]))]
        USub { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Subtract sints
        #[def(arch.reg[d] = arch.reg[s].saturating_sub(arch.reg[t]))]
        SSub { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },
        /// Subtract fracs
        #[def(arch.reg[d] = arch.reg[s] - arch.reg[t])]
        FSub { d: TReg<frac>, s: TReg<frac>, t: TReg<frac> },

        /// Multiply uints
        #[def(arch.reg[d] = arch.reg[s].saturating_mul(arch.reg[t]))]
        UMul { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Multiply sints
        #[def(arch.reg[d] = arch.reg[s].saturating_mul(arch.reg[t]))]
        SMul { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },
        /// Multiply fracs
        #[def(arch.reg[d] = arch.reg[s] * arch.reg[t])]
        FMul { d: TReg<frac>, s: TReg<frac>, t: TReg<frac> },

        /// Power uints
        #[def(arch.reg[d] = arch.reg[t].try_into().map_or(uint::MAX, |n| arch.reg[s].pow(n)))]
        UPow { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Power sints
        #[def(arch.reg[d] = arch.reg[t].max(0).try_into().map_or(sint::MAX, |n| arch.reg[s].pow(n)))]
        SPow { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },
        /// Power fracs
        #[def(arch.reg[d] = arch.reg[s].powf(arch.reg[t]))]
        FPow { d: TReg<frac>, s: TReg<frac>, t: TReg<frac> },

        /// Divide uints
        #[def(arch.reg[d] = arch.reg[s].checked_div(arch.reg[t]).unwrap_or(uint::MAX))]
        UDiv { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Divide sints
        #[def(arch.reg[d] = arch.reg[s].checked_div(arch.reg[t]).unwrap_or(sint::MAX))]
        SDiv { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },
        /// Divide fracs
        #[def(arch.reg[d] = arch.reg[s] / arch.reg[t])]
        FDiv { d: TReg<frac>, s: TReg<frac>, t: TReg<frac> },

        /// Remainder of uints
        #[def(arch.reg[d] = arch.reg[s].checked_rem(arch.reg[t]).unwrap_or(0))]
        URem { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Remainder of sints
        #[def(arch.reg[d] = arch.reg[s].checked_rem(arch.reg[t]).unwrap_or(0))]
        SRem { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },
        /// Remainder of fracs
        #[def(arch.reg[d] = arch.reg[s] % arch.reg[t])]
        FRem { d: TReg<frac>, s: TReg<frac>, t: TReg<frac> },

        /// Left bitshift of uint
        #[def(arch.reg[d] = arch.reg[t].try_into().map_or(0, |t| arch.reg[s].unbounded_shl(t)))]
        UShl { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Left bitshift of sint
        #[def(arch.reg[d] = arch.reg[t].try_into().map_or(0, |t| arch.reg[s].unbounded_shl(t)))]
        SShl { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Right bitshift of uint
        #[def(arch.reg[d] = arch.reg[t].try_into().map_or(0, |t| arch.reg[s].unbounded_shr(t)))]
        UShr { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Right bitshift of sint
        #[def(arch.reg[d] = arch.reg[t].try_into().map_or(0, |t| arch.reg[s].unbounded_shr(t)))]
        SShr { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotl(arch.reg[t]))]
        UURotl { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotl(arch.reg[t]))]
        USRotl { d: TReg<uint>, s: TReg<uint>, t: TReg<sint> },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotl(arch.reg[t]))]
        SURotl { d: TReg<sint>, s: TReg<sint>, t: TReg<uint> },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotl(arch.reg[t]))]
        SSRotl { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotr(arch.reg[t]))]
        UURotr { d: TReg<uint>, s: TReg<uint>, t: TReg<uint> },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotr(arch.reg[t]))]
        USRotr { d: TReg<uint>, s: TReg<uint>, t: TReg<sint> },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotr(arch.reg[t]))]
        SURotr { d: TReg<sint>, s: TReg<sint>, t: TReg<uint> },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotr(arch.reg[t]))]
        SSRotr { d: TReg<sint>, s: TReg<sint>, t: TReg<sint> },

        /// Set if less than uint
        #[def(arch.reg[d] = arch.reg[s] < arch.reg[t])]
        USlt { d: TReg<bool>, s: TReg<uint>, t: TReg<uint> },
        /// Set if less than sint
        #[def(arch.reg[d] = arch.reg[s] < arch.reg[t])]
        SSlt { d: TReg<bool>, s: TReg<sint>, t: TReg<sint> },
        /// Set if less than frac
        #[def(arch.reg[d] = arch.reg[s] < arch.reg[t])]
        FSlt { d: TReg<bool>, s: TReg<frac>, t: TReg<frac> },

        /// Set if greater than uint
        #[def(arch.reg[d] = arch.reg[s] > arch.reg[t])]
        USgt { d: TReg<bool>, s: TReg<uint>, t: TReg<uint> },
        /// Set if greater than sint
        #[def(arch.reg[d] = arch.reg[s] > arch.reg[t])]
        SSgt { d: TReg<bool>, s: TReg<sint>, t: TReg<sint> },
        /// Set if greater than frac
        #[def(arch.reg[d] = arch.reg[s] > arch.reg[t])]
        FSgt { d: TReg<bool>, s: TReg<frac>, t: TReg<frac> },

        /// Set if less than or equal to uint
        #[def(arch.reg[d] = arch.reg[s] <= arch.reg[t])]
        USle { d: TReg<bool>, s: TReg<uint>, t: TReg<uint> },
        /// Set if less than or equal to sint
        #[def(arch.reg[d] = arch.reg[s] <= arch.reg[t])]
        SSle { d: TReg<bool>, s: TReg<sint>, t: TReg<sint> },
        /// Set if less than or equal to frac
        #[def(arch.reg[d] = arch.reg[s] <= arch.reg[t])]
        FSle { d: TReg<bool>, s: TReg<frac>, t: TReg<frac> },

        /// Set if greater than or equal to uint
        #[def(arch.reg[d] = arch.reg[s] >= arch.reg[t])]
        USge { d: TReg<bool>, s: TReg<uint>, t: TReg<uint> },
        /// Set if greater than or equal to sint
        #[def(arch.reg[d] = arch.reg[s] >= arch.reg[t])]
        SSge { d: TReg<bool>, s: TReg<sint>, t: TReg<sint> },
        /// Set if greater than or equal to frac
        #[def(arch.reg[d] = arch.reg[s] >= arch.reg[t])]
        FSge { d: TReg<bool>, s: TReg<frac>, t: TReg<frac> },

        /// Set if equal to uint
        #[def(arch.reg[d] = arch.reg[s] == arch.reg[t])]
        USeq { d: TReg<bool>, s: TReg<uint>, t: TReg<uint> },
        /// Set if equal to sint
        #[def(arch.reg[d] = arch.reg[s] == arch.reg[t])]
        SSeq { d: TReg<bool>, s: TReg<sint>, t: TReg<sint> },
        /// Set if equal to frac
        #[def(arch.reg[d] = arch.reg[s].eq(&arch.reg[t]))]
        FSeq { d: TReg<bool>, s: TReg<frac>, t: TReg<frac> },

        /// Set if not equal to uint
        #[def(arch.reg[d] = arch.reg[s] != arch.reg[t])]
        USne { d: TReg<bool>, s: TReg<uint>, t: TReg<uint> },
        /// Set if not equal to sint
        #[def(arch.reg[d] = arch.reg[s] != arch.reg[t])]
        SSne { d: TReg<bool>, s: TReg<sint>, t: TReg<sint> },
        /// Set if not equal to frac
        #[def(arch.reg[d] = arch.reg[s].ne(&arch.reg[t]))]
        FSne { d: TReg<bool>, s: TReg<frac>, t: TReg<frac> },

        // ---------------------------------------------
        // Immediate
        // ---------------------------------------------

        /// And bools
        #[def(arch.reg[d] = arch.reg[s] & imm)]
        LAndI { d: TReg<bool>, s: TReg<bool>, imm: bool },
        /// And uints
        #[def(arch.reg[d] = arch.reg[s] & imm)]
        UAndI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// And sints
        #[def(arch.reg[d] = arch.reg[s] & imm)]
        SAndI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Or bools
        #[def(arch.reg[d] = arch.reg[s] | imm)]
        LOrI { d: TReg<bool>, s: TReg<bool>, imm: bool },
        /// Or uints
        #[def(arch.reg[d] = arch.reg[s] | imm)]
        UOrI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Or sints
        #[def(arch.reg[d] = arch.reg[s] | imm)]
        SOrI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Xor bools
        #[def(arch.reg[d] = arch.reg[s] ^ imm)]
        LXorI { d: TReg<bool>, s: TReg<bool>, imm: bool },
        /// Xor uints
        #[def(arch.reg[d] = arch.reg[s] ^ imm)]
        UXorI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Xor sints
        #[def(arch.reg[d] = arch.reg[s] ^ imm)]
        SXorI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Nand bools
        #[def(arch.reg[d] = !(arch.reg[s] & imm))]
        LNandI { d: TReg<bool>, s: TReg<bool>, imm: bool },
        /// Nand uints
        #[def(arch.reg[d] = !(arch.reg[s] & imm))]
        UNandI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Nand sints
        #[def(arch.reg[d] = !(arch.reg[s] & imm))]
        SNandI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Nor bools
        #[def(arch.reg[d] = !(arch.reg[s] | imm))]
        LNorI { d: TReg<bool>, s: TReg<bool>, imm: bool },
        /// Nor uints
        #[def(arch.reg[d] = !(arch.reg[s] | imm))]
        UNorI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Nor sints
        #[def(arch.reg[d] = !(arch.reg[s] | imm))]
        SNorI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Xnor bools
        #[def(arch.reg[d] = !(arch.reg[s] ^ imm))]
        LXnorI { d: TReg<bool>, s: TReg<bool>, imm: bool },
        /// Xnor uints
        #[def(arch.reg[d] = !(arch.reg[s] ^ imm))]
        UXnorI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Xnor sints
        #[def(arch.reg[d] = !(arch.reg[s] ^ imm))]
        SXnorI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Not bool
        #[def(arch.reg[d] = !arch.reg[s])]
        LNotI { d: TReg<bool>, s: TReg<bool> },
        /// Not uint
        #[def(arch.reg[d] = !arch.reg[s])]
        UNotI { d: TReg<uint>, s: TReg<uint> },
        /// Not sint
        #[def(arch.reg[d] = !arch.reg[s])]
        SNotI { d: TReg<sint>, s: TReg<sint> },

        /// Add uints
        #[def(arch.reg[d] = arch.reg[s].saturating_add(imm))]
        UAddI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Add sints
        #[def(arch.reg[d] = arch.reg[s].saturating_add(imm))]
        SAddI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Add fracs
        #[def(arch.reg[d] = arch.reg[s] + imm)]
        FAddI { d: TReg<frac>, s: TReg<frac>, imm: frac },

        /// Subtract uints
        #[def(arch.reg[d] = arch.reg[s].saturating_sub(imm))]
        USubI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Subtract sints
        #[def(arch.reg[d] = arch.reg[s].saturating_sub(imm))]
        SSubI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Subtract fracs
        #[def(arch.reg[d] = arch.reg[s] - imm)]
        FSubI { d: TReg<frac>, s: TReg<frac>, imm: frac },

        /// Multiply uints
        #[def(arch.reg[d] = arch.reg[s].saturating_mul(imm))]
        UMulI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Multiply sints
        #[def(arch.reg[d] = arch.reg[s].saturating_mul(imm))]
        SMulI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Multiply fracs
        #[def(arch.reg[d] = arch.reg[s] * imm)]
        FMulI { d: TReg<frac>, s: TReg<frac>, imm: frac },

        /// Power uints
        #[def(arch.reg[d] = imm.try_into().map_or(uint::MAX, |n| arch.reg[s].pow(n)))]
        UPowI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Power sints
        #[def(arch.reg[d] = imm.max(0).try_into().map_or(sint::MAX, |n| arch.reg[s].pow(n)))]
        SPowI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Power fracs
        #[def(arch.reg[d] = arch.reg[s].powf(imm))]
        FPowI { d: TReg<frac>, s: TReg<frac>, imm: frac },

        /// Divide uints
        #[def(arch.reg[d] = arch.reg[s].checked_div(imm).unwrap_or(uint::MAX))]
        UDivI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Divide sints
        #[def(arch.reg[d] = arch.reg[s].checked_div(imm).unwrap_or(sint::MAX))]
        SDivI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Divide fracs
        #[def(arch.reg[d] = arch.reg[s] / imm)]
        FDivI { d: TReg<frac>, s: TReg<frac>, imm: frac },

        /// Remainder of uints
        #[def(arch.reg[d] = arch.reg[s].checked_rem(imm).unwrap_or(0))]
        URemI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Remainder of sints
        #[def(arch.reg[d] = arch.reg[s].checked_rem(imm).unwrap_or(0))]
        SRemI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Remainder of fracs
        #[def(arch.reg[d] = arch.reg[s] % imm)]
        FRemI { d: TReg<frac>, s: TReg<frac>, imm: frac },

        /// Left bitshift of uint
        #[def(arch.reg[d] = imm.try_into().map_or(0, |t| arch.reg[s].unbounded_shl(t)))]
        UShlI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Left bitshift of sint
        #[def(arch.reg[d] = imm.try_into().map_or(0, |t| arch.reg[s].unbounded_shl(t)))]
        SShlI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Right bitshift of uint
        #[def(arch.reg[d] = imm.try_into().map_or(0, |t| arch.reg[s].unbounded_shr(t)))]
        UShrI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Right bitshift of sint
        #[def(arch.reg[d] = imm.try_into().map_or(0, |t| arch.reg[s].unbounded_shr(t)))]
        SShrI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotl(imm))]
        UURotlI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotl(imm))]
        USRotlI { d: TReg<uint>, s: TReg<uint>, imm: sint },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotl(imm))]
        SURotlI { d: TReg<sint>, s: TReg<sint>, imm: uint },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotl(imm))]
        SSRotlI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotr(imm))]
        UURotrI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotr(imm))]
        USRotrI { d: TReg<uint>, s: TReg<uint>, imm: sint },

        /// Right bit rotate of uint
        #[def(arch.reg[d] = arch.reg[s].rotr(imm))]
        SURotrI { d: TReg<sint>, s: TReg<sint>, imm: uint },
        /// Right bit rotate of sint
        #[def(arch.reg[d] = arch.reg[s].rotr(imm))]
        SSRotrI { d: TReg<sint>, s: TReg<sint>, imm: sint },

        /// Set if less than uint
        #[def(arch.reg[d] = arch.reg[s] < imm)]
        USltI { d: TReg<bool>, s: TReg<uint>, imm: uint },
        /// Set if less than sint
        #[def(arch.reg[d] = arch.reg[s] < imm)]
        SSltI { d: TReg<bool>, s: TReg<sint>, imm: sint },
        /// Set if less than frac
        #[def(arch.reg[d] = arch.reg[s] < imm)]
        FSltI { d: TReg<bool>, s: TReg<frac>, imm: frac },

        /// Set if greater than uint
        #[def(arch.reg[d] = arch.reg[s] > imm)]
        USgtI { d: TReg<bool>, s: TReg<uint>, imm: uint },
        /// Set if greater than sint
        #[def(arch.reg[d] = arch.reg[s] > imm)]
        SSgtI { d: TReg<bool>, s: TReg<sint>, imm: sint },
        /// Set if greater than frac
        #[def(arch.reg[d] = arch.reg[s] > imm)]
        FSgtI { d: TReg<bool>, s: TReg<frac>, imm: frac },

        /// Set if less than or equal to uint
        #[def(arch.reg[d] = arch.reg[s] <= imm)]
        USleI { d: TReg<bool>, s: TReg<uint>, imm: uint },
        /// Set if less than or equal to sint
        #[def(arch.reg[d] = arch.reg[s] <= imm)]
        SSleI { d: TReg<bool>, s: TReg<sint>, imm: sint },
        /// Set if less than or equal to frac
        #[def(arch.reg[d] = arch.reg[s] <= imm)]
        FSleI { d: TReg<bool>, s: TReg<frac>, imm: frac },

        /// Set if greater than or equal to uint
        #[def(arch.reg[d] = arch.reg[s] >= imm)]
        USgeI { d: TReg<bool>, s: TReg<uint>, imm: uint },
        /// Set if greater than or equal to sint
        #[def(arch.reg[d] = arch.reg[s] >= imm)]
        SSgeI { d: TReg<bool>, s: TReg<sint>, imm: sint },
        /// Set if greater than or equal to frac
        #[def(arch.reg[d] = arch.reg[s] >= imm)]
        FSgeI { d: TReg<bool>, s: TReg<frac>, imm: frac },

        /// Set if equal to uint
        #[def(arch.reg[d] = arch.reg[s] == imm)]
        USeqI { d: TReg<bool>, s: TReg<uint>, imm: uint },
        /// Set if equal to sint
        #[def(arch.reg[d] = arch.reg[s] == imm)]
        SSeqI { d: TReg<bool>, s: TReg<sint>, imm: sint },
        /// Set if equal to frac
        #[def(arch.reg[d] = arch.reg[s].eq(&imm))]
        FSeqI { d: TReg<bool>, s: TReg<frac>, imm: frac },

        /// Set if not equal to uint
        #[def(arch.reg[d] = arch.reg[s] != imm)]
        USneI { d: TReg<bool>, s: TReg<uint>, imm: uint },
        /// Set if not equal to sint
        #[def(arch.reg[d] = arch.reg[s] != imm)]
        SSneI { d: TReg<bool>, s: TReg<sint>, imm: sint },
        /// Set if not equal to frac
        #[def(arch.reg[d] = arch.reg[s].ne(&imm))]
        FSneI { d: TReg<bool>, s: TReg<frac>, imm: frac },

        // ---------------------------------------------
        // Data transfer
        // ---------------------------------------------

        /// Load word
        #[def(arch.reg[s] = arch.reg[t].checked_add_signed(imm).and_then(|pos| read_word(&arch.ram, pos)).ok_or(Failure::OutOfBounds)?)]
        Lw { s: Reg, t: Reg, imm: isize },
        /// Store word
        #[def(arch.reg[t].checked_add_signed(imm).and_then(|pos| write_word(&mut arch.ram, pos, arch.reg[s])).ok_or(Failure::OutOfBounds)?)]
        Sw { s: Reg, t: Reg, imm: isize },
        /// Load immediate
        #[def(arch.reg[s] = imm)]
        Li { s: Reg, imm: usize },
        /// Copy from register to register
        #[def(arch.reg[s] = arch.reg[t])]
        Mov { s: Reg, t: Reg },

        // ---------------------------------------------
        // Jump
        // ---------------------------------------------

        /// Jump
        #[def(*pc = addr)]
        J { addr: usize },

        /// Jump to register
        #[def(*pc = arch.reg[s])]
        Jr { s: Reg },

        /// Jump and link
        #[def({
            arch.link(*pc).expect("stack overflow"); // TODO: make this a failure
            *pc = addr;
        })]
        Jal { addr: usize },

        /// Branch if equal
        #[def(if s == t { *pc = addr })]
        Beq { s: Reg, t: Reg, addr: usize },

        /// Branch if not equal
        #[def(if s != t { *pc = addr })]
        Bne { s: Reg, t: Reg, addr: usize },

        // ---------------------------------------------
        // System
        // ---------------------------------------------

        /// Output string
        #[def(print!("{}", str::from_utf8(arch.ram.get(arch.reg.a[0]..=arch.reg.a[1]).ok_or(Failure::OutOfBounds)?)?))]
        PrintStr,

        /// Read input from user
        #[def(todo!())]
        ReadStr,
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    lines: Box<[Instruction]>,
    pc: usize,
}

impl Program {
    pub fn new<I>(lines: I) -> Self
    where
        I: IntoIterator<Item = Instruction>,
    {
        Self {
            lines: Vec::from_iter(lines).into_boxed_slice(),
            pc: 0,
        }
    }

    pub fn run(&mut self, arch: &mut VirtualArch) -> Result<(), Failure> {
        loop {
            if let Some(&line) = self.lines.get(self.pc) {
                line.step(&mut self.pc, arch)?;
                self.pc = self.pc.checked_add(1).ok_or(Failure::OutOfBounds)?; // TBD: should this be its own error?
            } else if self.pc == self.lines.len() {
                break Ok(());
            } else {
                break Err(Failure::OutOfBounds); // TBD: should this be its own error?
            }
        }
    }
}
