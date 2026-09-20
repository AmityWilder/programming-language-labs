//! Syntax (not semantic) highlighting

use crate::{
    grammar::style::{Style, Styled},
    scanner::{
        CharLiteral, InterpolatedString, RemappedEscapes, RemappedReplacements, StringLiteral,
        TokenResult, TokenType, TokenValue,
    },
};
use std::range::Range;

pub mod style;

/// Forces me to make both correctly
macro_rules! syntaxes {
    (
        $(#[$smeta:meta])*
        $svis:vis struct $Struct:ident;

        $(#[$emeta:meta])*
        $evis:vis enum $Enum:ident {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident {
                    $(#[$fmeta:meta])*
                    $field:ident
                }
            ),* $(,)?
        }
    ) => {
        $(#[$smeta])*
        $svis struct $Struct {$(
            $(#[$fmeta])*
            pub $field: Style,
        )*}

        $(#[$emeta])*
        $evis enum $Enum {$(
            $(#[$vmeta])*
            $Variant,
        )*}

        impl std::ops::Index<$Enum> for $Struct {
            type Output = Style;

            fn index(&self, index: $Enum) -> &Self::Output {
                match index {
                    $($Enum::$Variant => &self.$field,)*
                }
            }
        }
    };
}

syntaxes! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SyntaxStyle;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub enum Syntax {
        #[default]
        Normal { normal },
        Comment { comment },
        NumberLiteral { number_literal },
        CharLiteral { char_literal },
        StringLiteral { string_literal },
        InterpStrLiteral { interp_str_literal },
        EscapeSeq { escape_seq },
        InterpExpr { interp_expr },
        Variable { variable },
        Constant { constant },
        Callable { callable },
        Keyword { keyword },
        CtrlKeyword { ctrl_keyword },
        Invalid { invalid },
    }
}

impl SyntaxStyle {
    pub fn stylize<T>(&self, syntax: Syntax, what: T) -> Styled<T> {
        Styled {
            style: self[syntax],
            inner: what,
        }
    }
}

fn syntax_of<'a: 'b, 'b, T>(
    item: &'b TokenResult<'a, T>,
) -> (&'a str, Syntax, Option<&'b TokenValue<'a, T>>) {
    match item {
        Ok((token, value)) => (
            token.src,
            match token.ty {
                TokenType::Whitespace | TokenType::Punctuation => Syntax::Normal,
                TokenType::Comment => Syntax::Comment,
                TokenType::NumberLiteral => Syntax::NumberLiteral,
                TokenType::CharLiteral => Syntax::CharLiteral,
                TokenType::StringLiteral => Syntax::StringLiteral,
                TokenType::InterpolatedString => Syntax::InterpStrLiteral,
                TokenType::Identifier => Syntax::Variable, // TODO: distinguish from constants
                TokenType::Callable => Syntax::Callable,
                TokenType::Keyword => Syntax::Keyword,
                TokenType::CtrlKeyword => Syntax::CtrlKeyword,
            },
            value.as_ref(),
        ),
        Err(e) => (&e.source[e.range], Syntax::Invalid, None),
    }
}

/// An iterator over sub-tokens within literals
#[derive(Debug, Clone)]
pub enum SimpleLiteralIter<'a> {
    /// The escape within the character literal
    Char(std::iter::Once<Range<usize>>),

    /// Each escape within the character literal
    String(RemappedEscapes<std::iter::Copied<std::slice::Iter<'a, Range<usize>>>>),
}

impl Iterator for SimpleLiteralIter<'_> {
    type Item = (Range<usize>, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Char(iter) => iter.next().map(|x| (x, Syntax::EscapeSeq)),
            Self::String(iter) => iter.next().map(|x| (x, Syntax::EscapeSeq)),
        }
    }
}

/// An iterator over sub-tokens within literals with support for nested expressions
#[derive(Debug, Clone)]
pub enum LiteralIter<'a, T> {
    /// The escape within the character literal
    Char(std::iter::Once<Range<usize>>),

    /// Each escape within the character literal
    String(RemappedEscapes<std::iter::Copied<std::slice::Iter<'a, Range<usize>>>>),

    /// Each escape or expression within the character literal
    ///
    /// Expressions yeild iterators that must be flattened by the outer iterator
    Interp(RemappedReplacements<'a, T>),
}

impl<'a, T, I> Iterator for LiteralIter<'a, T>
where
    &'a T: IntoIterator<IntoIter = I>,
    I: Iterator<Item = &'a TokenResult<'a, !>>,
{
    type Item = HighlightItem<std::iter::Flatten<SimpleHighlight<I>>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Char(iter) => iter
                .next()
                .map(|x| HighlightItem::simple(x, Syntax::EscapeSeq)),

            Self::String(iter) => iter
                .next()
                .map(|x| HighlightItem::simple(x, Syntax::EscapeSeq)),

            Self::Interp(iter) => iter.next().map(|(range, x)| match x {
                Some(iter) => HighlightItem::nested(
                    range,
                    Syntax::InterpExpr,
                    SimpleHighlight::new(iter.into_iter()).flatten(),
                ),
                None => HighlightItem::simple(range, Syntax::EscapeSeq),
            }),
        }
    }
}

/// A conditionally-flattening iterator over subranges
///
/// `I`: The iterator used
#[derive(Debug, Clone)]
pub enum HighlightItem<I: Iterator<Item = (Range<usize>, Syntax)>> {
    Simple(std::iter::Once<(Range<usize>, Syntax)>),

    Nested {
        outer_range: Range<usize>,
        outer_syn: Syntax,
        prev_end: usize,
        iter: std::iter::Peekable<I>,
    },
}

impl<I: Iterator<Item = (Range<usize>, Syntax)>> HighlightItem<I> {
    fn simple(range: Range<usize>, syntax: Syntax) -> Self {
        Self::Simple(std::iter::once((range, syntax)))
    }

    fn nested(outer_range: Range<usize>, outer_syn: Syntax, iter: I) -> Self {
        Self::Nested {
            outer_range,
            outer_syn,
            prev_end: 0,
            iter: iter.peekable(),
        }
    }
}

impl<I> Iterator for HighlightItem<I>
where
    I: Iterator<Item = (Range<usize>, Syntax)>,
{
    type Item = (Range<usize>, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::Simple(ref mut item) => item.next(),

            Self::Nested {
                outer_range,
                outer_syn,
                ref mut prev_end,
                ref mut iter,
            } => iter
                .next_if(|(range, _)| range.start == *prev_end)
                .or_else(|| {
                    iter.peek()
                        .map(|(range, _)| Range::from(*prev_end..range.start))
                        .or_else(|| {
                            let outer_len = outer_range.end - outer_range.start;
                            (*prev_end < outer_len).then_some(Range::from(*prev_end..outer_len))
                        })
                        .map(|range| (range, outer_syn))
                })
                .inspect(|(range, _)| *prev_end = range.end)
                .map(|(range, syn)| {
                    let mapped_range = Range {
                        start: outer_range.start + range.start,
                        end: outer_range.start + range.end,
                    };
                    (mapped_range, syn)
                }),
        }
    }
}

