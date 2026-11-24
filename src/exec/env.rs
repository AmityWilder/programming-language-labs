use super::EnvId;
use crate::exec::Value;

/// Runtime environment
pub struct Environment {
    stack: Vec<Value>,
    heap: Vec<Box<[Value]>>,
}

impl Environment {
    pub const fn new() -> Self {
        Self {
            stack: Vec::new(),
            heap: Vec::new(),
        }
    }

    pub fn get(&self, id: EnvId) -> Option<&Value> {
        self.stack.get(id.0)
    }

    pub fn get_mut(&mut self, id: EnvId) -> Option<&mut Value> {
        self.stack.get_mut(id.0)
    }

    pub fn slice(&self, start: EnvId, count: usize) -> Option<&[Value]> {
        let start = start.0;
        let end = start + count;
        self.stack.get(start..end)
    }

    pub fn slice_mut(&mut self, start: EnvId, count: usize) -> Option<&mut [Value]> {
        let start = start.0;
        let end = start + count;
        self.stack.get_mut(start..end)
    }
}

/// Runtime sub-environment
pub struct Closure {
    data: Vec<Value>,
}

impl Closure {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }
}
