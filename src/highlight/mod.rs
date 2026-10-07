//! Syntax (not semantic, yet) highlighting

use crate::{
    error::{ContextError, ErrorType},
    highlight::{
        style::StyleWrapper,
        syntax::{Syntax, SyntaxStyle},
    },
    scanner::{
        symbols::{CHAR_DELIM, TEXT_DELIM},
        token::{
            Token,
            value::{CharLiteral, Escapes, LexValue, StrLiteral},
        },
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
            CHAR_DELIM.len_utf8() == ASCII_DELIM_LEN && TEXT_DELIM.len_utf8() == ASCII_DELIM_LEN,
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
    assert!(
        lex.starts_with(CHAR_DELIM) && lex.ends_with(CHAR_DELIM),
        "char literal should include delimiters"
    );

    let (pre, inner, post) = lex
        .len()
        .checked_sub(CHAR_DELIM.len_utf8())
        .and_then(|mid| lex.split_at_checked(mid))
        .and_then(|(pre, post)| {
            pre.split_at_checked(CHAR_DELIM.len_utf8())
                .map(|(pre, inner)| (pre, inner, post))
        })
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
        let (end, syn) = if let Some(&(Range { start, end }, syn)) = self.iter.peek() {
            if start == self.prev_end {
                _ = self.iter.next(); // consume it
                // concatenate adjacent, syntax-sharing chunks
                let mut prev_end = end;
                while self
                    .iter
                    .next_if(|&(Range { start, end }, syn1)| {
                        let should_concat = syn1 == syn && start == prev_end;
                        if should_concat {
                            prev_end = end;
                        }
                        should_concat
                    })
                    .is_some()
                {}
                (prev_end, syn)
            } else {
                (start, self.syn)
            }
        } else if self.prev_end < self.lex.len() {
            (self.lex.len(), self.syn)
        } else {
            return None;
        };

        let range = Range {
            start: self.prev_end,
            end,
        };
        self.prev_end = end;

        #[expect(clippy::string_slice, reason = "range should be a range in lex")]
        let lex = &self.lex[range];
        Some((lex, syn))
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
            Syntax::TextLiteral,
            [(Range::from(7..9), Syntax::EscapeSeq)].into_iter(),
        )
        .collect::<Vec<_>>();
        assert_eq!(
            list.as_slice(),
            &[
                ("\"apple ", Syntax::TextLiteral),
                ("\\n", Syntax::EscapeSeq),
                (" orange\"", Syntax::TextLiteral)
            ]
        );
    }
}

const fn escaped_ranges_closure(range: Range<usize>) -> (Range<usize>, Syntax) {
    (remap_subtoken_range(range), Syntax::EscapeSeq)
}

/// Offsets [`Range`]s by the length of a char/string delimiter and tuples them with [`Syntax::EscapeSeq`]
pub type EscapedRanges<I> = std::iter::Map<I, fn(Range<usize>) -> (Range<usize>, Syntax)>;

#[expect(clippy::needless_pass_by_value, reason = "filter_map closure shape")]
const fn escape_ranges_closure(
    item: Result<(Range<usize>, char), ErrorType<'_>>,
) -> Option<Range<usize>> {
    match item {
        Ok((r, _)) => Some(r),
        Err(_) => None,
    }
}

/// Not related to [`EscapedRanges`], actually. Just adapts an [`Escapes`] iterator into its non-error ranges.
pub type EscapeRanges<'src> = std::iter::FilterMap<
    Escapes<'src>,
    fn(Result<(Range<usize>, char), ErrorType<'src>>) -> Option<Range<usize>>,
>;

/// Returns a syntax iterator over subtokens of a text literal
fn escaped_text_literal<'src>(
    lex: &'src str,
    syn: Syntax,
    literal: StrLiteral<'src>,
) -> SubTokenSyntax<'src, EscapedRanges<EscapeRanges<'src>>> {
    SubTokenSyntax::new(
        lex,
        syn,
        #[expect(
            clippy::as_conversions,
            reason = "only way to do this as far as I know"
        )]
        Escapes::new(literal.content)
            .filter_map(
                escape_ranges_closure
                    as fn(Result<(Range<usize>, char), ErrorType<'src>>) -> Option<Range<usize>>,
            )
            .map(escaped_ranges_closure),
    )
}

