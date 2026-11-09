/// Returns `true` regardless of the element being tested.
#[inline]
const fn always<T: ?Sized>(_: &T) -> bool {
    true
}

pub trait MapCategory<T: ?Sized>: Sized {
    type Mapped;

    fn map_category(self) -> Self::Mapped;

    fn map_category_if<P>(self, predicate: P) -> Result<Self::Mapped, Self>
    where
        P: FnOnce(&T) -> bool;
}

pub trait TryMapCategory<T: ?Sized>: Sized {
    type Mapped;

    fn try_map_category(self) -> Result<Self::Mapped, Self> {
        self.try_map_category_if(always)
    }

    fn try_map_category_if<P>(self, predicate: P) -> Result<Self::Mapped, Self>
    where
        P: FnOnce(&T) -> bool;
}

pub trait CategoryEq<T: ?Sized> {
    fn category_eq(&self, value: &T) -> bool;
}

pub trait LocCategoryEq<T: ?Sized>: Sized {
    type Location;

    fn loc_if_category_eq(self, value: &T) -> Result<Self::Location, Self>;
}

/// Based on the intuition that `enumerate` produces `(index, value)`
impl<T, U, R> MapCategory<R> for (T, U)
where
    U: Copy + Into<R>,
{
    type Mapped = (T, R);

    #[inline]
    fn map_category(self) -> Self::Mapped {
        (self.0, self.1.into())
    }

    #[inline]
    fn map_category_if<P>(self, predicate: P) -> Result<Self::Mapped, Self>
    where
        P: FnOnce(&R) -> bool,
    {
        let res = self.1.into();
        if predicate(&res) {
            Ok((self.0, res))
        } else {
            Err(self)
        }
    }
}

/// Based on the intuition that `enumerate` produces `(index, value)`
impl<T, U, R> TryMapCategory<R> for (T, U)
where
    U: Copy + TryInto<R>,
{
    type Mapped = (T, R);

    #[inline]
    fn try_map_category_if<P>(self, predicate: P) -> Result<Self::Mapped, Self>
    where
        P: FnOnce(&R) -> bool,
    {
        if let Some(res) = self.1.try_into().ok().filter(predicate) {
            Ok((self.0, res))
        } else {
            Err(self)
        }
    }
}

/// Based on the intuition that `enumerate` produces `(index, value)`
impl<T, U, Q> CategoryEq<Q> for (T, U)
where
    Q: ?Sized,
    U: PartialEq<Q>,
{
    #[inline]
    fn category_eq(&self, value: &Q) -> bool {
        self.1 == *value
    }
}

/// Based on the intuition that `enumerate` produces `(index, value)`
impl<T, U, Q> LocCategoryEq<Q> for (T, U)
where
    Q: ?Sized,
    U: PartialEq<Q>,
{
    type Location = T;

    #[inline]
    fn loc_if_category_eq(self, value: &Q) -> Result<Self::Location, Self> {
        if self.1 == *value {
            Ok(self.0)
        } else {
            Err(self)
        }
    }
}

pub trait Parser {
    /// The elements being parsed.
    type Item;

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
        Q: ?Sized,
        Self::Item: CategoryEq<Q>,
    {
        self.is_upcoming(|x| x.category_eq(value))
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
    fn pull_if_eq<Q>(&mut self, value: &Q) -> Option<<Self::Item as LocCategoryEq<Q>>::Location>
    where
        Q: ?Sized,
        Self::Item: LocCategoryEq<Q>,
    {
        self.pull_if_map(|x| x.loc_if_category_eq(value))
    }

    /// Extract the upcoming element and convert it
    #[inline]
    fn pull_into<T>(&mut self) -> Option<<Self::Item as MapCategory<T>>::Mapped>
    where
        Self::Item: MapCategory<T>,
    {
        self.pull().map(MapCategory::map_category)
    }

    /// Extract the upcoming element and convert it if it satisfies the predicate
    #[inline]
    fn pull_into_if<T, P>(&mut self, predicate: P) -> Option<<Self::Item as MapCategory<T>>::Mapped>
    where
        P: FnOnce(&T) -> bool,
        Self::Item: MapCategory<T>,
    {
        self.pull_if_map(|x| x.map_category_if(predicate))
    }

    /// Extract the upcoming element and convert it, returning it if it successfully converts
    #[inline]
    fn pull_if_into<T>(&mut self) -> Option<<Self::Item as TryMapCategory<T>>::Mapped>
    where
        Self::Item: TryMapCategory<T>,
    {
        self.pull_if_map(TryMapCategory::try_map_category)
    }

    /// Extract the upcoming element and convert it, returning it if it successfully converts and satisfies the predicate
    #[inline]
    fn pull_if_into_and<T, P>(
        &mut self,
        predicate: P,
    ) -> Option<<Self::Item as TryMapCategory<T>>::Mapped>
    where
        Self::Item: TryMapCategory<T>,
        P: FnOnce(&T) -> bool,
    {
        self.pull_if_map(|x| x.try_map_category_if(predicate))
    }
}

pub trait TryParser: Parser {
    type Error;

