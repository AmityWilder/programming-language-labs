/// Returns `true` regardless of the element being tested.
#[inline]
const fn always<T>(_: &T) -> bool {
    true
}

pub trait ParserElement {
    /// The part of the element that represents location-specific information, such as a lexeme.
    type Location;

    /// The part of the element that represents general category information, such as a token kind.
    type Category;

    /// A reference to the category information about the element
    type CategoryRef<'a>
    where
        Self: 'a;

    type Mapped<T>;

    /// Apply a transformation to the category part of the element
    fn map_category<T, F>(self, f: F) -> Self::Mapped<T>
    where
        F: FnOnce(Self::Category) -> T;

    /// Exclude the category information about the element and keep only the location,
    /// since the category information passes equality.
    fn into_location(self) -> Self::Location;

    /// The category info about the element - the part that is tested for equality.
    fn category(&self) -> Self::CategoryRef<'_>;
}

/// Based on the intuition that `enumerate` returns (index, value).
impl<L, C> ParserElement for (L, C) {
    type Location = L;
    type Category = C;
    type Mapped<R> = (L, R);

    type CategoryRef<'a>
        = &'a C
    where
        Self: 'a;

    #[inline]
    fn map_category<R, F>(self, f: F) -> Self::Mapped<R>
    where
        F: FnOnce(Self::Category) -> R,
    {
        (self.0, f(self.1))
    }

    #[inline]
    fn into_location(self) -> Self::Location {
        self.0
    }

    #[inline]
    fn category(&self) -> Self::CategoryRef<'_> {
        &self.1
    }
}

pub trait Parser {
    /// The elements being parsed.
    type Item: ParserElement;

    /// Look ahead at the upcoming element.
    fn peek(&mut self) -> Option<&Self::Item>;

    /// Check if the upcoming element satisfies `predicate`.
    #[inline]
    fn is_upcoming<P>(&mut self, predicate: P) -> bool
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.peek().is_some_and(predicate)
    }

    /// Check if the upcoming element is equal to `value`.
    #[inline]
    fn is_upcoming_eq<Q>(&mut self, value: &Q) -> bool
    where
        Q: ?Sized + PartialEq<Self::Item>,
    {
        self.is_upcoming(|x| value == x)
    }

    /// Extract the upcoming element and return it.
    #[inline]
    fn pull(&mut self) -> Option<Self::Item> {
        self.pull_if(always)
    }

    /// Extract the upcoming element and return it; if it satisfies the predicate.
    #[inline]
    fn pull_if<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.pull_if_map(|x| if predicate(&x) { Ok(x) } else { Err(x) })
    }

    /// Extract the upcoming element, transform it, and return the transformation.
    ///
    /// If the element should not be returned, have `f` return the original element in an [`Err`].
    fn pull_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>;

    /// Extract the upcoming element and return it; if its category is equal to `value`.
    #[inline]
    fn pull_if_eq<T, Q>(&mut self, value: T) -> Option<<Self::Item as ParserElement>::Location>
    where
        T: Into<Q>,
        Q: for<'a> PartialEq<<Self::Item as ParserElement>::CategoryRef<'a>>,
    {
        self.pull_if(|x| value.into() == x.category())
            .map(<Self::Item>::into_location)
    }

    #[inline]
    fn pull_into<T>(&mut self) -> Option<<Self::Item as ParserElement>::Mapped<T>>
    where
        <Self::Item as ParserElement>::Category: Into<T>,
    {
        self.pull().map(|x| x.map_category(Into::into))
    }

    #[inline]
    fn pull_into_if<T, P>(
        &mut self,
        predicate: P,
    ) -> Option<<Self::Item as ParserElement>::Mapped<T>>
    where
        Self::Item: Into<T> + Copy,
        <Self::Item as ParserElement>::Mapped<T>: ParserElement,
        P: FnOnce(
            <<Self::Item as ParserElement>::Mapped<T> as ParserElement>::CategoryRef<'_>,
        ) -> bool,
    {
        self.pull_if_map(|x| {
            if let y = x.map_category(Into::into)
                && predicate(y.category())
            {
                Ok((l, y))
            } else {
                Err(x)
            }
        })
    }

    #[inline]
    fn pull_if_into<T>(&mut self) -> Option<<Self::Item as ParserElement>::Mapped<T>>
    where
        Self::Item: TryInto<T> + Copy,
    {
        self.pull_if_into_and(always)
    }

    #[inline]
    fn pull_if_into_and<T, P>(
        &mut self,
        predicate: P,
    ) -> Option<<Self::Item as ParserElement>::Mapped<T>>
    where
        Self::Item: TryInto<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_if_map(|x| {
            if let Ok(y) = x.try_into()
                && predicate(&y)
            {
                Ok(y)
            } else {
                Err(x)
            }
        })
    }
}

