use std::{collections::HashMap, range::Range};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment<'src> {
    /// Bytestring of memory (allows for dynamically-sized types)
    arena: Vec<u8>,
    sp: usize,

    /// Lookup table associating variable names with ranges into the arena.
    /// Each name is associated with a stack, the last element will be the most recent.
    /// When the stack is emptied, its key should be removed from the table; however this isn't guaranteed.
    table: HashMap<&'src str, Vec<Range<usize>>>,
}

impl<'src> Environment<'src> {
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            sp: 0,
            table: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Vec::with_capacity(capacity),
            sp: 0,
            table: HashMap::new(),
        }
    }

    pub fn declare<const N: usize>(&mut self, ident: &'src str) -> &mut [u8; N] {
        todo!()
    }

    pub fn declare_uint(&mut self, ident: &'src str) -> &mut usize {
        let data = self.declare::<{ std::mem::size_of::<usize>() }>(ident);
        // SAFETY: pointer came from a mutable reference
        unsafe { (&raw mut *data).cast::<usize>().as_mut_unchecked() }
    }

    pub fn declare_dyn(&mut self, ident: &'src str, size: usize) -> &mut [u8] {
        todo!()
    }
}