/// Essentially [`std::iter::Map`] but with [`Self::transform`] as the function.
/// (since associated types can't use [`impl Iterator`](https://doc.rust-lang.org/rust-by-example/trait/impl_trait.html)
/// and every closure is a unique type that can't be written explicitly, only `impl FnMut(T) -> U`)
#[derive(Debug, Clone)]
pub struct HighlightLexeme<'a, I> {
    src: &'a str,
    iter: I,
}

impl<'a, I> HighlightLexeme<'a, I> {
    fn new(src: &'a str, iter: I) -> Self {
        Self { src, iter }
    }

    fn transform(&self, (range, syn): (Range<usize>, Syntax)) -> (&'a str, Syntax) {
        (&self.src[range], syn)
    }
}

impl<'a, I> Iterator for HighlightLexeme<'a, I>
where
    I: Iterator<Item = (Range<usize>, Syntax)>,
{
    type Item = (&'a str, Syntax);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|x| self.transform(x))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(|x| self.transform(x))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<I> DoubleEndedIterator for HighlightLexeme<'_, I>
where
    I: DoubleEndedIterator<Item = (Range<usize>, Syntax)>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|x| self.transform(x))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth_back(n).map(|x| self.transform(x))
    }
}

impl<I> ExactSizeIterator for HighlightLexeme<'_, I>
where
    I: ExactSizeIterator<Item = (Range<usize>, Syntax)>,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<I> std::iter::FusedIterator for HighlightLexeme<'_, I> where
    I: std::iter::FusedIterator<Item = (Range<usize>, Syntax)>
{
}