    /// A type indicating what was expected, for use in erroring
    type Expect;

    fn error(&mut self, expected: Self::Expect) -> Self::Error;

    /// Extract the upcoming element and return it.
    #[inline]
    fn try_pull(&mut self) -> Result<Self::Item, Self::Error> {
        todo!()
    }

    /// Extract the upcoming element and return it; if it satisfies the predicate.
    #[inline]
    fn try_pull_if<P>(&mut self, predicate: P) -> Result<Self::Item, Self::Error>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        todo!()
    }

    /// Extract the upcoming element, transform it, and return the transformation.
    ///
    /// If the element should not be returned, have `f` return the original element in an [`Err`].
    fn try_pull_if_map<T, F>(&mut self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>;

    /// Extract the upcoming element and return it; if its category is equal to `value`.
    #[inline]
    fn try_pull_if_eq<Q>(
        &mut self,
        value: &Q,
    ) -> Result<<Self::Item as LocCategoryEq<Q>>::Location, Self::Error>
    where
        Q: ?Sized,
        Self::Item: LocCategoryEq<Q>,
    {
        todo!()
    }

    /// Extract the upcoming element and convert it
    #[inline]
    fn try_pull_into<T>(&mut self) -> Result<<Self::Item as MapCategory<T>>::Mapped, Self::Error>
    where
        Self::Item: MapCategory<T>,
    {
        todo!()
    }

    /// Extract the upcoming element and convert it if it satisfies the predicate
    #[inline]
    fn try_pull_into_if<T, P>(
        &mut self,
        predicate: P,
    ) -> Result<<Self::Item as MapCategory<T>>::Mapped, Self::Error>
    where
        P: FnOnce(&T) -> bool,
        Self::Item: MapCategory<T>,
    {
        todo!()
    }

    /// Extract the upcoming element and convert it, returning it if it successfully converts
    #[inline]
    fn try_pull_if_into<T>(
        &mut self,
    ) -> Result<<Self::Item as TryMapCategory<T>>::Mapped, Self::Error>
    where
        Self::Item: TryMapCategory<T>,
    {
        todo!()
    }

    /// Extract the upcoming element and convert it, returning it if it successfully converts and satisfies the predicate
    #[inline]
    fn try_pull_if_into_and<T, P>(
        &mut self,
        predicate: P,
    ) -> Result<<Self::Item as TryMapCategory<T>>::Mapped, Self::Error>
    where
        Self::Item: TryMapCategory<T>,
        P: FnOnce(&T) -> bool,
    {
        todo!()
    }
}

/// A [`Parser`] that typically skips elements, but can be forced not to
pub trait FilterParser: Parser {
    /// Look ahead at the next element.
    fn peek_next(&mut self) -> Option<&Self::Item>;

    /// Check if the next element satisfies `predicate`.
    #[inline]
    fn is_next<P>(&mut self, predicate: P) -> bool
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.peek_next().is_some_and(predicate)
    }

    /// Check if the next element is equal to `value`.
    #[inline]
    fn is_next_eq<Q>(&mut self, value: &Q) -> bool
    where
        Q: ?Sized,
        Self::Item: CategoryEq<Q>,
    {
        self.is_next(|x| x.category_eq(value))
    }

    /// Extract the next element and return it.
    #[inline]
    fn pull_next(&mut self) -> Option<Self::Item> {
        self.pull_next_if(always)
    }

