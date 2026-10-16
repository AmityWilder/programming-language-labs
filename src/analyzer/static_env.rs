use crate::exec::eval::ValueType;
use std::collections::HashMap;

/// [`Environment`](crate::exec::env::Environment) specialized for static analysis
pub struct StaticEnvironment<'src> {
    variables: HashMap<&'src str, ValueType>,
}

impl<'src> StaticEnvironment<'src> {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}
