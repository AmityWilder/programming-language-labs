use std::range::Range;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Environment<'stk> {
    stack: &'stk mut [u8],
    /// Keeps track of offsets to ranges - essentially, the position of the
    /// first element in `stack` relative to the **original** buffer
    offset: usize,
}

impl<'stk> Environment<'stk> {
    pub fn new(buf: &'stk mut [u8], offset: usize) -> Self {
        Self { stack: buf, offset }
    }

    fn offset_range(&self, Range { start, end }: Range<usize>) -> Range<usize> {
        Range {
            start: start - self.offset,
            end: end - self.offset,
        }
    }

    /// Access a variable on the stack
    ///
    /// Returns [`None`] if the range is out of bounds
    pub fn get(&self, range: Range<usize>) -> Option<&[u8]> {
        self.stack.get(self.offset_range(range))
    }

    /// Mutably access a variable on the stack
    ///
    /// The variable is the middle tuple field
    ///
    /// Returns [`None`] if the range is out of bounds
    pub fn get_mut(
        &mut self,
        range: Range<usize>,
    ) -> Option<(Environment<'_>, &mut [u8], Environment<'_>)> {
        let range = self.offset_range(range);
        let (main, post) = self.stack.split_at_mut_checked(range.end)?;
        let (pre, target) = main.split_at_mut(range.start);
        Some((
            Environment::new(pre, self.offset),
            target,
            Environment::new(post, self.offset + range.end),
        ))
    }

    /// Request a temporary variable
    ///
    /// The temporary variable is the first tuple field
    ///
    /// Returns [`None`] if out of memory
    pub fn temp(&mut self, size: usize) -> Option<(&mut [u8], Environment<'_>)> {
        self.stack
            .split_at_mut_checked(size)
            .map(|(temporary, rest)| (temporary, Environment::new(rest, self.offset + size)))
    }
}
