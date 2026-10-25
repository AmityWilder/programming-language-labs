use std::ops::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Ternary {
    False = -1,
    #[default]
    Unknown = 0,
    True = 1,
}

pub use Ternary::{False, True, Unknown};

impl From<bool> for Ternary {
    fn from(value: bool) -> Self {
        if value { True } else { False }
    }
}

impl Ternary {
    #[inline]
    pub const fn is_true(&self) -> bool {
        matches!(self, True)
    }

    #[inline]
    pub const fn is_false(&self) -> bool {
        matches!(self, False)
    }

    #[inline]
    pub const fn is_maybe(&self) -> bool {
        matches!(self, Unknown)
    }
}

impl Not for Ternary {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        match self {
            False => True,
            Unknown => Unknown,
            True => False,
        }
    }
}

impl BitAnd for Ternary {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (False, _) | (_, False) => False,
            (Unknown, _) | (_, Unknown) => Unknown,
            (True, True) => True,
        }
    }
}

impl BitOr for Ternary {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (False, False) => False,
            (Unknown, _) | (_, Unknown) => Unknown,
            (True, _) | (_, True) => True,
        }
    }
}

impl BitAndAssign for Ternary {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.bitand(rhs)
    }
}

impl BitOrAssign for Ternary {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.bitand(rhs)
    }
}

pub trait TernaryIterExt: Iterator {
    fn ternary_any<F>(&mut self, f: F) -> Ternary
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Ternary,
    {
        let mut iter = self.map(f);
        if let Some(mut acc) = iter.next() {
            if !acc.is_true() {
                while let Some(x) = iter.next() {
                    acc |= x;
                    if acc.is_true() {
                        break;
                    }
                }
            }
            acc
        } else {
            Unknown
        }
    }

    fn ternary_all<F>(&mut self, f: F) -> Ternary
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Ternary,
    {
        let mut iter = self.map(f);
        if let Some(mut acc) = iter.next() {
            if !acc.is_false() {
                while let Some(x) = iter.next() {
                    acc &= x;
                    if acc.is_false() {
                        break;
                    }
                }
            }
            acc
        } else {
            Unknown
        }
    }
}

impl<I: Iterator> TernaryIterExt for I {}