    /// Extract the next element and return it; if it satisfies the predicate.
    #[inline]
    fn pull_next_if<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        self.pull_next_if_map(|x| if predicate(&x) { Ok(x) } else { Err(x) })
    }

    /// Extract the next element, transform it, and return the transformation.
    ///
    /// If the element should not be returned, have `f` return the original element in an [`Err`].
    fn pull_next_if_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>;

    /// Extract the next element and return it; if its category is equal to `value`.
    #[inline]
    fn pull_next_if_eq<Q>(
        &mut self,
        value: &Q,
    ) -> Option<<Self::Item as LocCategoryEq<Q>>::Location>
    where
        Q: ?Sized,
        Self::Item: LocCategoryEq<Q>,
    {
        self.pull_next_if_map(|x| x.loc_if_category_eq(value))
    }

    /// Extract the next element and convert it
    #[inline]
    fn pull_next_into<T>(&mut self) -> Option<<Self::Item as MapCategory<T>>::Mapped>
    where
        Self::Item: MapCategory<T>,
    {
        self.pull_next().map(MapCategory::map_category)
    }

    /// Extract the next element and convert it if it satisfies the predicate
    #[inline]
    fn pull_next_into_if<T, P>(
        &mut self,
        predicate: P,
    ) -> Option<<Self::Item as MapCategory<T>>::Mapped>
    where
        P: FnOnce(&T) -> bool,
        Self::Item: MapCategory<T>,
    {
        self.pull_next_if_map(|x| x.map_category_if(predicate))
    }

    /// Extract the next element and convert it, returning it if it successfully converts
    #[inline]
    fn pull_next_if_into<T>(&mut self) -> Option<<Self::Item as TryMapCategory<T>>::Mapped>
    where
        Self::Item: TryMapCategory<T>,
    {
        self.pull_next_if_map(TryMapCategory::try_map_category)
    }

    /// Extract the next element and convert it, returning it if it successfully converts and satisfies the predicate
    #[inline]
    fn pull_next_if_into_and<T, P>(
        &mut self,
        predicate: P,
    ) -> Option<<Self::Item as TryMapCategory<T>>::Mapped>
    where
        Self::Item: TryMapCategory<T>,
        P: FnOnce(&T) -> bool,
    {
        self.pull_next_if_map(|x| x.try_map_category_if(predicate))
    }
}

pub trait TryFilterParser: FilterParser + TryParser {
    /// Extract the next element and return it.
    #[inline]
    fn try_pull_next(&mut self) -> Result<Self::Item, Self::Error> {
        todo!()
    }

    /// Extract the next element and return it; if it satisfies the predicate.
    #[inline]
    fn try_pull_next_if<P>(&mut self, predicate: P) -> Result<Self::Item, Self::Error>
    where
        P: FnOnce(&Self::Item) -> bool,
    {
        todo!()
    }

    /// Extract the next element, transform it, and return the transformation.
    ///
    /// If the element should not be returned, have `f` return the original element in an [`Err`].
    fn try_pull_next_if_map<T, F>(&mut self, f: F) -> Result<T, Self::Error>
    where
        F: FnOnce(Self::Item) -> Result<T, Self::Item>;

    /// Extract the next element and return it; if its category is equal to `value`.
    #[inline]
    fn try_pull_next_if_eq<Q>(
        &mut self,
        value: &Q,
    ) -> Result<<Self::Item as LocCategoryEq<Q>>::Location, Self::Error>
    where
        Q: ?Sized,
        Self::Item: LocCategoryEq<Q>,
    {
        todo!()
    }

    /// Extract the next element and convert it
    #[inline]
    fn try_pull_next_into<T>(
        &mut self,
    ) -> Result<<Self::Item as MapCategory<T>>::Mapped, Self::Error>
    where
        Self::Item: MapCategory<T>,
    {
        todo!()
    }

    /// Extract the next element and convert it if it satisfies the predicate
    #[inline]
    fn try_pull_next_into_if<T, P>(
        &mut self,
        predicate: P,
    ) -> Result<<Self::Item as MapCategory<T>>::Mapped, Self::Error>
    where
        P: FnOnce(&T) -> bool,
        Self::Item: MapCategory<T>,
    {
        todo!()
    }

    /// Extract the next element and convert it, returning it if it successfully converts
    #[inline]
    fn try_pull_next_if_into<T>(
        &mut self,
    ) -> Result<<Self::Item as TryMapCategory<T>>::Mapped, Self::Error>
    where
        Self::Item: TryMapCategory<T>,
    {
        todo!()
    }

    /// Extract the next element and convert it, returning it if it successfully converts and satisfies the predicate
    #[inline]
    fn try_pull_next_if_into_and<T, P>(
        &mut self,
        predicate: P,
    ) -> Result<<Self::Item as TryMapCategory<T>>::Mapped, Self::Error>
    where
        Self::Item: TryMapCategory<T>,
        P: FnOnce(&T) -> bool,
    {
        todo!()
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
