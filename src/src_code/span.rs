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

#[cfg_attr(not(debug_assertions), inline)]
pub const fn uspan_to_usize(n: uspan) -> usize {
    cfg_select! {
        // n is already defined as usize
        target_pointer_width = "16" => n,

        _ => {
            #[cfg(debug_assertions)]
            const {
                assert!(
                    uspan::BITS <= usize::BITS,
                    "`uspan` should be at most pointer-width"
                );
            }
            #[expect(
                clippy::as_conversions,
                reason = "`uspan` is guaranteed to be at most pointer-width"
            )]
            (n as usize)
        }
    }
}

/// Relies on the assumption `n` is from a non-oversize file
///
/// # Panics
/// This method may panic in debug if `n` does not actually fit in [`uspan`]
#[cfg_attr(not(debug_assertions), inline)]
pub const fn usize_to_uspan(n: usize) -> uspan {
    debug_assert!(uspan::try_from(n).is_ok(), "file should not be oversized");
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "file should not be oversized"
    )]
    (n as uspan)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct RelSpan<T = uspan>(pub Range<T>);

impl<T> From<Range<T>> for RelSpan<T> {
    fn from(value: Range<T>) -> Self {
        Self(value)
    }
}

impl<T> From<std::ops::Range<T>> for RelSpan<T> {
    fn from(value: std::ops::Range<T>) -> Self {
        Self(value.into())
    }
}

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

impl<T: PartialEq + Copy + std::fmt::Debug> std::fmt::Debug for RelSpan<T> {
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

    #[track_caller]
    pub const fn len(self) -> <T as std::ops::Sub>::Output
    where
        T: [const] PartialOrd + [const] std::ops::Sub + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "ranges should be ordered");
        #[expect(clippy::arithmetic_side_effects, reason = "ranges should be ordered")]
        (self.0.end - self.0.start)
    }

    #[track_caller]
    pub const fn is_empty(self) -> bool
    where
        T: [const] PartialEq + Copy,
    {
        self.0.start == self.0.end
    }

    /// Returns `true` if every element of `other` is an element of `self`.
    #[track_caller]
    pub const fn contains(self, other: Self) -> bool
    where
        T: [const] PartialOrd + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "self: ranges should be ordered");
        debug_assert!(
            other.0.start <= other.0.end,
            "other: ranges should be ordered"
        );
        self.0.start <= other.0.start && other.0.end <= self.0.end
    }

    /// Returns `true` if `self` and `other` do not intersect.
    #[track_caller]
    pub const fn is_disjoint(self, other: Self) -> bool
    where
        T: [const] PartialOrd + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "self: ranges should be ordered");
        debug_assert!(
            other.0.start <= other.0.end,
            "other: ranges should be ordered"
        );
        self.0.start >= other.0.end || self.0.end <= other.0.start
    }

    /// Returns `true` if `self` and `other` have a non-zero intersection.
    #[track_caller]
    pub const fn overlaps(self, other: Self) -> bool
    where
        T: [const] PartialOrd + Copy,
    {
        debug_assert!(self.0.start <= self.0.end, "self: ranges should be ordered");
        debug_assert!(
            other.0.start <= other.0.end,
            "other: ranges should be ordered"
        );
        self.0.start < other.0.end && self.0.end > other.0.start
    }

    /// `self` ends exactly where `next` begins.
    pub const fn is_joint(self, next: Self) -> bool
    where
        T: [const] PartialEq + Copy,
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
        T: [const] PartialEq + [const] Destruct + Copy,
    {
        match ((self.0.start, self.0.end), (other.0.start, other.0.end)) {
            ((start, border1), (border2, end)) | ((border2, end), (start, border1))
                if border1 == border2 =>
            {
                Some(Self(Range { start, end }))
            }

            _ => None,
        }
    }

    /// Finds the union of `self` and `other`, including all elements between even if not contained in either.
    #[track_caller]
    pub const fn super_union(self, other: Self) -> Self
    where
        T: [const] Ord + [const] Destruct + Copy,
    {
        debug_assert!(
            self.0.start <= other.0.start,
            "should not union out-of-order spans"
        );
        let start = self.0.start;
        let end = self.0.end.max(other.0.end);
        Self(Range { start, end })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(uspan);

impl std::fmt::Debug for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "file[{}]", self.0)
    }
}

impl FileId {
    /// Caller must ensure this is a legitimate file index.
    pub(super) const fn new(i: uspan) -> Self {
        Self(i)
    }

    pub(super) const fn into_usize(self) -> usize {
        uspan_to_usize(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub rel: RelSpan<uspan>,
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { file, rel } = self;
        write!(f, "{file:?}:{rel:?}")
    }
}

impl PartialOrd for Span {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.file.partial_cmp(&other.file) {
            Some(std::cmp::Ordering::Equal) => self.rel.partial_cmp(&other.rel),
            ord => ord,
        }
    }
}

impl Span {
    pub const fn contains(self, other: Self) -> bool {
        self.file.0 == other.file.0 && self.rel.contains(other.rel)
    }

    pub const fn is_disjoint(self, other: Self) -> bool {
        self.file.0 == other.file.0 && self.rel.is_disjoint(other.rel)
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.file.0 == other.file.0 && self.rel.overlaps(other.rel)
    }

    pub const fn is_joint(self, next: Self) -> bool {
        self.file.0 == next.file.0 && self.rel.is_joint(next.rel)
    }

    pub const fn intersection(self, other: Self) -> Option<Self> {
        let file = self.file;
        if file.0 == other.file.0
            && let Some(rel) = self.rel.intersection(other.rel)
        {
            Some(Self { file, rel })
        } else {
            None
        }
    }

    pub const fn union(self, other: Self) -> Option<Self> {
        let file = self.file;
        if file.0 == other.file.0
            && let Some(rel) = self.rel.union(other.rel)
        {
            Some(Self { file, rel })
        } else {
            None
        }
    }

    pub const fn super_union(self, other: Self) -> Option<Self> {
        let file = self.file;
        if file.0 == other.file.0 {
            Some(Self {
                file,
                rel: self.rel.super_union(other.rel),
            })
        } else {
            None
        }
    }
}

impl RelSpan {
    pub const fn within_file(self, file: FileId) -> Span {
        Span { file, rel: self }
    }
}
