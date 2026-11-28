use crate::exec::{EnvId, Value};

// TODO: push down automata

/// Runtime environment
#[derive(Debug)]
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

    /// Get the current position of the stack, so that a function can remember where to rewind to when it ends
    pub fn stack_ptr(&mut self) -> EnvId {
        EnvId(self.stack.len())
    }

    pub fn push(&mut self, value: Value) -> EnvId {
        let id = self.stack.len();
        self.stack.push(value);
        EnvId(id)
    }

    /// Pop all variables off the stack until only IDs lower than `end` remain
    pub fn pop_stack(&mut self, end: EnvId) {
        self.stack.truncate(end.0);
    }

    /// Create a new variable and assign it with the value
    pub fn insert(&mut self, id: EnvId, value: Value) -> &mut Value {
        let EnvId(id) = id;
        if let Some(var) = self.stack.get_mut(id) {
            *var = value;
            var
        } else {
            if id > self.stack.len() {
                self.stack.resize_with(id, Value::default);
            }
            self.stack.push_mut(value)
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