/// Iterates over tokens and yeilds their lexeme and [`Syntax`], without support for recursively nested tokens
#[derive(Debug, Clone)]
pub struct SimpleHighlight<I> {
    tokens: I,
}

impl<I> SimpleHighlight<I> {
    const fn new(tokens: I) -> Self {
        Self { tokens }
    }
}

impl<'a, I> Iterator for SimpleHighlight<I>
where
    I: Iterator<Item = &'a TokenResult<'a, !>>,
{
    type Item = HighlightItem<SimpleLiteralIter<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.next().map(|item| {
            let (lex, syn, val) = syntax_of(item);
            let range = Range::from(0..lex.len());
            match val {
                Some(TokenValue::CharLiteral(CharLiteral {
                    is_escaped: true, ..
                })) => {
                    const DELIM: char = '\'';
                    let iter = SimpleLiteralIter::Char(std::iter::once(Range::from(
                        DELIM.len_utf8()..lex.len() - DELIM.len_utf8(),
                    )));
                    HighlightItem::nested(range, syn, iter)
                }

                Some(TokenValue::StringLiteral(lit @ StringLiteral { escapes, .. }))
                    if !escapes.is_empty() =>
                {
                    const DELIM: &str = "\"";
                    let iter = SimpleLiteralIter::String(lit.remapped_escapes(DELIM));
                    HighlightItem::nested(range, syn, iter)
                }

                Some(TokenValue::InterpolatedString(_)) => {
                    unreachable!("should not be possible to nest interpolated strings")
                }

                _ => HighlightItem::simple(range, syn),
            }
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tokens.size_hint()
    }
}

/// Iterates over tokens and yeilds their lexeme and [`Syntax`]
#[derive(Debug, Clone)]
pub struct Highlight<I> {
    tokens: I,
}

impl<I> Highlight<I> {
    const fn new(tokens: I) -> Self {
        Self { tokens }
    }
}

impl<'a, I, T> Iterator for Highlight<I>
where
    I: Iterator<Item = &'a TokenResult<'a, T>>,
    &'a T: 'a + IntoIterator<Item = &'a TokenResult<'a, !>>,
{
    type Item = HighlightLexeme<'a, HighlightItem<std::iter::Flatten<LiteralIter<'a, T>>>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.tokens.next().map(|item| {
            let (lex, syn, val) = syntax_of(item);
            let range = Range::from(0..lex.len());
            let iter = match val {
                Some(TokenValue::CharLiteral(CharLiteral {
                    is_escaped: true, ..
                })) => {
                    const DELIM: char = '\'';
                    let iter = LiteralIter::Char(std::iter::once(Range::from(
                        DELIM.len_utf8()..lex.len() - DELIM.len_utf8(),
                    )));
                    HighlightItem::nested(range, syn, iter.flatten())
                }

                Some(TokenValue::StringLiteral(lit @ StringLiteral { escapes, .. }))
                    if !escapes.is_empty() =>
                {
                    const DELIM: &str = "\"";
                    let iter = LiteralIter::String(lit.remapped_escapes(DELIM));
                    HighlightItem::nested(range, syn, iter.flatten())
                }

                Some(TokenValue::InterpolatedString(
                    lit @ InterpolatedString {
                        text: StringLiteral { escapes, .. },
                        expressions,
                    },
                )) if !escapes.is_empty() || !expressions.is_empty() => {
                    const DELIM: &str = "`";
                    let iter = LiteralIter::Interp(lit.remapped_replacements(DELIM));
                    HighlightItem::nested(range, syn, iter.flatten())
                }

                _ => HighlightItem::simple(range, syn),
            };
            HighlightLexeme::new(lex, iter)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tokens.size_hint()
    }
}

pub fn highlight<'a, I, T>(tokens: I) -> std::iter::Flatten<Highlight<I::IntoIter>>
where
    I: IntoIterator<Item = &'a TokenResult<'a, T>>,
    &'a T: 'a + IntoIterator<Item = &'a TokenResult<'a, !>>,
{
    Highlight::new(tokens.into_iter()).flatten()
}
