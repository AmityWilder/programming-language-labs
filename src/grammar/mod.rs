//! Context-free grammar
//!
//! ```not_code
//! <let_statement> ::= "let" <binding> "=" <expression>
//! <expression> ::=
//!     <literal>
//!     | "(" <expression> ")"
//!     | <expression> "+" <expression>
//!     | <expression> "-" <expression>
//!     | <expression> "*" <expression>
//!     | <expression> "/" <expression>
//!     | ...
//! <literal> = <number literal> | <string literal> | <char literal> | <bool literal>
//! ```

#![allow(clippy::missing_docs_in_private_items, reason = "under construction")]

use crate::{
    error::ContextError,
    scanner::token::{Punctuation, Token, TokenValue, TokenValueSimplicity},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NonTerminal {
    Add,
}

impl NonTerminal {
    pub fn rule<'a, S: TokenValueSimplicity>(
        &self,
        (token, value): (Token<'a>, &TokenValue<'a, S>),
    ) -> Result<Symbol<'a>, ContextError<'a>> {
        match self {
            Self::Add => {
                if matches!(value, TokenValue::Punctuation(Punctuation::Add)) {
                    Ok(Symbol::Terminal(token))
                } else {
                    Err(todo!())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol<'a> {
    Terminal(Token<'a>),
    NonTerminal(NonTerminal),
}

// Productions

pub struct Rule<'a> {
    head: Symbol<'a>,
    body: Vec<Symbol<'a>>,
}