pub trait TryParser: Parser {
    type Error;

    /// A type indicating what was expected, for use in erroring
    type Expect;

    fn error(&mut self, expected: Self::Expect) -> Self::Error;

    #[inline]
    fn try_pull(&mut self, expected: Self::Expect) -> Result<Self::Item, Self::Error> {
        self.pull().ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_if<P>(
        &mut self,
        expected: Self::Expect,
        predicate: P,
    ) -> Result<Self::Item, Self::Error>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.pull_if(predicate).ok_or_else(|| self.error(expected))
    }

    fn try_pull_if_map<T, F>(&mut self, expected: Self::Expect, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>;

    #[inline]
    fn try_pull_if_eq<T, Q>(
        &mut self,
        expected: Self::Expect,
        value: T,
    ) -> Result<<Self::Item as ParserElement>::Location, Self::Error>
    where
        T: Into<Q>,
        Q: for<'a> PartialEq<<Self::Item as ParserElement>::CategoryRef<'a>>,
    {
        self.pull_if_eq(value).ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_into<T>(&mut self, expected: Self::Expect) -> Result<T, Self::Error>
    where
        Self::Item: Into<T>,
    {
        self.pull_into().ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_into_if<T, P>(
        &mut self,
        expected: Self::Expect,
        predicate: P,
    ) -> Result<T, Self::Error>
    where
        Self::Item: Into<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_into_if(predicate)
            .ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_if_into<T>(&mut self, expected: Self::Expect) -> Result<T, Self::Error>
    where
        Self::Item: TryInto<T> + Copy,
    {
        self.pull_if_into().ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_if_into_and<T, P>(
        &mut self,
        expected: Self::Expect,
        predicate: P,
    ) -> Result<T, Self::Error>
    where
        Self::Item: TryInto<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_if_into_and(predicate)
            .ok_or_else(|| self.error(expected))
    }
}

/// A [`Parser`] that typically skips elements, but can be forced not to
pub trait FilterParser: Parser {
    fn peek_next(&mut self) -> Option<&Self::Item>;

    #[inline]
    fn is_next<P>(&mut self, predicate: P) -> bool
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.peek_next().is_some_and(predicate)
    }

    #[inline]
    fn is_next_eq<Q>(&mut self, value: &Q) -> bool
    where
        Q: ?Sized + PartialEq<Self::Item>,
    {
        self.is_next(|x| value == x)
    }

    #[inline]
    fn pull_next(&mut self) -> Option<Self::Item> {
        self.pull_next_if(always)
    }

    #[inline]
    fn pull_next_if<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.pull_next_if_map(|x| if predicate(&x) { Ok(x) } else { Err(x) })
    }

    fn pull_next_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>;

    #[inline]
    fn pull_next_if_eq<Q>(&mut self, value: &Q) -> Option<Self::Item>
    where
        Q: ?Sized + PartialEq<Self::Item>,
    {
        self.pull_next_if(|item| value == item)
    }

    #[inline]
    fn pull_next_into<T>(&mut self) -> Option<T>
    where
        Self::Item: Into<T>,
    {
        self.pull_next().map(Into::into)
    }

    #[inline]
    fn pull_next_into_if<T, P>(&mut self, predicate: P) -> Option<T>
    where
        Self::Item: Into<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_next_if_map(|x| {
            if let y = x.into()
                && predicate(&y)
            {
                Ok(y)
            } else {
                Err(x)
            }
        })
    }

