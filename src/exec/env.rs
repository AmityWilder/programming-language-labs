use std::{collections::HashMap, range::Range};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment<'src> {
    /// Bytestring of memory (allows for dynamically-sized types)
    arena: Vec<u8>,
    stack_ptr: usize,

    /// Lookup table associating variable names with ranges into the arena.
    /// Each name is associated with a stack, the last element will be the most recent.
    /// When the stack is emptied, its key should be removed from the table; however this isn't guaranteed.
    table: HashMap<&'src str, Vec<Range<usize>>>,
}

impl<'src> Environment<'src> {
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            stack_ptr: 0,
            table: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Vec::with_capacity(capacity),
            stack_ptr: 0,
            table: HashMap::new(),
        }
    }

    /// Returns [`None`] if out of memory
    pub fn declare_dyn(
        &mut self,
        ident: &'src str,
        align: usize,
        size: usize,
    ) -> Option<&mut [u8]> {
        let start = self.stack_ptr.checked_next_multiple_of(align)?;
        let end = start.checked_add(size)?;
        self.stack_ptr = end;
        let range = Range { start, end };
        let mem = self.arena.get_mut(range)?;
        self.table.entry(ident).or_default().push(range);
        Some(mem)
    }

    /// Returns [`None`] if out of memory
    pub fn declare<const N: usize>(
        &mut self,
        ident: &'src str,
        align: usize,
    ) -> Option<&mut [u8; N]> {
        self.declare_dyn(ident, align, N).map(|slice| {
            slice
                .as_mut_array()
                .expect("declare_dyn should allocate exactly N bytes")
        })
    }
}
