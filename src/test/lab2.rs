//! AST Printer

use crate::{
    error::{ContextError, ErrorType},
    scanner::{
        token::{Token, punc::Punctuation, value::LexValue},
        tokenize,
    },
};
use std::assert_matches;
