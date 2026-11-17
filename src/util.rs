type ResErr<I, E> = fn(<I as Iterator>::Item) -> Option<E>;

pub type ErrorIter<I, E> =
    std::iter::Chain<std::iter::Once<E>, std::iter::FilterMap<I, ResErr<I, E>>>;

pub fn drop_all(iter: impl Iterator) {
    iter.for_each(drop);
}

pub trait OrAllErrsExt<T, E>: Sized + IntoIterator<Item = Result<T, E>> {
    fn ok_or_all_errs<F>(self, mut visit_ok: F) -> Result<(), ErrorIter<Self::IntoIter, E>>
    where
        F: FnMut(T),
    {
        let mut iter = self.into_iter();
        while let Some(res) = iter.next() {
            match res {
                Ok(x) => visit_ok(x),
                Err(e) => {
                    let f: ResErr<Self::IntoIter, E> = Result::err;
                    return Err(std::iter::once(e).chain(iter.filter_map(f)));
                }
            }
        }
        Ok(())
    }

    /// Returns true if there were no errors
    fn ok_or_drop_all_errs<F>(self, visit_ok: F) -> bool
    where
        F: FnMut(T),
    {
        self.ok_or_all_errs(visit_ok).map_err(drop_all).is_ok()
    }

    fn collect_or_all_errs<A>(self) -> Result<A, ErrorIter<Self::IntoIter, E>>
    where
        A: Default + Extend<T>,
    {
        let iter = self.into_iter();
        let mut buf = A::default();
        let (lo, hi) = iter.size_hint();
        buf.extend_reserve(hi.unwrap_or(lo));
        iter.ok_or_all_errs(|x| buf.extend_one(x)).map(|()| buf)
    }

    // TODO: can this be specialized for ExactSizeIterator?
    fn collect_boxed_or_all_errs(self) -> Result<Box<[T]>, ErrorIter<Self::IntoIter, E>> {
        self.collect_or_all_errs().map(Vec::into_boxed_slice)
    }

    fn collect_or_drop_all_errs<A>(self) -> Option<A>
    where
        A: Default + Extend<T>,
    {
        self.collect_or_all_errs().map_err(drop_all).ok()
    }

    fn collect_boxed_or_drop_all_errs(self) -> Option<Box<[T]>> {
        self.collect_boxed_or_all_errs().map_err(drop_all).ok()
    }
}

impl<T, E, I: IntoIterator<Item = Result<T, E>>> OrAllErrsExt<T, E> for I {}
