use crate::exec::eval::ValueType;
use std::{collections::LinkedList, num::NonZero, range::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct VarData {
    pub range: Range<usize>,
    pub ty: ValueType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Scope<'src> {
    pub vars: Vec<(&'src str, VarData)>,
}

impl Scope<'_> {
    pub const fn new() -> Self {
        Self { vars: Vec::new() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Domain {
    #[default]
    Stack,
    Global,
}

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
pub struct StaticEnvironment<'src> {
    /// Global variables
    global: Scope<'src>,
    /// Local variables
    #[expect(
        clippy::linkedlist,
        reason = "scopes should be able to push and pop without many copies"
    )]
    stack: LinkedList<Scope<'src>>,

    /// Global pointer
    gp: usize,
    /// Stack pointer
    sp: usize,
    sp_max: usize,
}

impl<'src> StaticEnvironment<'src> {
    pub const fn new() -> Self {
        Self {
            global: Scope::new(),
            stack: LinkedList::new(),
            gp: 0,
            sp: 0,
            sp_max: 0,
        }
    }

    pub fn lookup<Q>(&self, name: &Q) -> Option<(&VarData, Domain)>
    where
        Q: ?Sized + PartialEq<str>,
    {
        self.stack
            .iter()
            .flat_map(|scope| scope.vars.iter())
            .rfind(|(k, _)| name == *k)
            .map(|(_, data)| (data, Domain::Stack))
            .or_else(|| {
                self.global
                    .vars
                    .iter()
                    .rfind(|(k, _)| name == *k)
                    .map(|(_, data)| (data, Domain::Global))
            })
    }

    pub fn push_scope(&mut self) -> &mut Scope<'src> {
        self.stack.push_back_mut(Scope::new())
    }

    /// Returns [`None`] if `ty` is not constructable
    fn create(head: &mut usize, ty: ValueType) -> Option<VarData> {
        let size = ty.size()?;
        let align = ty.align()?;
        assert!(
            align.is_power_of_two(),
            "`align` must be a power of 2\n align: {align}\n  type: {ty}"
        );
        let range = if let Some(size) = NonZero::new(size) {
            let align = NonZero::new(align)
                .unwrap_or_else(|| panic!("sized types cannot align to 0\n type: {ty}"));
            let start = head.checked_next_multiple_of(align.get())?;
            let end = start.checked_add(size.get())?;
            *head = end;
            Range { start, end }
        } else {
            // ZST
            let start = *head;
            Range { start, end: start }
        };
        Some(VarData { range, ty })
    }

    /// Returns [`None`] if `ty` is not constructable
    pub fn push_global<'env>(
        &'env mut self,
        name: &'src str,
        ty: ValueType,
    ) -> Option<&'env mut VarData> {
        Self::create(&mut self.gp, ty).map(|data| &mut self.global.vars.push_mut((name, data)).1)
    }

    /// Returns [`None`] if `ty` is not constructable
    pub fn push_local<'env>(
        &'env mut self,
        name: &'src str,
        ty: ValueType,
    ) -> Option<&'env mut VarData> {
        Self::create(&mut self.sp, ty).map(|data| {
            self.sp_max = self.sp_max.max(self.sp);
            let scope = if let Some(scope) = self.stack.back_mut() {
                scope
            } else {
                self.push_scope() // TBD: is this a legitimate way of handling this?
            };
            &mut scope.vars.push_mut((name, data)).1
        })
    }

    // TODO: needs to `Drop` all elements in memory!!
    pub fn pop_scope(&mut self) -> Option<Scope<'src>> {
        self.stack.pop_back().inspect(|_| {
            self.sp = self
                .stack
                .back()
                .and_then(|scope| scope.vars.last())
                .map_or(0, |var| var.1.range.end);
        })
    }

    #[inline]
    pub const fn sp_max(&self) -> usize {
        self.sp_max
    }

    #[inline]
    pub const fn gp_max(&self) -> usize {
        self.gp
    }
}
