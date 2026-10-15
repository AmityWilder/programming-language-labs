use crate::exec::asm::reg::Registers;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct VirtualArch {
    pub(super) ram: Box<[usize]>,
    pub(super) stk: Box<[usize]>,
    pub(super) reg: Registers,
}

impl VirtualArch {
    pub const fn new(ram: Box<[usize]>, stk: Box<[usize]>) -> Self {
        Self {
            ram,
            stk,
            reg: Registers::new(),
        }
    }

    /// Returns [`None`] on failure
    pub fn push(&mut self, value: usize) -> Option<()> {
        *self
            .stk
            .get_mut(self.reg.sp)
            .expect("should never be out of bounds") = value;
        self.reg
            .sp
            .checked_add(1)
            .is_some_and(|i| i < self.stk.len())
            .then_some(())
    }

    /// Returns [`None`] on failure
    pub fn pop(&mut self) -> Option<()> {
        self.reg.sp.checked_sub(1).is_some().then_some(())
    }

    pub fn link(&mut self, addr: usize) {
        self.push(self.reg.ra);
        self.reg.ra = addr;
    }
}
