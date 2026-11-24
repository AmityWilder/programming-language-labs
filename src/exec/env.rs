use super::EnvId;
use crate::exec::Value;

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

    /// Create a new variable and assign it with the value
    pub fn insert(&mut self, id: EnvId, value: Value) -> &mut Value {
        let EnvId(id) = id;
        let len = self.stack.len();
        assert!(
            id >= len,
            "should not insert into existing variable\
            \n  left: {id}\
            \n right: {len}",
        );
        if id > len {
            self.stack.resize_with(id, Value::default);
        }
        self.stack.push_mut(value)
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
