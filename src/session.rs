use crate::{
    error::Error,
    src_code::{
        code::{SourceCode, SourceLoc},
        map::SourceMap,
    },
};
use std::{cell::RefCell, path::PathBuf};

pub struct Session {
    source_map: RefCell<SourceMap>,
    errors: RefCell<Vec<Error>>,
}

impl Session {
    pub fn new() -> Self {
        let mut source_map = SourceMap::new();

        source_map.push(
            SourceCode::new(
                SourceLoc::File(PathBuf::from("builtin").join("std").with_extension("bats")),
                include_str!("batstd/std.bats").to_string(),
            )
            .expect("standard library should not be valid"),
        );

        Self {
            source_map: RefCell::new(source_map),
            errors: RefCell::new(Vec::new()),
        }
    }

    pub fn push_err(&self, e: Error) {
        self.errors.borrow_mut().push(e);
    }

    pub fn extend_err(&self, e: impl IntoIterator<Item = Error>) {
        self.errors.borrow_mut().extend(e);
    }
}
