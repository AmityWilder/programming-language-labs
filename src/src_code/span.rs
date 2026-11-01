use std::{
    marker::Destruct,
    ops::{Deref, DerefMut, DerefPure},
    range::Range,
};

/// Guaranteed to be at most pointer-width
#[expect(non_camel_case_types)]
pub type uspan = cfg_select! {
    target_pointer_width = "16" => usize,
    _ => u32,
};

pub const fn uspan_to_usize(n: uspan) -> usize {
    #![expect(
        clippy::as_conversions,
        reason = "`uspan` is guaranteed to be at most pointer-width"
    )]
    cfg_select! {
        // n is already defined as usize
        target_pointer_width = "16" => n,

        _ => {
            const {
                assert!(uspan::BITS <= usize::BITS);
            }
            n as usize
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct RelSpan<T = uspan>(pub Range<T>);

impl<T> From<RelSpan<T>> for Range<T> {
    fn from(value: RelSpan<T>) -> Self {
        value.0
    }
}

impl From<RelSpan<uspan>> for Range<usize> {
    fn from(span: RelSpan<uspan>) -> Self {
        Range {
            start: uspan_to_usize(span.0.start),
            end: uspan_to_usize(span.0.end),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for RelSpan<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Deref for RelSpan<T> {
    type Target = Range<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RelSpan<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// SAFETY: RelSpan is a transparent wrapper for its Target
unsafe impl<T> DerefPure for RelSpan<T> {}

impl<T: Ord> PartialOrd for RelSpan<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};
        match (
            self.0.start.cmp(&other.0.start),
            self.0.end.cmp(&other.0.end),
        ) {
            (Less, Less) => Some(Less),
            (Equal, Equal) => Some(Equal),
            (Greater, Greater) => Some(Greater),

            _ => None,
        }
    }
}

impl<T> RelSpan<T> {
    pub const fn from_ref(value: &Range<T>) -> &Self {
        // SAFETY: RelSpan<T> is a transparent wrapper for Range<T>
        unsafe { std::mem::transmute(value) }
    }

    pub const fn from_mut(value: &mut Range<T>) -> &mut Self {
        // SAFETY: RelSpan<T> is a transparent wrapper for Range<T>
        unsafe { std::mem::transmute(value) }
    }

    /// Returns `true` if every element of `other` is an element of `self`.
    pub const fn contains(self, other: Self) -> bool
    where
        T: [const] Ord + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "ranges should be ordered");
        debug_assert!(other.0.start <= other.0.end, "ranges should be ordered");
        self.0.start <= other.0.start && self.0.end <= other.0.end
    }

    /// Returns `true` if `self` and `other` do not intersect.
    pub const fn is_disjoint(self, other: Self) -> bool
    where
        T: [const] Ord + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "ranges should be ordered");
        debug_assert!(other.0.start <= other.0.end, "ranges should be ordered");
        self.0.start >= other.0.end || self.0.end <= other.0.start
    }

    /// Returns `true` if `self` and `other` have a non-zero intersection.
    pub const fn overlaps(self, other: Self) -> bool
    where
        T: [const] Ord + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "ranges should be ordered");
        debug_assert!(other.0.start <= other.0.end, "ranges should be ordered");
        self.0.start < other.0.end && self.0.end > other.0.start
    }

    /// `self` ends exactly where `next` begins.
    pub const fn is_joint(self, next: Self) -> bool
    where
        T: [const] Ord + Copy,
    {
        self.0.end == next.0.start
    }

    /// Finds the subset of elements which are in both `self` and `other`.
    ///
    /// Returns [`None`] if they do not intersect.
    /// Returns `Some(x..x)` for some integer `x` if they share a boundary.
    pub const fn intersection(self, other: Self) -> Option<Self>
    where
        T: [const] Ord + [const] Destruct + Copy,
    {
        let start = self.0.start.max(other.0.start);
        let end = self.0.end.min(other.0.end);
        if start <= end {
            Some(Self(Range { start, end }))
        } else {
            None
        }
    }

    /// Finds the supset of elements which are in either `self` or `other`.
    ///
    /// Returns [`None`] if they do not share a boundary.
    pub const fn union(self, other: Self) -> Option<Self>
    where
        T: [const] Ord + [const] Destruct + Copy,
    {
        match ((self.0.start, self.0.end), (other.0.start, other.0.end)) {
            ((start, border1), (border2, end)) | ((border2, end), (start, border1))
                if border1 == border2 =>
            {
                return Some(Self(Range { start, end }));
            }

            _ => None,
        }
    }

    /// Finds the union of `self` and `other`, including all elements between even if not contained in either.
    pub const fn super_union(self, other: Self) -> Self
    where
        T: [const] Ord + [const] Destruct + Copy,
    {
        let start = self.0.start.min(other.0.start);
        let end = self.0.end.max(other.0.end);
        Self(Range { start, end })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub file: uspan,
    pub rel: RelSpan<uspan>,
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.file.cmp(&other.file) {
            std::cmp::Ordering::Equal => self.rel.partial_cmp(&other.rel),
            ord => Some(ord),
        }
    }
}

impl Span {
    pub const fn contains(self, other: Self) -> bool {
        self.file == other.file && self.rel.contains(other.rel)
    }

    pub const fn is_disjoint(self, other: Self) -> bool {
        self.file == other.file && self.rel.is_disjoint(other.rel)
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.file == other.file && self.rel.overlaps(other.rel)
    }

    pub const fn is_joint(self, next: Self) -> bool {
        self.file == next.file && self.rel.is_joint(next.rel)
    }

    pub const fn intersection(self, other: Self) -> Option<Self> {
        let file = self.file;
        if file == other.file {
            if let Some(rel) = self.rel.intersection(other.rel) {
                return Some(Self { file, rel });
            }
        }
        None
    }

    pub const fn union(self, other: Self) -> Option<Self> {
        let file = self.file;
        if file == other.file {
            if let Some(rel) = self.rel.union(other.rel) {
                return Some(Self { file, rel });
            }
        }
        None
    }

    pub const fn super_union(self, other: Self) -> Option<Self> {
        let file = self.file;
        if file == other.file {
            Some(Self {
                file,
                rel: self.rel.super_union(other.rel),
            })
        } else {
            None
        }
    }
}
