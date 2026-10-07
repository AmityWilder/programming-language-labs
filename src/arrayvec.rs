use std::mem::MaybeUninit;

/// Custom, minimal implementation of `ArrayVec` as to not include any libraries
pub struct ArrayVec<T, const CAP: usize> {
    buf: [MaybeUninit<T>; CAP],
    /// The number of initialized elements in [`Self::buf`]
    len: usize,
}

impl<T: std::fmt::Debug, const CAP: usize> std::fmt::Debug for ArrayVec<T, CAP> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: PartialEq, const CAP: usize> PartialEq for ArrayVec<T, CAP> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (**self).eq(&**other)
    }
}

impl<T: PartialOrd, const CAP: usize> PartialOrd for ArrayVec<T, CAP> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: Ord, const CAP: usize> Ord for ArrayVec<T, CAP> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: Eq, const CAP: usize> Eq for ArrayVec<T, CAP> {}

impl<T: Clone, const CAP: usize> Clone for ArrayVec<T, CAP> {
    fn clone(&self) -> Self {
        Self {
            buf: std::array::from_fn(|i| {
                self.get(i).map_or(const { MaybeUninit::uninit() }, |val| {
                    MaybeUninit::new(val.clone())
                })
            }),
            len: self.len,
        }
    }
}

// this is the main reason why `ArrayVec` had to be a type instead of just doing this stuff in-place: it might fail to drop
impl<T, const CAP: usize> Drop for ArrayVec<T, CAP> {
    fn drop(&mut self) {
        let buf = self
            .buf
            .get_mut(..self.len)
            .expect("len should not exceed capacity");

        // SAFETY: `len` is the number of initialized elements in `buf`
        unsafe { buf.assume_init_drop() }
    }
}

impl<T, const CAP: usize> std::ops::Deref for ArrayVec<T, CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let buf = self
            .buf
            .get(..self.len)
            .expect("len should not exceed capacity");

        // SAFETY: `len` is the number of initialized elements in `buf`
        unsafe { buf.assume_init_ref() }
    }
}

impl<T, const CAP: usize> std::ops::DerefMut for ArrayVec<T, CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let buf = self
            .buf
            .get_mut(..self.len)
            .expect("len should not exceed capacity");

        // SAFETY: `len` is the number of initialized elements in `buf`
        unsafe { buf.assume_init_mut() }
    }
}

impl<T, const CAP: usize> Default for ArrayVec<T, CAP> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAP: usize> ArrayVec<T, CAP> {
    pub const fn new() -> Self {
        Self {
            buf: [const { MaybeUninit::uninit() }; CAP],
            len: 0,
        }
    }

    /// Returns [`Err`] if out of capacity
    pub const fn push_mut(&mut self, value: T) -> Result<&mut T, T> {
        // if we can't even add 1 without overflowing, we're definitely out of capacity
        if let Some(new_len) = self.len.checked_add(1)
            && new_len <= CAP
        {
            #[expect(clippy::indexing_slicing, reason = "Guarded by 'if' condition")]
            let uninit = &mut self.buf[std::mem::replace(&mut self.len, new_len)];
            Ok(uninit.write(value))
        } else {
            Err(value)
        }
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a ArrayVec<T, CAP> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a mut ArrayVec<T, CAP> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
