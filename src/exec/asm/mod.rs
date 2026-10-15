//! Assembly-style runtime for Batscript

use crate::{
    exec::error::Failure,
    types::{frac, sint, uint},
};
use arch::VirtualArch;
use reg::{Reg, TReg};

pub mod arch;
pub mod reg;

/// Lemma 1: `2^n` is a multiple of `2^h` for every integer `h` where `0<=h<=n`
///
/// Proof of Lemma 1:
/// Let `P(n)` be the statement "`2^n` is a multiple of `2^h` for every integer `h` where `0<=h<=n`".
/// Base case: `P(0)`: `2^0 = 1`. 1 is a multiple of 1.
/// Induction step: Assume `P(0), ..., P(k)` are all true for some integer `k` such that `k>=0`.
/// We will prove that `P(k+1)` is true.
/// Observe: `2^(k+1) = 2 * 2^k`.
///
/// Theorem 1: Let `n` and `k` be non-negative integers. If `k<=n`, then any bit rotation of `n` bits
/// on a bitstring with `k` bits can be performed with `k` bits.
///
/// Proof:
///
trait BitRotate<By> {
    fn rotr(self, bits: By) -> Self;
    fn rotl(self, bits: By) -> Self;
}

impl BitRotate<usize> for usize {
    #[inline]
    fn rotr(self, bits: usize) -> Self {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the truncation is desired"
        )]
        self.rotate_right(bits as u32)
    }

    #[inline]
    fn rotl(self, bits: usize) -> Self {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the truncation is desired"
        )]
        self.rotate_left(bits as u32)
    }
}

impl BitRotate<isize> for usize {
    #[inline]
    fn rotr(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.rotr(bits)
        } else {
            self.rotl(bits.unsigned_abs())
        }
    }

    #[inline]
    fn rotl(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.rotl(bits)
        } else {
            self.rotr(bits.unsigned_abs())
        }
    }
}

impl BitRotate<usize> for isize {
    #[inline]
    fn rotr(self, bits: usize) -> Self {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the truncation is desired"
        )]
        self.rotate_right(bits as u32)
    }

    #[inline]
    fn rotl(self, bits: usize) -> Self {
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "the truncation is desired"
        )]
        self.rotate_left(bits as u32)
    }
}

impl BitRotate<isize> for isize {
    #[inline]
    fn rotr(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.rotr(bits)
        } else {
            self.rotl(bits.unsigned_abs())
        }
    }

    #[inline]
    fn rotl(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.rotl(bits)
        } else {
            self.rotr(bits.unsigned_abs())
        }
    }
}

macro_rules! instructions {
    (
        arch: $arch:ident;
        pc: $pc:ident;

        $(#[$meta:meta])*
        $vis:vis enum $Enum:ident {$(
            $(#[doc = $doc:expr])*
            #[def($def:expr)]
            $Variant:ident {$(
                $(#[$fmeta:meta])*
                $field:ident: $Type:ty
            ),* $(,)?}
        ),* $(,)?}
    ) => {
        $(#[$meta])*
        $vis enum $Enum {$(
            $(#[doc = $doc])*
            #[allow(dead_code, reason = "under construction")]
            $Variant {$(
                $(#[$fmeta])*
                $field: $Type
            ),*}
        ),*}

        impl $Enum {
            fn step(self, $pc: &mut usize, $arch: &mut VirtualArch) -> Result<(), Failure> {
                #[allow(clippy::arithmetic_side_effects)]
                match self {
                    $(Self::$Variant {$(#[allow(unused_variables)] $field),*} => $def,)*
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
        /// Add floats
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

        // ---------------------------------------------
        // Immediate
        // ---------------------------------------------

        /// Add uints
        #[def(arch.reg[d] = arch.reg[s] + imm)]
        UAddI { d: TReg<uint>, s: TReg<uint>, imm: uint },
        /// Add sints
        #[def(arch.reg[d] = arch.reg[s] + imm)]
        SAddI { d: TReg<sint>, s: TReg<sint>, imm: sint },
        /// Add fracs
        #[def(arch.reg[d] = arch.reg[s] + imm)]
        FAddI { d: TReg<frac>, s: TReg<frac>, imm: frac },

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
            arch.link(*pc);
            *pc = addr;
        })]
        Jal { addr: usize },

        /// Branch if equal
        #[def(todo!())]
        Beq { s: Reg, t: Reg, addr: usize },

        /// Branch if not equal
        #[def(todo!())]
        Bne { s: Reg, t: Reg, addr: usize },
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
            } else if self.pc == self.lines.len() {
                break Ok(());
            } else {
                todo!("error: pc out of range");
            }
        }
    }
}
