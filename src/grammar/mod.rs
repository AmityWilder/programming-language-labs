//! Syntax (not semantic) highlighting

use crate::{
    grammar::syntax::{Syntax, syntax_of},
    scanner::{
        error::TokenResult,
        token::{Allocated, CharLiteral, Escapes, NoAlloc, TokenValue, TokenValueSimplicity},
    },
};
use std::range::Range;

pub mod style;
pub mod syntax;

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

#[derive(Debug, Clone)]
pub struct SubTokenSyntax<'a, I: Iterator<Item = (Range<usize>, Syntax)>> {
    /// Full lexeme
    lex: &'a str,

    /// Outer syntax
    syn: Syntax,

    /// Iterator over subtokens
    iter: std::iter::Peekable<I>,

    /// Position of the end of the previous subtoken's range
    prev_end: usize,
}

impl<'a, I: Iterator<Item = (Range<usize>, Syntax)>> SubTokenSyntax<'a, I> {
    fn new(lex: &'a str, syn: Syntax, iter: I) -> Self {
        Self {
            lex,
            syn,
            iter: iter.peekable(),
            prev_end: 0,
        }
    }
}

impl<'a, I> Iterator for SubTokenSyntax<'a, I>
where
    I: Iterator<Item = (Range<usize>, Syntax)>,
{
    type Item = (&'a str, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next_if(|(range, _)| range.start == self.prev_end)
            .or_else(|| {
                self.iter
                    .peek()
                    .map(|(range, _)| range.start)
                    .or_else(|| (self.prev_end < self.lex.len()).then_some(self.lex.len()))
                    .map(|end| (Range::from(self.prev_end..end), self.syn))
            })
            .inspect(|(range, _)| self.prev_end = range.end)
            .map(|(range, syn)| (&self.lex[range], syn))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.iter.size_hint();
        // min: first item starts at 0, last item ends at len, and every item immediately follows the previous one ([item,*])
        // absolute min: there are no items, so lex is emitted exactly once
        // max: first item starts > 0, last item ends < len, and every item is separated ([(pre, item,)* post])
        (
            lo.max(usize::from(self.prev_end < self.lex.len())),
            hi.and_then(|n| n.checked_mul(2)?.checked_add(1)),
        )
    }
}

#[cfg(test)]
mod subtoken_syn_tests {
    use super::*;

    #[test]
    fn test0() {
        let lex = r#""apple \n orange""#;
        let list = SubTokenSyntax::new(
            lex,
            Syntax::StringLiteral,
            [(Range::from(7..9), Syntax::EscapeSeq)].into_iter(),
        )
        .collect::<Vec<_>>();
        assert_eq!(
            list.as_slice(),
            &[
                ("\"apple ", Syntax::StringLiteral),
                ("\\n", Syntax::EscapeSeq),
                (" orange\"", Syntax::StringLiteral)
            ]
        );
    }
}

#[derive(Debug, Clone)]
pub struct EscapedRanges<I> {
    iter: I,
}

impl<I> EscapedRanges<I> {
    const fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I> Iterator for EscapedRanges<I>
where
    I: Iterator<Item = Range<usize>>,
{
    type Item = (Range<usize>, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|range| (remap_subtoken_range(range), Syntax::EscapeSeq))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub trait Highlighting: TokenValueSimplicity {
    type Escaped<'a: 'b, 'b>: 'b + Iterator<Item = (&'a str, Syntax)>;

    fn escaped_str_literal<'a, 'b>(
        lex: &'a str,
        syn: Syntax,
        literal: &'b Self::StringLiteral<'a>,
    ) -> Self::Escaped<'a, 'b>;
}

impl Highlighting for Allocated {
    type Escaped<'a: 'b, 'b> =
        SubTokenSyntax<'a, EscapedRanges<std::iter::Copied<std::slice::Iter<'b, Range<usize>>>>>;

    fn escaped_str_literal<'a, 'b>(
        lex: &'a str,
        syn: Syntax,
        literal: &'b Self::StringLiteral<'a>,
    ) -> Self::Escaped<'a, 'b> {
        SubTokenSyntax::new(
            lex,
            syn,
            EscapedRanges::new(literal.escapes.iter().copied()),
        )
    }
}

#[derive(Debug, Clone)]
pub struct EscapeRanges<'a> {
    iter: Escapes<'a>,
}

impl<'a> EscapeRanges<'a> {
    const fn new(iter: Escapes<'a>) -> Self {
        Self { iter }
    }
}

impl Iterator for EscapeRanges<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(Result::ok).map(|(r, _)| r)
    }
}

impl Highlighting for NoAlloc {
    type Escaped<'a: 'b, 'b> = SubTokenSyntax<'a, EscapedRanges<EscapeRanges<'b>>>;

    fn escaped_str_literal<'a, 'b>(
        lex: &'a str,
        syn: Syntax,
        literal: &'b Self::StringLiteral<'a>,
    ) -> Self::Escaped<'a, 'b> {
        SubTokenSyntax::new(
            lex,
            syn,
            EscapedRanges::new(EscapeRanges::new(Escapes::new(literal))),
        )
    }
}

#[derive(Debug, Clone)]
pub enum HighlightToken<'a: 'b, 'b, H: Highlighting> {
    CharLiteral(std::array::IntoIter<(&'a str, Syntax), 3>),
    StrLiteral(H::Escaped<'a, 'b>),
    Simple(std::iter::Once<(&'a str, Syntax)>),
}

impl<'a: 'b, 'b, H: Highlighting> Iterator for HighlightToken<'a, 'b, H> {
    type Item = (&'a str, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::CharLiteral(iter) => iter.next(),
            Self::StrLiteral(iter) => iter.next(),
            Self::Simple(iter) => iter.next(),
        }
    }

    fn count(self) -> usize {
        match self {
            Self::CharLiteral(iter) => iter.count(),
            Self::StrLiteral(iter) => iter.count(),
            Self::Simple(iter) => iter.count(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::CharLiteral(iter) => iter.size_hint(),
            Self::StrLiteral(iter) => iter.size_hint(),
            Self::Simple(iter) => iter.size_hint(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HighlightIter<I> {
    iter: I,
}

impl<I> HighlightIter<I> {
    const fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<'a: 'b, 'b, H: Highlighting, I: Iterator<Item = &'b TokenResult<'a, H>>> Iterator
    for HighlightIter<I>
{
    type Item = HighlightToken<'a, 'b, H>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|res| {
            let (lex, syn, val) = syntax_of(res);
            match val {
                // char literal with escape - an iterator
                TokenValue::CharLiteral(CharLiteral {
                    is_escaped: true, ..
                }) => HighlightToken::CharLiteral(escaped_char_literal(lex, syn)),

                // string literal with escapes or interpolated string with escapes and no expressions - an iterator
                TokenValue::StringLiteral(literal) if H::has_escapes(literal) => {
                    HighlightToken::StrLiteral(H::escaped_str_literal(lex, syn, literal))
                }

                // an item
                _ => HighlightToken::Simple(std::iter::once((lex, syn))),
            }
        })
    }
}

pub fn highlight<'a: 'b, 'b, H, I>(tokens: I) -> std::iter::Flatten<HighlightIter<I::IntoIter>>
where
    H: Highlighting,
    I: IntoIterator<Item = &'b TokenResult<'a, H>>,
{
    HighlightIter::new(tokens.into_iter()).flatten()
}
