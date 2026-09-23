//! Syntax (not semantic) highlighting

use crate::{
    grammar::syntax::{Syntax, syntax_of},
    scanner::{
        error::TokenResult,
        token::{Allocated, CharLiteral, StringLiteral, TokenValue},
    },
};
use std::range::Range;

pub mod style;
pub mod syntax;

/// One of two iterators over the same `Item`
#[derive(Debug, Clone)]
pub enum Pick<I, J> {
    A(I),
    B(J),
}

impl<T, I: Iterator<Item = T>, J: Iterator<Item = T>> Iterator for Pick<I, J> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::A(iter) => iter.next(),
            Self::B(iter) => iter.next(),
        }
    }
}

const fn remap_subtoken_range(Range { start, end }: Range<usize>) -> Range<usize> {
    const ASCII_DELIM_LEN: usize = 1;
    Range {
        start: start.strict_add(ASCII_DELIM_LEN),
        end: end.strict_add(ASCII_DELIM_LEN),
    }
}

fn escaped_char_literal(lex: &str, syn: Syntax) -> std::array::IntoIter<(&str, Syntax), 3> {
    const DELIM: char = '\'';
    let mid1 = DELIM.len_utf8();
    let mid2 = lex
        .len()
        .checked_sub(DELIM.len_utf8())
        .expect("char literal with escape should not be empty");
    [
        (&lex[..mid1], syn),
        (&lex[mid1..mid2], Syntax::EscapeSeq),
        (&lex[mid2..], syn),
    ]
    .into_iter()
}

fn escaped_str_literal<'a>(
    lex: &'a str,
    syn: Syntax,
    literal: &StringLiteral<'a>,
) -> impl Iterator<Item = (&'a str, Syntax)> {
    let mut prev_end = 0;
    literal
        .escapes
        .iter()
        .copied()
        .map(remap_subtoken_range)
        // TODO: this feels wasteful
        .chain(std::iter::once(Range {
            start: lex.len(),
            end: lex.len(),
        }))
        .flat_map(move |range| {
            [
                (
                    Range::from(std::mem::replace(&mut prev_end, range.end)..range.start),
                    syn,
                ),
                (range, Syntax::EscapeSeq),
            ]
        })
        .map(|(range, syn)| (&lex[range], syn))
}

pub fn highlight<'a>(
    tokens: &'a [TokenResult<'a, Allocated>],
) -> impl Iterator<Item = (&'a str, Syntax)> {
    tokens.iter().map(syntax_of).flat_map(|(lex, syn, val)| {
        match val {
            // char literal with escape - an iterator
            TokenValue::CharLiteral(CharLiteral {
                is_escaped: true, ..
            }) => Pick::A(Pick::A(escaped_char_literal(lex, syn))),

            // string literal with escapes or interpolated string with escapes and no expressions - an iterator
            TokenValue::StringLiteral(literal @ StringLiteral { escapes, .. })
                if !escapes.is_empty() =>
            {
                Pick::A(Pick::B(escaped_str_literal(lex, syn, literal)))
            }

            // an item
            _ => Pick::B(std::iter::once((lex, syn))),
        }
    })
}
