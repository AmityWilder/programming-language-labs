use crate::{collections::linked_stack::LinkedStack, exec::Value};

/// Runtime environment
pub struct Environment {
    stack: LinkedStack<Closure>,
    heap: Vec<Box<[Value]>>,
}

/// Runtime sub-environment
pub struct Closure {
    data: Box<[Value]>,
}
