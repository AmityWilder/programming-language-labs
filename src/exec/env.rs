use std::range::Range;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Environment<'stk> {
    stack: &'stk mut [u8],
    /// Keeps track of offsets to ranges - essentially, the position of the
    /// first element in `stack` relative to the **original** buffer
    sp: usize,
}

impl<'stk> Environment<'stk> {
    pub const fn new(buf: &'stk mut [u8]) -> Self {
        Self { stack: buf, sp: 0 }
    }

    /// Declare that a variable is now in use and update the stack pointer to reflect this
    pub fn stack_push(&mut self, range: Range<usize>) {
        #[cfg(debug_assertions)]
        println!(
            "pushing to stack: sp: {} -> {} ({range:?})",
            self.sp, range.end
        );
        if self.sp < range.end {
            self.sp = range.end;
        } else {
            #[cfg(debug_assertions)]
            println!(
                "WARNING: pushing to a position the stack is already above: be careful not to trample over a variable!"
            );
        }
    }

    /// Declare that all this variable and all variables above it are no longer in use and update the stack pointer to reflect this
    pub fn stack_pop(&mut self, range: Range<usize>) {
        #[cfg(debug_assertions)]
        println!(
            "popping from stack: sp: {} -> {} ({range:?})",
            self.sp, range.start
        );
        if self.sp > range.start {
            self.sp = range.start;
        } else {
            #[cfg(debug_assertions)]
            println!(
                "WARNING: popping to a position the stack is already below; you may have been using variables that weren't pushed!"
            );
        }
    }

    /// Access a variable on the stack
    ///
    /// Does NOT call [`stack_push`](Self::stack_push)
    ///
    /// Returns [`None`] if the range is out of bounds
    pub fn get(&self, range: Range<usize>) -> Option<&[u8]> {
        self.stack.get(range)
    }

    /// Mutably access a variable on the stack
    ///
    /// Automatically calls [`stack_push`](Self::stack_push)
    ///
    /// Returns [`None`] if the range is out of bounds
    pub fn get_mut(&mut self, range: Range<usize>) -> Option<&mut [u8]> {
        self.stack_push(range);
        self.stack.get_mut(range)
    }

    /// Mutably access a variable on the stack
    ///
    /// Does NOT call [`stack_push`](Self::stack_push)
    ///
    /// Returns [`None`] if any range is out of bounds or overlapping
    pub fn get_disjoint_mut<const N: usize>(
        &mut self,
        ranges: [Range<usize>; N],
    ) -> Result<[&mut [u8]; N], std::slice::GetDisjointMutError> {
        self.stack.get_disjoint_mut(ranges.map(|range| range))
    }

    /// Request a temporary variable
    ///
    /// Automatically calls [`stack_push`](Self::stack_push)
    ///
    /// Returns [`None`] if out of memory
    pub fn temp(&mut self, size: usize) -> Option<(Range<usize>, &mut [u8])> {
        #[cfg(debug_assertions)]
        println!("INFO: requesting a {size}-byte temporary");
        // TODO: alignment
        let range = Range {
            start: self.sp,
            end: self.sp.checked_add(size)?,
        };
        #[cfg(debug_assertions)]
        println!("INFO: request granted: range: {range:?}");
        self.stack_push(range);
        self.stack.get_mut(range).map(|mem| (range, mem))
    }
}
