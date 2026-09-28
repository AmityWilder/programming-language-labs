//! Syntax (not semantic, yet) highlighting

use crate::{
    error::TokenResult,
    highlight::syntax::{Syntax, syntax_of},
    scanner::{
        symbols::{CHAR_DELIM, STR_DELIM},
        token::{CharLiteral, Escapes, StrLiteral, StringLiteral, TokenValue},
    },
};
use std::range::Range;

pub mod style;
pub mod syntax;

/// Remaps input ranges to be offset by the length of a delimiter ([`CHAR_DELIM`]/[`STR_DELIM`])
const fn remap_subtoken_range(Range { start, end }: Range<usize>) -> Range<usize> {
    const ASCII_DELIM_LEN: usize = 1;
    const {
        assert!(
            CHAR_DELIM.len_utf8() == ASCII_DELIM_LEN && STR_DELIM.len_utf8() == ASCII_DELIM_LEN,
            "proof. ASCII_DELIM_LEN is the length of a delimiter"
        );
    }
    Range {
        start: start.strict_add(ASCII_DELIM_LEN),
        end: end.strict_add(ASCII_DELIM_LEN),
    }
}

/// Returns an iterator over char subtokens (assumes the character is escaped)
fn escaped_char_literal(lex: &str, syn: Syntax) -> std::array::IntoIter<(&str, Syntax), 3> {
    let start = lex
        .strip_suffix(CHAR_DELIM)
        .expect("char literal should include delimiters");

    // SAFETY: `start` is a prefix substr `&str` of `lex` (because the suffix was stripped off).
    // By definition, `str` must be valid UTF-8, therefore it will not end partway through a UTF-8
    // character (if it did, then it would not be valid UTF-8). Therefore, `lex.len()` must be the
    // position of a boundary between UTF-8 characters. It is also not out of bounds for `lex`,
    // because `start.len() <= lex.len()`, since `strip_suffix` does not add add characters.
    // So, `start.len()` is AT MOST `lex.len()`, and `s[s.len()..]` for all `s: &str` is valid (it is an empty str
    // at the end of `s`).
    let post = unsafe { lex.get_unchecked(start.len()..) };

    let [pre, inner] = start
        .split_inclusive(CHAR_DELIM)
        .next_chunk::<2>()
        .expect("char literal should include delimiters");

    [(pre, syn), (inner, Syntax::EscapeSeq), (post, syn)].into_iter()
}

/// An iterator over sub-tokens (like escape sequences in char/string literals)
#[derive(Debug, Clone)]
pub struct SubTokenSyntax<'src, I: Iterator<Item = (Range<usize>, Syntax)>> {
    /// Full lexeme
    lex: &'src str,

    /// Outer syntax
    syn: Syntax,

    /// Iterator over subtokens
    iter: std::iter::Peekable<I>,

    /// Position of the end of the previous subtoken's range
    prev_end: usize,
}

impl<'src, I: Iterator<Item = (Range<usize>, Syntax)>> SubTokenSyntax<'src, I> {
    /// Constructs a new [`SubTokenSyntax`]
    fn new(lex: &'src str, syn: Syntax, iter: I) -> Self {
        Self {
            lex,
            syn,
            iter: iter.peekable(),
            prev_end: 0,
        }
    }
}

