//! Special operations

#![expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "truncation is desired"
)]

/// If `bits` exceeds the size that fits in a [`u32`], it DEFINITELY exceeds the bounds of self
/// (we don't have a `u4294967295` type)
pub trait BitShift<By> {
    fn bshl(self, bits: By) -> Self;
    fn bshr(self, bits: By) -> Self;
}

impl BitShift<usize> for usize {
    #[inline]
    fn bshl(self, bits: usize) -> Self {
        self.unbounded_shl(bits as u32)
    }

    #[inline]
    fn bshr(self, bits: usize) -> Self {
        self.unbounded_shr(bits as u32)
    }
}

impl BitShift<isize> for usize {
    #[inline]
    fn bshl(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.bshl(bits)
        } else {
            self.bshr(bits.unsigned_abs())
        }
    }

    #[inline]
    fn bshr(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.bshr(bits)
        } else {
            self.bshl(bits.unsigned_abs())
        }
    }
}

impl BitShift<usize> for isize {
    #[inline]
    fn bshl(self, bits: usize) -> Self {
        self.unbounded_shl(bits as u32)
    }

    #[inline]
    fn bshr(self, bits: usize) -> Self {
        self.unbounded_shr(bits as u32)
    }
}

impl BitShift<isize> for isize {
    #[inline]
    fn bshl(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.bshl(bits)
        } else {
            self.bshr(bits.unsigned_abs())
        }
    }

    #[inline]
    fn bshr(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.bshr(bits)
        } else {
            self.bshl(bits.unsigned_abs())
        }
    }
}

impl BitShift<usize> for u8 {
    #[inline]
    fn bshl(self, bits: usize) -> Self {
        self.unbounded_shl(bits as u32)
    }

    #[inline]
    fn bshr(self, bits: usize) -> Self {
        self.unbounded_shr(bits as u32)
    }
}

impl BitShift<isize> for u8 {
    #[inline]
    fn bshl(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.bshl(bits)
        } else {
            self.bshr(bits.unsigned_abs())
        }
    }

    #[inline]
    fn bshr(self, bits: isize) -> Self {
        if let Ok(bits) = usize::try_from(bits) {
            self.bshr(bits)
        } else {
            self.bshl(bits.unsigned_abs())
        }
    }
}

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
pub trait BitRotate<By> {
    fn rotr(self, bits: By) -> Self;
    fn rotl(self, bits: By) -> Self;
}

impl BitRotate<usize> for usize {
    #[inline]
    fn rotr(self, bits: usize) -> Self {
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

impl BitRotate<usize> for u8 {
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

impl BitRotate<isize> for u8 {
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