/// An iterator over the subtokens of any valid token, since each has its own method of iterating
#[derive(Debug, Clone)]
pub enum HighlightToken<'src> {
    /// Character literal containing escapes - the open delimiter, the escape sequence, then the close delimiter
    CharLiteral(std::array::IntoIter<(&'src str, Syntax), 3>),

    /// String literal containing escapes - interleaves the escape sequences between un-escaped chunks
    TextLiteral(SubTokenSyntax<'src, EscapedRanges<EscapeRanges<'src>>>),

    /// Any token that doesn't have subtokens
    Simple(std::iter::Once<(&'src str, Syntax)>),
}

impl<'src> Iterator for HighlightToken<'src> {
    type Item = (&'src str, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::CharLiteral(iter) => iter.next(),
            Self::TextLiteral(iter) => iter.next(),
            Self::Simple(iter) => iter.next(),
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            Self::CharLiteral(iter) => iter.nth(n),
            Self::TextLiteral(iter) => iter.nth(n),
            Self::Simple(iter) => iter.nth(n),
        }
    }

    fn count(self) -> usize {
        match self {
            Self::CharLiteral(iter) => iter.count(),
            Self::TextLiteral(iter) => iter.count(),
            Self::Simple(iter) => iter.count(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::CharLiteral(iter) => iter.size_hint(),
            Self::TextLiteral(iter) => iter.size_hint(),
            Self::Simple(iter) => iter.size_hint(),
        }
    }
}

fn highlight_iter_closure<'src, T>(item: T) -> HighlightToken<'src>
where
    T: TokenHighlight<'src>,
{
    let (lex, syn, val) = item.get_syntax();
    match val {
        // char literal with escape - an iterator
        LexValue::CharLiteral(CharLiteral {
            is_escaped: true, ..
        }) => HighlightToken::CharLiteral(escaped_char_literal(lex, syn)),

        // string literal with escapes or interpolated string with escapes and no expressions - an iterator
        LexValue::TextLiteral(literal) if literal.has_escapes() => {
            HighlightToken::TextLiteral(escaped_text_literal(lex, syn, literal))
        }

        // an item
        _ => HighlightToken::Simple(std::iter::once((lex, syn))),
    }
}

/// An iterator over tokens, outputting their lexeme and [Syntax]
pub type HighlightIter<'src, I> =
    std::iter::FlatMap<I, HighlightToken<'src>, fn(<I as Iterator>::Item) -> HighlightToken<'src>>;

pub trait TokenHighlight<'src> {
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>);
}

impl<'src, T> TokenHighlight<'src> for &T
where
    T: ?Sized + TokenHighlight<'src>,
{
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        (*self).get_syntax()
    }
}

impl<'src> TokenHighlight<'src> for Token<'src> {
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        (self.lex, self.syntax(), self.val)
    }
}

impl<'src> TokenHighlight<'src> for ContextError<'src> {
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        #[expect(clippy::string_slice, reason = "range should be a range of source")]
        (
            &self.source[self.range],
            Syntax::Invalid,
            const { LexValue::Comment },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GenericError;

impl<'src> TokenHighlight<'src> for GenericError {
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        ("[error]", Syntax::Invalid, const { LexValue::Comment })
    }
}

impl<'src, T, E> TokenHighlight<'src> for Result<T, E>
where
    T: TokenHighlight<'src>,
    E: TokenHighlight<'src>,
{
    fn get_syntax(&self) -> (&'src str, Syntax, LexValue<'src>) {
        match self {
            Ok(x) => x.get_syntax(),
            Err(e) => e.get_syntax(),
        }
    }
}

/// An iterator over each lexeme and [`Syntax`] in the list
pub fn highlight<'src, I>(tokens: I) -> HighlightIter<'src, I::IntoIter>
where
    I: IntoIterator<Item: TokenHighlight<'src>>,
{
    #[expect(
        clippy::as_conversions,
        reason = "this is the only way to do this as far as I know"
    )]
    tokens
        .into_iter()
        .flat_map(highlight_iter_closure as fn(I::Item) -> HighlightToken<'src>)
}

pub fn write_highlight<'src, I, T, A>(
    iter: I,
    f: &mut std::fmt::Formatter<'_>,
    syntax_style: &SyntaxStyle<T, A>,
) -> std::fmt::Result
where
    I: IntoIterator<Item: TokenHighlight<'src>>,
    T: StyleWrapper,
    A: AsRef<[T]>,
{
    use std::fmt::Display;
    for (lexeme, syntax) in highlight(iter) {
        syntax_style[syntax].style(lexeme).fmt(f)?;
    }
    f.write_str("\x1b[0m")
}
