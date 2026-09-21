//! Syntax (not semantic) highlighting

use crate::{
    grammar::syntax::{Syntax, syntax_of},
    scanner::{
        error::{NestedTokenResult, TokenResult},
        symbols::{INTERP_EXPR_CLOSE, INTERP_EXPR_OPEN},
        token::{AllocNested, CharLiteral, InterpolatedString, StringLiteral, TokenValue},
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

fn remap_subtoken_range(Range { start, end }: Range<usize>) -> Range<usize> {
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

pub fn highlight_simple<'a>(
    tokens: &'a [TokenResult<'a, AllocNested>],
) -> impl Iterator<Item = (&'a str, Syntax)> {
    tokens.iter().map(syntax_of).flat_map(|(lex, syn, val)| {
        match val {
            // char literal with escape - an iterator
            TokenValue::CharLiteral(CharLiteral {
                is_escaped: true, ..
            }) => Pick::A(Pick::A(escaped_char_literal(lex, syn))),

            // string literal with escapes or interpolated string with escapes and no expressions - an iterator
            TokenValue::StringLiteral(literal @ StringLiteral { escapes, .. })
            /* | TokenValue::InterpolatedString(InterpolatedString {
                text: literal @ StringLiteral { escapes, .. },
                ..
            }) */
            if !escapes.is_empty() => Pick::A(Pick::B(escaped_str_literal(lex, syn, literal))),

            // an item
            _ => Pick::B(std::iter::once((lex, syn))),
        }
    })
}

pub fn highlight<'a>(
    tokens: &'a [NestedTokenResult<'a>],
) -> impl Iterator<Item = (&'a str, Syntax)> {
    tokens.iter().map(syntax_of).flat_map(|(lex, syn, val)| {
        match val {
            // interpolated string literal with expressions - an iterator of iterators
            TokenValue::InterpolatedString(
                literal @ InterpolatedString { expressions, .. },
            ) if !expressions.is_empty() => {
                let mut prev_end = 0;
                let iter = literal
                    .replacements()
                    .map(|(range, x)| (remap_subtoken_range(range), x))
                    // TODO: this feels wasteful
                    .chain(std::iter::once((
                        Range {
                            start: lex.len(),
                            end: lex.len(),
                        },
                        None,
                    )))
                    .flat_map(move |(range, val)| {
                        std::iter::once((
                            Range::from(
                                std::mem::replace(&mut prev_end, range.end)..range.start,
                            ),
                            syn,
                        )).chain(match val {
                            Some(list) => {
                                let open = (
                                    Range::from(range.start..range.start.checked_add(INTERP_EXPR_OPEN.len())
                                        .expect("interpolated string expression should include delimiters")),
                                    Syntax::InterpExpr,
                                );
                                let mid = highlight_simple(list)
                                        .map(|(s, syn): (&str, Syntax)| (
                                            lex.substr_range(s)
                                                .expect("subtokens should always be a subset of their parent token"),
                                            syn,
                                        ));
                                let close = (
                                    Range::from(
                                        range.end.checked_sub(INTERP_EXPR_CLOSE.len_utf8())
                                            .expect("interpolated string expression should include delimiters")
                                        ..range.end),
                                    Syntax::InterpExpr,
                                );
                                let iter = std::iter::once(open).chain(mid).chain(std::iter::once(close));
                                Pick::A(iter)
                            },

                            None => Pick::B(std::iter::once((range, Syntax::EscapeSeq))),
                        })

                    })
                    .map(|(range, syn)| (&lex[range], syn));
                Pick::A(Pick::B(iter))
            }

            // The rest is essentially [`highlight_simple`]

            // char literal with escape - an iterator
            TokenValue::CharLiteral(CharLiteral {
                is_escaped: true, ..
            }) => {
                Pick::A(Pick::A(escaped_char_literal(lex, syn)))
            }

            // string literal with escapes or interpolated string with escapes and no expressions - an iterator
            TokenValue::StringLiteral(literal @ StringLiteral { escapes, .. })
            | TokenValue::InterpolatedString(InterpolatedString {
                text: literal @ StringLiteral { escapes, .. },
                ..
            }) if !escapes.is_empty() => {
                Pick::B(Pick::A(escaped_str_literal(lex, syn, literal)))
            }

            // an item
            _ => Pick::B(Pick::B(std::iter::once((lex, syn)))),
        }
    })
}
