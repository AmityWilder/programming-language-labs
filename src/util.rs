/// Like [`std::iter::TakeWhile`], but only for [`std::iter::Peekable`] iterators,
/// and without consuming the first item that doesn't satisfy the predicate
#[derive(Clone)]
pub struct PeekedTakeWhile<I, P> {
    iter: I,
    flag: bool,
    predicate: P,
}

impl<I, P> PeekedTakeWhile<I, P> {
    const fn new(iter: I, predicate: P) -> PeekedTakeWhile<I, P> {
        PeekedTakeWhile {
            iter,
            flag: false,
            predicate,
        }
    }

    pub fn into_inner(self) -> I {
        self.iter
    }
}

impl<I, J, P> Iterator for PeekedTakeWhile<J, P>
where
    I: Iterator,
    J: std::ops::DerefMut<Target = std::iter::Peekable<I>>,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<I::Item> {
        if self.flag {
            None
        } else if let Some(x) = self.iter.next_if(|x| (self.predicate)(x)) {
            Some(x)
        } else {
            self.flag = true;
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.flag {
            (0, Some(0))
        } else {
            let (_, upper) = self.iter.size_hint();
            (0, upper) // can't know a lower bound, due to the predicate
        }
    }
}

impl<I, J, P> std::iter::FusedIterator for PeekedTakeWhile<J, P>
where
    I: std::iter::FusedIterator,
    J: std::ops::DerefMut<Target = std::iter::Peekable<I>>,
    P: FnMut(&I::Item) -> bool,
{
}

pub trait PeekedTakeWhileExt<I: Iterator>: Sized {
    fn peeked_take_while<P>(self, predicate: P) -> PeekedTakeWhile<Self, P>
    where
        P: FnMut(&I::Item) -> bool;
}

impl<I, J> PeekedTakeWhileExt<I> for J
where
    I: Iterator,
    J: std::ops::DerefMut<Target = std::iter::Peekable<I>>,
{
    fn peeked_take_while<P>(self, predicate: P) -> PeekedTakeWhile<Self, P>
    where
        P: FnMut(&I::Item) -> bool,
    {
        PeekedTakeWhile::new(self, predicate)
    }
}

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

    fn collect_or_drop_all_errs<A>(self) -> Option<A>
    where
        A: Default + Extend<T>,
    {
        self.collect_or_all_errs().map_err(drop_all).ok()
    }
}

impl<T, E, I: IntoIterator<Item = Result<T, E>>> OrAllErrsExt<T, E> for I {}
