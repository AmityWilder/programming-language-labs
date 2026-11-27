type ResErr<I, E> = fn(<I as Iterator>::Item) -> Option<E>;

pub type ErrorIter<I, E> =
    std::iter::Chain<std::iter::Once<E>, std::iter::FilterMap<I, ResErr<I, E>>>;

#[inline]
pub fn drop_all(iter: impl IntoIterator) {
    iter.into_iter().for_each(drop);
}

#[derive(Default)]
pub struct DropAll;

impl<E> FromIterator<E> for DropAll {
    #[inline]
    fn from_iter<I: IntoIterator<Item = E>>(iter: I) -> Self {
        drop_all(iter);
        Self
    }
}

impl Extend<Self> for DropAll {
    fn extend<I: IntoIterator<Item = Self>>(&mut self, iter: I) {
        drop_all(iter.into_iter());
    }
}

pub trait OrAllErrsExt<T, E>: Sized + IntoIterator<Item = Result<T, E>> {
    #[inline]
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

    #[inline]
    fn ok_or_acc_errs<F, B>(self, visit_ok: F) -> Result<(), B>
    where
        F: FnMut(T),
        B: FromIterator<E>,
    {
        self.ok_or_all_errs(visit_ok).map_err(B::from_iter)
    }

    #[inline]
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

    #[inline]
    fn collect_or_acc_errs<A, B>(self) -> Result<A, B>
    where
        A: Default + Extend<T>,
        B: FromIterator<E>,
    {
        self.collect_or_all_errs().map_err(B::from_iter)
    }
}

impl<T, E, I: IntoIterator<Item = Result<T, E>>> OrAllErrsExt<T, E> for I {}

/// The `err_or_*` counterparts to the standard library's `ok_or_*` methods on [`Option`].
pub trait ErrOrExt<E> {
    fn err_or<T>(self, v: T) -> Result<T, E>;

    fn err_or_else<T, F>(self, v: F) -> Result<T, E>
    where
        F: FnOnce() -> T;
}

impl<E> ErrOrExt<E> for Option<E> {
    #[inline]
    fn err_or<T>(self, v: T) -> Result<T, E> {
        match self {
            Some(err) => Err(err),
            None => Ok(v),
        }
    }

    #[inline]
    fn err_or_else<T, F>(self, v: F) -> Result<T, E>
    where
        F: FnOnce() -> T,
    {
        match self {
            Some(err) => Err(err),
            None => Ok(v()),
        }
    }
}

pub trait TransposeErrExt<T, E> {
    fn transpose_err(self) -> Result<T, Option<E>>;
}

impl<T, E> TransposeErrExt<T, E> for Option<Result<T, E>> {
    #[inline]
    fn transpose_err(self) -> Result<T, Option<E>> {
        match self {
            Some(Ok(x)) => Ok(x),
            Some(Err(e)) => Err(Some(e)),
            None => Err(None),
        }
    }
}

pub trait InspectNone {
    fn inspect_none<F>(self, f: F) -> Self
    where
        F: FnOnce();
}

impl<T> InspectNone for Option<T> {
    fn inspect_none<F>(self, f: F) -> Self
    where
        F: FnOnce(),
    {
        if matches!(self, None) {
            f();
        }
        self
    }
}

pub trait InspectAlways: Sized {
    fn inspect_always(self, f: impl FnOnce(&Self)) -> Self {
        f(&self);
        self
    }
}

impl<T> InspectAlways for Option<T> {}
impl<T, E> InspectAlways for Result<T, E> {}
