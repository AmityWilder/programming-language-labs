use crate::types::{fail, frac, none, sint, text, uint};
use std::range::Range;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Memory(Box<[u8]>);

impl Memory {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(vec![0; capacity].into_boxed_slice())
    }

    pub fn get(&self, range: Range<usize>) -> Option<&[u8]> {
        self.0.get(range)
    }

    pub fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [u8]> {
        self.0.get_mut(range)
    }

    /// # Safety
    /// `range` must be the range of a (type-specific) valid bytestring representation of `T`
    pub unsafe fn get_as<T: ?Sized + Primitive>(&self, range: Range<usize>) -> Option<&T> {
        // SAFETY: caller must uphold safety contract
        self.get(range).map(|data| unsafe { T::ref_as(data) })
    }

    /// # Safety
    /// `range` must be the range of a (type-specific) valid bytestring representation of `T`
    pub unsafe fn get_mut_as<T: ?Sized + Primitive>(
        &mut self,
        range: Range<usize>,
    ) -> Option<&mut T> {
        // SAFETY: caller must uphold safety contract
        self.get_mut(range).map(|data| unsafe { T::mut_as(data) })
    }
}

pub trait Primitive {
    /// # Safety
    /// `data` must point to a (type-specific) valid bytestring representation of `Self`
    unsafe fn ref_as(data: &[u8]) -> &Self;

    /// # Safety
    /// `data` must point to a (type-specific) valid bytestring representation of `Self`
    unsafe fn mut_as(data: &mut [u8]) -> &mut Self;
}

/// Marker for trivial [`Primitive`] impl
pub trait SizedPrimitive: Sized {}

impl<T: SizedPrimitive> Primitive for T {
    unsafe fn ref_as(data: &[u8]) -> &Self {
        let ptr = (&raw const *data).cast::<T>();
        debug_assert!(ptr.is_aligned(), "pointer must be aligned");
        // SAFETY: caller must uphold safety contract
        unsafe { &*ptr }
    }

    unsafe fn mut_as(data: &mut [u8]) -> &mut Self {
        let ptr = (&raw mut *data).cast::<T>();
        debug_assert!(ptr.is_aligned(), "pointer must be aligned");
        // SAFETY: caller must uphold safety contract
        unsafe { &mut *ptr }
    }
}

impl SizedPrimitive for none {}
impl SizedPrimitive for fail {}
impl SizedPrimitive for bool {}
impl SizedPrimitive for uint {}
impl SizedPrimitive for sint {}
impl SizedPrimitive for frac {}
impl SizedPrimitive for char {}
impl SizedPrimitive for text {}

impl Primitive for [u8] {
    unsafe fn ref_as(data: &[u8]) -> &Self {
        data
    }

    unsafe fn mut_as(data: &mut [u8]) -> &mut Self {
        data
    }
}

impl Primitive for str {
    unsafe fn ref_as(data: &[u8]) -> &Self {
        // SAFETY: caller must uphold safety contract
        unsafe { str::from_utf8_unchecked(data) }
    }

    unsafe fn mut_as(data: &mut [u8]) -> &mut Self {
        // SAFETY: caller must uphold safety contract
        unsafe { str::from_utf8_unchecked_mut(data) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment {
    global: Memory,
    stack: Memory,
}

impl Environment {
    pub fn with_capacity(g_cap: usize, s_cap: usize) -> Self {
        Self {
            global: Memory::with_capacity(g_cap),
            stack: Memory::with_capacity(s_cap),
        }
    }

    pub fn global(&self) -> &Memory {
        &self.global
    }
    pub fn global_mut(&mut self) -> &mut Memory {
        &mut self.global
    }

    pub fn stack(&self) -> &Memory {
        &self.stack
    }
    pub fn stack_mut(&mut self) -> &mut Memory {
        &mut self.stack
    }
}
