use crate::exec::eval::ValueType;
use std::{collections::LinkedList, num::NonZero, range::Range};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct VarData {
    pub ty: ValueType,
    pub range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Scope<'src> {
    vars: Vec<(&'src str, VarData)>,
}

impl<'src> Scope<'src> {
    pub const fn new() -> Self {
        Self { vars: Vec::new() }
    }

    pub fn declare(&mut self, ident: &'src str, data: VarData) {
        self.vars.push((ident, data));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Layer<'src> {
    pub scopes: LinkedList<Scope<'src>>,
}

impl<'src> Layer<'src> {
    pub const fn new() -> Self {
        Self {
            scopes: LinkedList::new(),
        }
    }

    pub fn local_scope(&self) -> Option<&Scope<'src>> {
        self.scopes.back()
    }

    pub fn local_scope_mut(&mut self) -> Option<&mut Scope<'src>> {
        self.scopes.back_mut()
    }

    pub fn push_scope(&mut self) -> &mut Scope<'src> {
        self.scopes.push_back_mut(Scope::new())
    }

    pub fn pop_scope(&mut self) -> Option<Scope<'src>> {
        self.scopes.pop_back()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Table<'src> {
    global: Scope<'src>,
    layers: LinkedList<Layer<'src>>,
}

impl<'src> Table<'src> {
    pub const fn new() -> Self {
        Self {
            global: Scope::new(),
            layers: LinkedList::new(),
        }
    }

    pub fn local_layer(&self) -> Option<&Layer<'src>> {
        self.layers.back()
    }

    pub fn local_layer_mut(&mut self) -> Option<&mut Layer<'src>> {
        self.layers.back_mut()
    }

    pub fn local_scope(&self) -> &Scope<'src> {
        if let Some(scope) = self.local_layer().and_then(Layer::local_scope) {
            scope
        } else {
            &self.global
        }
    }

    pub fn local_scope_mut(&mut self) -> &mut Scope<'src> {
        if let Some(scope) = self.local_layer_mut().and_then(Layer::local_scope_mut) {
            scope
        } else {
            &mut self.global
        }
    }

    pub fn push_layer(&mut self) -> &mut Layer<'src> {
        self.layers.push_back_mut(Layer::new())
    }

    pub fn pop_layer(&mut self) -> Option<Layer<'src>> {
        self.layers.pop_back()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct UntypedPointer(Range<usize>);

impl PartialOrd for UntypedPointer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UntypedPointer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // TBD: what if they have the same start but different end?
        self.0.start.cmp(&other.0.start)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Pointer {
    pub ty: ValueType,
    pub ptr: UntypedPointer,
}

impl PartialOrd for Pointer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pointer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StackMemory<'src> {
    /// Bytestring of memory (allows for dynamically-sized types)
    stack: Box<[u8]>,
    /// Position of the end of in-use memory
    stack_ptr: usize,
    /// Lookup table associating variable names with ranges into the stack.
    ///
    /// The 0th layer is global scope.
    table: Table<'src>,
}

impl<'src> StackMemory<'src> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: vec![const { 0 }; capacity].into_boxed_slice(),
            stack_ptr: 0,
            table: Table::new(),
        }
    }

    /// Returns [`None`] if scope creation failed.
    pub fn push_scope(&mut self) -> Option<NonZero<usize>> {
        todo!()
    }

    /// Returns `false` if attempting to pop global scope.
    #[must_use]
    pub fn pop_scope(&mut self) -> bool {
        todo!()
    }

    /// Pushes a new variable at the current scope
    ///
    /// Returns [`None`] if out of memory
    pub fn push(
        &mut self,
        ident: &'src str,
        ty: ValueType,
        size: usize,
        align: usize,
    ) -> Option<&mut [u8]> {
        assert!(
            align.is_power_of_two(),
            "`align` must be a power of 2\n align: {align}\n  type: {ty}"
        );
        let range = if let Some(size) = NonZero::new(size) {
            let align = NonZero::new(align)
                .unwrap_or_else(|| panic!("sized types cannot align to 0\n type: {ty}"));
            let start = self.stack_ptr.checked_next_multiple_of(align.get())?;
            let end = start.checked_add(size.get())?;
            self.stack_ptr = end;
            Range { start, end }
        } else {
            // ZST
            let start = self.stack_ptr;
            Range { start, end: start }
        };
        let mem: &mut [u8] = self.stack.get_mut(range)?; // do this first so we dont add a variable if there isn't room for it
        if let Some(align) = NonZero::new(align) {
            debug_assert!(
                mem.as_ptr().is_aligned_to(align.get()),
                "pointer must be aligned to `align`\n align: {align}\n   rem: {}",
                mem.as_ptr().addr() & (align.get() - 1)
            );
        }
        self.table
            .local_scope_mut()
            .declare(ident, VarData { ty, range });
        Some(mem)
    }

    /// Returns [`None`] if variable does not exist
    pub fn lookup<Q>(&self, ident: &Q) -> Option<Pointer>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.table
            .local_layer()
            .into_iter()
            .flat_map(|layer| layer.scopes.iter().flat_map(|scope| scope.vars.iter()))
            .chain(self.table.global.vars.iter())
            .rfind(|(name, _)| ident == *name)
            .map(|(_, item)| Pointer {
                ty: item.ty,
                ptr: UntypedPointer(item.range),
            })
    }

    /// Returns [`None`] if the pointer is out of range
    pub fn get(&self, ptr: UntypedPointer) -> Option<&[u8]> {
        self.stack.get(ptr.0)
    }

    /// Returns [`None`] if the pointer is out of range
    pub fn get_mut(&mut self, ptr: UntypedPointer) -> Option<&mut [u8]> {
        self.stack.get_mut(ptr.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment<'src> {
    stack: StackMemory<'src>,
}

impl<'src> Environment<'src> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: StackMemory::with_capacity(capacity),
        }
    }

    pub fn stack(&self) -> &StackMemory<'src> {
        &self.stack
    }

    pub fn stack_mut(&mut self) -> &mut StackMemory<'src> {
        &mut self.stack
    }
}
