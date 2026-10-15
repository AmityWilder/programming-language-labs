use crate::types::fsize;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RegCat {
    #[default]
    Zero,
    V(usize),
    A(usize),
    T(usize),
    S(usize),
    Gp,
    Sp,
    Ra,
}

impl From<Reg> for RegCat {
    fn from(value: Reg) -> Self {
        match value {
            Reg::Zero => Self::Zero,
            Reg::V0 => Self::V(0),
            Reg::V1 => Self::V(1),
            Reg::A0 => Self::A(0),
            Reg::A1 => Self::A(1),
            Reg::A2 => Self::A(2),
            Reg::A3 => Self::A(3),
            Reg::T0 => Self::T(0),
            Reg::T1 => Self::T(1),
            Reg::T2 => Self::T(2),
            Reg::T3 => Self::T(3),
            Reg::T4 => Self::T(4),
            Reg::T5 => Self::T(5),
            Reg::T6 => Self::T(6),
            Reg::T7 => Self::T(7),
            Reg::T8 => Self::T(8),
            Reg::T9 => Self::T(9),
            Reg::S0 => Self::S(0),
            Reg::S1 => Self::S(1),
            Reg::S2 => Self::S(2),
            Reg::S3 => Self::S(3),
            Reg::S4 => Self::S(4),
            Reg::S5 => Self::S(5),
            Reg::S6 => Self::S(6),
            Reg::S7 => Self::S(7),
            Reg::Gp => Self::Gp,
            Reg::Sp => Self::Sp,
            Reg::Ra => Self::Ra,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Reg {
    #[default]
    Zero,
    V0,
    V1,
    A0,
    A1,
    A2,
    A3,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    T7,
    T8,
    T9,
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    Gp,
    Sp,
    Ra,
}

impl TryFrom<RegCat> for Reg {
    type Error = ();

    fn try_from(value: RegCat) -> Result<Self, Self::Error> {
        match value {
            RegCat::Zero => Ok(Self::Zero),
            RegCat::V(0) => Ok(Self::V0),
            RegCat::V(1) => Ok(Self::V1),
            RegCat::A(0) => Ok(Self::A0),
            RegCat::A(1) => Ok(Self::A1),
            RegCat::A(2) => Ok(Self::A2),
            RegCat::A(3) => Ok(Self::A3),
            RegCat::T(0) => Ok(Self::T0),
            RegCat::T(1) => Ok(Self::T1),
            RegCat::T(2) => Ok(Self::T2),
            RegCat::T(3) => Ok(Self::T3),
            RegCat::T(4) => Ok(Self::T4),
            RegCat::T(5) => Ok(Self::T5),
            RegCat::T(6) => Ok(Self::T6),
            RegCat::T(7) => Ok(Self::T7),
            RegCat::T(8) => Ok(Self::T8),
            RegCat::T(9) => Ok(Self::T9),
            RegCat::S(0) => Ok(Self::S0),
            RegCat::S(1) => Ok(Self::S1),
            RegCat::S(2) => Ok(Self::S2),
            RegCat::S(3) => Ok(Self::S3),
            RegCat::S(4) => Ok(Self::S4),
            RegCat::S(5) => Ok(Self::S5),
            RegCat::S(6) => Ok(Self::S6),
            RegCat::S(7) => Ok(Self::S7),
            RegCat::Gp => Ok(Self::Gp),
            RegCat::Sp => Ok(Self::Sp),
            RegCat::Ra => Ok(Self::Ra),

            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TReg<T: ?Sized>(Reg, PhantomData<T>);

impl<T: ?Sized> PartialEq for TReg<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Eq for TReg<T> {}

impl<T: ?Sized> std::hash::Hash for TReg<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> TReg<T> {
    pub const fn new(reg: Reg) -> Self {
        Self(reg, PhantomData)
    }
}

impl<T> From<Reg> for TReg<T> {
    fn from(value: Reg) -> Self {
        Self::new(value)
    }
}

impl<T> From<TReg<T>> for Reg {
    fn from(value: TReg<T>) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Registers {
    pub zero: usize,
    pub v: [usize; 2],
    pub a: [usize; 4],
    pub t: [usize; 10],
    pub s: [usize; 8],
    pub gp: usize,
    pub sp: usize,
    pub ra: usize,
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    pub const fn new() -> Self {
        Self {
            zero: 0,
            v: [0; _],
            a: [0; _],
            t: [0; _],
            s: [0; _],
            gp: 0,
            sp: 0,
            ra: 0,
        }
    }
}

impl std::ops::Index<Reg> for Registers {
    type Output = usize;

    fn index(&self, index: Reg) -> &Self::Output {
        match index {
            Reg::Zero => &self.zero,
            Reg::V0 => &self.v[0],
            Reg::V1 => &self.v[1],
            Reg::A0 => &self.a[0],
            Reg::A1 => &self.a[1],
            Reg::A2 => &self.a[2],
            Reg::A3 => &self.a[3],
            Reg::T0 => &self.t[0],
            Reg::T1 => &self.t[1],
            Reg::T2 => &self.t[2],
            Reg::T3 => &self.t[3],
            Reg::T4 => &self.t[4],
            Reg::T5 => &self.t[5],
            Reg::T6 => &self.t[6],
            Reg::T7 => &self.t[7],
            Reg::T8 => &self.t[8],
            Reg::T9 => &self.t[9],
            Reg::S0 => &self.s[0],
            Reg::S1 => &self.s[1],
            Reg::S2 => &self.s[2],
            Reg::S3 => &self.s[3],
            Reg::S4 => &self.s[4],
            Reg::S5 => &self.s[5],
            Reg::S6 => &self.s[6],
            Reg::S7 => &self.s[7],
            Reg::Gp => &self.gp,
            Reg::Sp => &self.sp,
            Reg::Ra => &self.ra,
        }
    }
}

impl std::ops::IndexMut<Reg> for Registers {
    fn index_mut(&mut self, index: Reg) -> &mut Self::Output {
        match index {
            Reg::Zero => &mut self.zero,
            Reg::V0 => &mut self.v[0],
            Reg::V1 => &mut self.v[1],
            Reg::A0 => &mut self.a[0],
            Reg::A1 => &mut self.a[1],
            Reg::A2 => &mut self.a[2],
            Reg::A3 => &mut self.a[3],
            Reg::T0 => &mut self.t[0],
            Reg::T1 => &mut self.t[1],
            Reg::T2 => &mut self.t[2],
            Reg::T3 => &mut self.t[3],
            Reg::T4 => &mut self.t[4],
            Reg::T5 => &mut self.t[5],
            Reg::T6 => &mut self.t[6],
            Reg::T7 => &mut self.t[7],
            Reg::T8 => &mut self.t[8],
            Reg::T9 => &mut self.t[9],
            Reg::S0 => &mut self.s[0],
            Reg::S1 => &mut self.s[1],
            Reg::S2 => &mut self.s[2],
            Reg::S3 => &mut self.s[3],
            Reg::S4 => &mut self.s[4],
            Reg::S5 => &mut self.s[5],
            Reg::S6 => &mut self.s[6],
            Reg::S7 => &mut self.s[7],
            Reg::Gp => &mut self.gp,
            Reg::Sp => &mut self.sp,
            Reg::Ra => &mut self.ra,
        }
    }
}

impl<T: RegisterAs> std::ops::Index<TReg<T>> for Registers {
    type Output = T;

    #[inline]
    fn index(&self, index: TReg<T>) -> &Self::Output {
        T::reg_as(self.index(index.0))
    }
}

impl<T: RegisterAsMut> std::ops::IndexMut<TReg<T>> for Registers {
    #[inline]
    fn index_mut(&mut self, index: TReg<T>) -> &mut Self::Output {
        T::reg_as_mut(self.index_mut(index.0))
    }
}

pub trait RegisterAs {
    fn reg_as(data: &usize) -> &Self;
}
pub trait RegisterAsMut: RegisterAs {
    fn reg_as_mut(data: &mut usize) -> &mut Self;
}

impl RegisterAs for usize {
    #[inline]
    fn reg_as(data: &usize) -> &usize {
        data
    }
}
impl RegisterAsMut for usize {
    #[inline]
    fn reg_as_mut(data: &mut usize) -> &mut usize {
        data
    }
}

impl RegisterAs for isize {
    #[inline]
    fn reg_as(data: &usize) -> &isize {
        // SAFETY: pointer came from a reference
        unsafe { (&raw const *data).cast::<isize>().as_ref_unchecked() }
    }
}
impl RegisterAsMut for isize {
    #[inline]
    fn reg_as_mut(data: &mut usize) -> &mut isize {
        // SAFETY: pointer came from a mutable reference
        unsafe { (&raw mut *data).cast::<isize>().as_mut_unchecked() }
    }
}

impl RegisterAs for u8 {
    #[inline]
    fn reg_as(data: &usize) -> &u8 {
        // SAFETY: pointer came from a reference and u8 has a size and alignment of at most usize
        unsafe { (&raw const *data).cast::<u8>().as_ref_unchecked() }
    }
}
impl RegisterAsMut for u8 {
    #[inline]
    fn reg_as_mut(data: &mut usize) -> &mut u8 {
        // SAFETY: pointer came from a mutable reference and u8 has a size and alignment of at most usize
        unsafe { (&raw mut *data).cast::<u8>().as_mut_unchecked() }
    }
}

impl RegisterAs for bool {
    #[inline]
    fn reg_as(data: &usize) -> &bool {
        // SAFETY: pointer came from a reference and bool has a size and alignment of at most usize
        unsafe { (&raw const *data).cast::<bool>().as_ref_unchecked() }
    }
}
impl RegisterAsMut for bool {
    #[inline]
    fn reg_as_mut(data: &mut usize) -> &mut bool {
        // SAFETY: pointer came from a mutable reference and bool has a size and alignment of at most usize
        unsafe { (&raw mut *data).cast::<bool>().as_mut_unchecked() }
    }
}

impl RegisterAs for fsize {
    #[inline]
    fn reg_as(data: &usize) -> &fsize {
        // SAFETY: pointer came from a reference
        unsafe { (&raw const *data).cast::<fsize>().as_ref_unchecked() }
    }
}
impl RegisterAsMut for fsize {
    #[inline]
    fn reg_as_mut(data: &mut usize) -> &mut fsize {
        // SAFETY: pointer came from a mutable reference
        unsafe { (&raw mut *data).cast::<fsize>().as_mut_unchecked() }
    }
}