impl<'src, I> Iterator for SubTokenSyntax<'src, I>
where
    I: Iterator<Item = (Range<usize>, Syntax)>,
{
    type Item = (&'src str, Syntax);

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
            .map(|(range, syn)| {
                (
                    self.lex.get(range).expect("range should be a range in lex"),
                    syn,
                )
            })
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

/// Offsets [`Range`]s by the length of a char/string delimiter and tuples them with [`Syntax::EscapeSeq`]
#[derive(Debug, Clone)]
pub struct EscapedRanges<I> {
    /// The iterator being adapted
    iter: I,
}

impl<I> EscapedRanges<I> {
    /// Constructs a new [`EscapedRanges`]
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

/// An extension to [`StrLiteral`] defining helpers for syntax highlighting
pub trait Highlighting<'src: 'arr, 'arr>: 'arr {
    /// The type returned by [`Self::escaped_str_literal`]
    type Escaped: 'arr + Iterator<Item = (&'src str, Syntax)>;

    /// Returns a syntax iterator over subtokens of a char/string literal
    fn escaped_str_literal(lex: &'src str, syn: Syntax, literal: &'arr Self) -> Self::Escaped;
}

impl<'src: 'arr, 'arr> Highlighting<'src, 'arr> for StringLiteral {
    type Escaped = SubTokenSyntax<
        'src,
        EscapedRanges<std::iter::Copied<std::slice::Iter<'arr, Range<usize>>>>,
    >;

    fn escaped_str_literal(lex: &'src str, syn: Syntax, literal: &'arr Self) -> Self::Escaped {
        SubTokenSyntax::new(
            lex,
            syn,
            EscapedRanges::new(literal.escapes.iter().copied()),
        )
    }
}

/// Not related to [`EscapedRanges`], actually. Just adapts an [`Escapes`] iterator into its non-error ranges.
#[derive(Debug, Clone)]
pub struct EscapeRanges<'src> {
    /// Iterator being adapted
    iter: Escapes<'src>,
}

impl<'src> EscapeRanges<'src> {
    /// Creates a new iterator over [`EscapeRanges`]
    const fn new(iter: Escapes<'src>) -> Self {
        Self { iter }
    }
}

impl Iterator for EscapeRanges<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(Result::ok).map(|(r, _)| r)
    }
}

impl<'src: 'arr, 'arr> Highlighting<'src, 'arr> for StrLiteral<'src> {
    type Escaped = SubTokenSyntax<'src, EscapedRanges<EscapeRanges<'arr>>>;

    fn escaped_str_literal(lex: &'src str, syn: Syntax, literal: &'arr Self) -> Self::Escaped {
        SubTokenSyntax::new(
            lex,
            syn,
            EscapedRanges::new(EscapeRanges::new(Escapes::new(literal.content))),
        )
    }
}

/// An iterator over the subtokens of any valid token, since each has its own method of iterating
#[derive(Debug, Clone)]
pub enum HighlightToken<'src: 'arr, 'arr, H: Highlighting<'src, 'arr>> {
    /// Character literal containing escapes - the open delimiter, the escape sequence, then the close delimiter
    CharLiteral(std::array::IntoIter<(&'src str, Syntax), 3>),
    /// String literal containing escapes - interleaves the escape sequences between un-escaped chunks
    StrLiteral(H::Escaped),
    /// Any token that doesn't have subtokens
    Simple(std::iter::Once<(&'src str, Syntax)>),
}

impl<'src: 'arr, 'arr, H: Highlighting<'src, 'arr>> Iterator for HighlightToken<'src, 'arr, H> {
    type Item = (&'src str, Syntax);

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

/// An iterator over tokens, outputting their lexeme and [Syntax]
#[derive(Debug, Clone)]
pub struct HighlightIter<I> {
    /// Iterator being adapted
    iter: I,
}

impl<I> HighlightIter<I> {
    /// Constructs a new [`HighlightIter`] iterator
    const fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<'src: 'arr, 'arr, I: Iterator<Item = &'arr TokenResult<'src>>> Iterator for HighlightIter<I> {
    type Item = HighlightToken<'src, 'arr, StrLiteral<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|res| {
            let (lex, syn, val) = syntax_of(res);
            match val {
                // char literal with escape - an iterator
                TokenValue::CharLiteral(CharLiteral {
                    is_escaped: true, ..
                }) => HighlightToken::CharLiteral(escaped_char_literal(lex, syn)),

                // string literal with escapes or interpolated string with escapes and no expressions - an iterator
                TokenValue::StringLiteral(literal) if literal.has_escapes() => {
                    HighlightToken::StrLiteral(StrLiteral::escaped_str_literal(lex, syn, literal))
                }

                // an item
                _ => HighlightToken::Simple(std::iter::once((lex, syn))),
            }
        })
    }
}

/// An iterator over each lexeme and [`Syntax`] in the [`TokenResult`] list
pub fn highlight<'src: 'arr, 'arr, I>(tokens: I) -> std::iter::Flatten<HighlightIter<I::IntoIter>>
where
    I: IntoIterator<Item = &'arr TokenResult<'src>>,
{
    HighlightIter::new(tokens.into_iter()).flatten()
}
