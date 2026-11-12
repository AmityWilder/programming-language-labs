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

pub fn all_errs_if_any<I, T, E, F>(
    iter: I,
    mut visit_ok: F,
) -> Result<(), ErrorIter<I::IntoIter, E>>
where
    I: IntoIterator<Item = Result<T, E>>,
    F: FnMut(T),
{
    let mut iter = iter.into_iter();
    while let Some(res) = iter.next() {
        match res {
            Ok(x) => visit_ok(x),
            Err(e) => {
                let f: ResErr<I::IntoIter, E> = Result::err;
                return Err(std::iter::once(e).chain(iter.filter_map(f)));
            }
        }
    }
    Ok(())
}

pub fn collect_all_errs_if_any<I, T, E, A>(iter: I) -> Result<A, ErrorIter<I::IntoIter, E>>
where
    A: Default + Extend<T>,
    I: IntoIterator<Item = Result<T, E>>,
{
    let iter = iter.into_iter();
    let mut buf = A::default();
    let (lo, hi) = iter.size_hint();
    buf.extend_reserve(hi.unwrap_or(lo));
    all_errs_if_any(iter, |x| buf.extend_one(x)).map(|()| buf)
}
