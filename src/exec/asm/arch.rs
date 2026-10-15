use crate::exec::asm::reg::Registers;
use std::range::Range;

pub fn read_word(pool: &[u8], position: usize) -> Option<usize> {
    let start = position;
    let end = start.checked_add(std::mem::size_of::<usize>())?;
    pool.get(Range { start, end })
        .and_then(|s| s.as_array())
        .copied()
        .map(usize::from_ne_bytes)
}

pub fn write_word(pool: &mut [u8], position: usize, value: usize) -> Option<()> {
    let start = position;
    let end = start.checked_add(std::mem::size_of::<usize>())?;
    pool.get_mut(Range { start, end })
        .map(|data| data.copy_from_slice(&value.to_ne_bytes()))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct VirtualArch {
    pub(super) ram: Box<[u8]>,
    pub(super) stk: Box<[u8]>,
    pub(super) reg: Registers,
}

impl VirtualArch {
    pub const fn new(ram: Box<[u8]>, stk: Box<[u8]>) -> Self {
        Self {
            ram,
            stk,
            reg: Registers::new(),
        }
    }

    /// Returns [`None`] on failure
    #[must_use]
    pub fn push(&mut self, value: usize) -> Option<()> {
        let start = self.reg.sp;
        let end = start
            .checked_add(std::mem::size_of_val(&value))
            .filter(|&i| i < self.stk.len())?;
        write_word(
            &mut self.stk,
            std::mem::replace(&mut self.reg.sp, end),
            value,
        )
        .expect("should never be out of bounds");
        Some(())
    }

    /// Returns [`None`] on failure
    #[must_use]
    pub fn pop(&mut self) -> Option<()> {
        self.reg.sp.checked_sub(1).is_some().then_some(())
    }

    #[must_use]
    pub fn link(&mut self, addr: usize) -> Option<()> {
        self.push(self.reg.ra)?;
        self.reg.ra = addr;
        Some(())
    }
}