    #[inline]
    fn pull_next_if_into<T>(&mut self) -> Option<T>
    where
        Self::Item: TryInto<T> + Copy,
    {
        self.pull_next_if_into_and(always)
    }

    #[inline]
    fn pull_next_if_into_and<T, P>(&mut self, predicate: P) -> Option<T>
    where
        Self::Item: TryInto<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_next_if_map(|x| {
            if let Ok(y) = x.try_into()
                && predicate(&y)
            {
                Ok(y)
            } else {
                Err(x)
            }
        })
    }
}

pub trait TryFilterParser: FilterParser + TryParser {
    #[inline]
    fn try_pull_next(&mut self, expected: Self::Expect) -> Result<Self::Item, Self::Error> {
        self.pull_next().ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_next_if<P>(
        &mut self,
        expected: Self::Expect,
        predicate: P,
    ) -> Result<Self::Item, Self::Error>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.pull_next_if(predicate)
            .ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_next_if_eq<Q>(
        &mut self,
        expected: Self::Expect,
        value: &Q,
    ) -> Result<Self::Item, Self::Error>
    where
        Q: ?Sized + PartialEq<Self::Item>,
    {
        self.pull_next_if_eq(value)
            .ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_next_into<T>(&mut self, expected: Self::Expect) -> Result<T, Self::Error>
    where
        Self::Item: Into<T>,
    {
        self.pull_next_into().ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_next_into_if<T, P>(
        &mut self,
        expected: Self::Expect,
        predicate: P,
    ) -> Result<T, Self::Error>
    where
        Self::Item: Into<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_next_into_if(predicate)
            .ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_next_if_into<T>(&mut self, expected: Self::Expect) -> Result<T, Self::Error>
    where
        Self::Item: TryInto<T> + Copy,
    {
        self.pull_next_if_into().ok_or_else(|| self.error(expected))
    }

    #[inline]
    fn try_pull_next_if_into_and<T, P>(
        &mut self,
        expected: Self::Expect,
        predicate: P,
    ) -> Result<T, Self::Error>
    where
        Self::Item: TryInto<T> + Copy,
        P: FnOnce(&T) -> bool,
    {
        self.pull_next_if_into_and(predicate)
            .ok_or_else(|| self.error(expected))
    }
}

pub trait ParseTrivial<T>: Sized {
    fn parse_from<P: ?Sized + Parser<Item = T>>(parser: &mut P) -> Self;
}

pub trait ParseTrivialExt<T>
where
    Self: Parser,
    T: ParseTrivial<Self::Item>,
{
    fn parse(&mut self) -> T {
        T::parse_from(self)
    }
}

impl<P: Parser, T: ParseTrivial<P::Item>> ParseTrivialExt<T> for P {}

pub trait TryParseTrivial<T>: Sized {
    fn parse_from<P: ?Sized + Parser<Item = T>>(parser: &mut P) -> Option<Self>;
}

pub trait TryParseTrivialExt<T>
where
    Self: Parser,
    T: TryParseTrivial<Self::Item>,
{
    fn parse(&mut self) -> Option<T> {
        T::parse_from(self)
    }
}

impl<P: Parser, T: TryParseTrivial<P::Item>> TryParseTrivialExt<T> for P {}

pub trait Parse<T>: Sized {
    type Error;

    fn parse_from<P: ?Sized + Parser<Item = T>>(parser: &mut P) -> Result<Self, Self::Error>;
}

pub trait ParseExt<T>
where
    Self: Parser,
    T: Parse<Self::Item>,
{
    fn parse(&mut self) -> Result<T, T::Error> {
        T::parse_from(self)
    }
}

impl<P: Parser, T: Parse<P::Item>> ParseExt<T> for P {}

pub trait TryParse<T>: Sized {
    type Error;

    fn parse_from<P: ?Sized + Parser<Item = T>>(
        parser: &mut P,
    ) -> Option<Result<Self, Self::Error>>;
}

pub trait TryParseExt<T>
where
    Self: Parser,
    T: TryParse<Self::Item>,
{
    fn parse(&mut self) -> Option<Result<T, T::Error>> {
        T::parse_from(self)
    }
}

impl<P: Parser, T: TryParse<P::Item>> TryParseExt<T> for P {}
