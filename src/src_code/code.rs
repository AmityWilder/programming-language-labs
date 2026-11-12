use super::span::FileId;
use crate::{
    lex::token::{TokenKind, TokenTable},
    src_code::span::{RelSpan, Span, usize_to_uspan, uspan},
};
use std::{num::NonZero, path::PathBuf, range::Range};

/// # Panics
/// This method may panic in debug if `pos` is not the position of a char in `s` (i.e. within a UTF-8 character).
fn charnum_iter(
    chars: &mut std::iter::Enumerate<std::str::CharIndices<'_>>,
    pos: usize,
) -> Option<usize> {
    for (i, (idx, _)) in chars {
        match idx.cmp(&pos) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => return Some(i),
            std::cmp::Ordering::Greater => {
                // skipped over `pos`, it was not on the previous nor the current boundary, it must be between them.
                debug_assert!(false, "`pos` should be on a UTF-8 char boundary");
                break;
            }
        }
    }
    None
}

/// # Panics
/// This method may panic in debug if `pos` is not the position of a char in `s` (i.e. within a UTF-8 character).
pub fn charnum(s: &str, pos: usize) -> Option<usize> {
    charnum_iter(&mut s.char_indices().enumerate(), pos)
}

/// # Panics
/// This method may panic in debug if `range.start` and `range.end` are not the positions of chars in `s` (i.e. within a UTF-8 character).
pub fn charnum_range(s: &str, range: Range<usize>) -> Option<Range<usize>> {
    let mut it = s.char_indices().enumerate();
    charnum_iter(&mut it, range.start).and_then(|start| {
        if range.end == range.start {
            return Some(Range { start, end: start });
        }
        debug_assert!(range.start < range.end, "range fields should be ordered");
        charnum_iter(&mut it, range.end).map(|end| Range { start, end })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePos {
    pub file: u16,
    pub linenum: NonZero<usize>,
    pub charnum: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct SourceRange {
    pub file: u16,
    pub linenum: RelSpan<NonZero<usize>>,
    pub charnum: RelSpan<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceLoc {
    Repl,
    File(PathBuf),
    PreprocRepl(FileId),
    PreprocFile(FileId, PathBuf),
}

impl SourceLoc {
    pub const fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::File(path_buf) | Self::PreprocFile(_, path_buf) => Some(path_buf),
            Self::Repl | Self::PreprocRepl(_) => None,
        }
    }

    pub const fn original(&self) -> Option<&FileId> {
        match self {
            Self::Repl | Self::File(_) => None,
            Self::PreprocRepl(id) | Self::PreprocFile(id, _) => Some(id),
        }
    }
}

impl std::fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLoc::Repl | SourceLoc::PreprocRepl(_) => f.write_str("repl"),
            SourceLoc::File(path) | SourceLoc::PreprocFile(_, path) => path.display().fmt(f),
        }?;
        if matches!(
            self,
            SourceLoc::PreprocRepl(_) | SourceLoc::PreprocFile(_, _)
        ) {
            f.write_str(" (preprocessed)")?;
        }
        Ok(())
    }
}

/// # Example
/// ```not_code
///          vvvv field.param
/// def \foo($bar) { $bar * 3 }
/// ^^^^^^^^^^^^^^^^^^^^^^^^^^^ def
///
///              vvvvv field.arg
/// let x = \foo{7 + 2};
///         ^^^^^^^^^^^ call
///
/// // expanded from \foo{7 + 2}
///
///         vvvvv exp
/// let x = 7 + 2 * 3;
///              ^^^^ exp (for a different instance with field=None)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpansionData {
    /// The span of bytes in this file that were expanded from a macro.
    pub exp: RelSpan,

    /// The macro call that expanded into `exp`.
    pub call: Span,

    /// The original definition of the macro used in `cal`.
    pub def: Span,

    /// [`None`] if from the macro body instead of an argument.
    pub field: Option<ExpansionField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpansionField {
    /// The span of the macro argument that was substituted.
    pub arg: Span,

    /// The span of the macro parameter that was substituted.
    pub param: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreateSourceCodeError(pub usize);

impl std::error::Error for CreateSourceCodeError {}

impl std::fmt::Display for CreateSourceCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "file too big: cannot exceed {} bytes, found at least {} bytes",
            uspan::MAX,
            self.0
        )
    }
}

#[derive(Debug, Clone)]
pub struct LineSpans<'src> {
    iter: std::iter::Copied<std::slice::Iter<'src, uspan>>,
    head: uspan,
}

impl<'src> LineSpans<'src> {
    fn new(lens: &'src [uspan]) -> Self {
        Self {
            iter: lens.iter().copied(),
            head: 0,
        }
    }
}

impl Iterator for LineSpans<'_> {
    type Item = RelSpan;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|len| {
            let start = self.head;
            self.head = self.head.strict_add(len);
            RelSpan(Range {
                start,
                end: self.head,
            })
        })
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl ExactSizeIterator for LineSpans<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl std::iter::FusedIterator for LineSpans<'_> {}

#[derive(Debug, Clone)]
pub struct LineSpansDe<'src> {
    iter: std::iter::Copied<std::slice::Iter<'src, uspan>>,
    head: uspan,
    tail: uspan,
}

impl<'src> LineSpansDe<'src> {
    fn new(lens: &'src [uspan], full_len: uspan) -> Self {
        Self {
            iter: lens.iter().copied(),
            head: 0,
            tail: full_len,
        }
    }
}

impl Iterator for LineSpansDe<'_> {
    type Item = RelSpan;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|len| {
            let start = self.head;
            self.head = self.head.strict_add(len);
            debug_assert!(
                self.head <= self.tail,
                "head should not exceed tail\
                \n head: {}\
                \n tail: {}",
                self.head,
                self.tail,
            );
            RelSpan(Range {
                start,
                end: self.head,
            })
        })
    }

    fn last(mut self) -> Option<Self::Item> {
        // this iterator does not mutate external state
        self.next_back()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for LineSpansDe<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|len| {
            let end = self.tail;
            debug_assert!(
                end >= len,
                "no token should be longer than the sum of all token lengths up to and including it\
                \n token length: {len}\
                \n total length: {end}"
            );
            #[expect(clippy::arithmetic_side_effects, reason = "see assertion")]
            {
                self.tail -= len;
            }
            debug_assert!(
                self.head <= self.tail,
                "tail should not subceed head\
                \n head: {}\
                \n tail: {}",
                self.head,
                self.tail,
            );
            RelSpan(Range {
                start: self.tail,
                end,
            })
        })
    }
}

impl ExactSizeIterator for LineSpansDe<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl std::iter::FusedIterator for LineSpansDe<'_> {}

#[derive(Debug, Clone)]
pub struct LineContentSpans<'src> {
    iter: std::iter::Copied<std::slice::Iter<'src, uspan>>,
    last: std::option::IntoIter<RelSpan>,
    head: uspan,
}

impl<'src> LineContentSpans<'src> {
    fn new(lens: &'src [uspan], has_trailing_newline: bool, full_len: uspan) -> Self {
        let mut iter = lens.iter().copied();

        let last = if has_trailing_newline {
            None
        } else {
            iter.next_back().map(|len| {
                debug_assert!(
                    len <= full_len,
                    "every `len` should be a subset of `full_len`\
                    \n      len: {len}\
                    \n full_len: {full_len}"
                );
                RelSpan(Range {
                    #[expect(clippy::arithmetic_side_effects, reason = "see assertion")]
                    start: full_len - len,
                    end: full_len,
                })
            })
        }
        .into_iter();

        Self {
            last,
            iter,
            head: 0,
        }
    }

    fn transform(&mut self, len: uspan) -> RelSpan {
        // `char::len_utf8()` is guaranteed to be at most 4, which easily fits in uspan
        const NEWLINE_LEN: uspan = usize_to_uspan('\n'.len_utf8());
        // only proves it CAN contain a newline character, not that it DOES
        debug_assert!(
            len >= NEWLINE_LEN,
            "every line in this branch should contain a newline character\
            \n         len: {len}\
            \n NEWLINE_LEN: {NEWLINE_LEN}"
        );
        let start = self.head;
        self.head = self.head.strict_add(len);
        RelSpan(Range {
            start,
            #[expect(clippy::arithmetic_side_effects, reason = "see assertion")]
            end: self.head - NEWLINE_LEN,
        })
    }
}

impl Iterator for LineContentSpans<'_> {
    type Item = RelSpan;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(len) = self.iter.next() {
            Some(self.transform(len))
        } else {
            self.last.next()
        }
    }

    fn count(self) -> usize {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "this behavior is expected according to the [`Iterator::count`] documentation"
        )]
        (self.iter.count() + self.last.count())
    }

    fn last(mut self) -> Option<Self::Item> {
        // this iterator does not mutate external state
        if let res @ Some(_) = self.last.next() {
            res
        } else {
            self.iter.next_back().map(|l| self.transform(l))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for LineContentSpans<'_> {
    fn len(&self) -> usize {
        debug_assert!(
            self.iter.len().checked_add(self.last.len()).is_some(),
            "`last` can only come from calling `iter.next_back()` in the constructor, \
            so if `iter.len()` is `usize::MAX` ({}), then `last.len()` must be 0, \
            and if `last.len()` is non-zero, then `iter.len()` must be *at most* `usize::MAX - 1` ({}). \
            therefore, the sum of their lengths should never exceed `usize::MAX`\
            \n iter.len(): {}\
            \n last.len(): {}",
            usize::MAX,
            usize::MAX - 1,
            self.iter.len(),
            self.last.len(),
        );
        #[expect(clippy::arithmetic_side_effects, reason = "see assertion")]
        (self.iter.len() + self.last.len())
    }
}

impl std::iter::FusedIterator for LineContentSpans<'_> {}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SourceCode {
    loc: SourceLoc,
    src: String,
    /// Macro expansion ranges
    mac: Vec<ExpansionData>,
    /// The length of each line in bytes (including the trailing `\n`)
    lines: Vec<uspan>,
    has_trailing_newline: bool,
    tokens: Option<TokenTable>,
    preproc: Option<FileId>,
}

impl std::ops::Index<RelSpan> for SourceCode {
    type Output = str;

    #[track_caller]
    fn index(&self, index: RelSpan) -> &Self::Output {
        #[expect(clippy::string_slice, reason = "caller's responsibility")]
        &self.src[<Range<usize>>::from(index)]
    }
}

impl std::ops::IndexMut<RelSpan> for SourceCode {
    #[track_caller]
    fn index_mut(&mut self, index: RelSpan) -> &mut Self::Output {
        #[expect(clippy::string_slice, reason = "caller's responsibility")]
        &mut self.src[<Range<usize>>::from(index)]
    }
}

impl SourceCode {
    pub fn new(loc: SourceLoc, src: String) -> Result<Self, CreateSourceCodeError> {
        Self::with_expansion(loc, src, Vec::new())
    }

    pub fn with_expansion(
        loc: SourceLoc,
        src: String,
        mac: Vec<ExpansionData>,
    ) -> Result<Self, CreateSourceCodeError> {
        let len = src.len();
        if uspan::try_from(len).is_ok() {
            Ok(Self {
                loc,
                lines: src
                    .split_inclusive('\n')
                    .map(str::len)
                    .map(usize_to_uspan)
                    .collect(),
                has_trailing_newline: src.ends_with('\n'),
                src,
                mac,
                tokens: None,
                preproc: None,
            })
        } else {
            Err(CreateSourceCodeError(len))
        }
    }

    pub const fn set_tokens(&mut self, tokens: TokenTable) {
        debug_assert!(self.tokens.is_none(), "should not reassign tokens");
        self.tokens = Some(tokens);
    }

    pub const fn tokens(&self) -> &TokenTable {
        self.tokens
            .as_ref()
            .expect("should not access tokens before initialization")
    }

    pub const fn set_preproc(&mut self, preproc: FileId) {
        debug_assert!(self.preproc.is_none(), "should not reassign preproc");
        self.preproc = Some(preproc);
    }

    /// The ID of the preprocessed version of this file
    pub const fn preproc(&self) -> Option<&FileId> {
        self.preproc.as_ref()
    }

    pub const fn loc(&self) -> &SourceLoc {
        &self.loc
    }

    pub const fn content(&self) -> &str {
        self.src.as_str()
    }

    pub const fn len(&self) -> uspan {
        usize_to_uspan(self.src.len())
    }

    pub const fn line_lens(&self) -> &[uspan] {
        &self.lines
    }

    pub const fn has_trailing_newline(&self) -> bool {
        self.has_trailing_newline
    }

    pub fn line_spans(&self) -> LineSpans<'_> {
        LineSpans::new(&self.lines)
    }

    /// Like [`Self::line_spans`], but excluding the `\n` characters from the span
    pub fn line_content_spans(&self) -> LineContentSpans<'_> {
        LineContentSpans::new(&self.lines, self.has_trailing_newline, self.len())
    }

    /// Finds the most specific expansion containing `span`
    pub fn expansion(&self, span: RelSpan) -> Option<&ExpansionData> {
        self.mac
            .iter()
            .filter(|x| x.exp.contains(span))
            .reduce(|best, x| if best.exp.contains(x.exp) { x } else { best })
    }

    pub const fn expansions(&self) -> &[ExpansionData] {
        self.mac.as_slice()
    }

    pub fn get(&self, span: RelSpan) -> Option<&str> {
        self.src.get(<Range<usize>>::from(span))
    }

    pub fn line_index(&self, pos: uspan) -> Option<uspan> {
        self.lines
            .iter()
            .copied()
            .map({
                let mut end: uspan = 0;
                move |len| {
                    let start = end;
                    end = end.strict_add(len);
                    Range { start, end }
                }
            })
            .position(|range| {
                let res = pos < range.end;
                if res {
                    debug_assert!(range.contains(&pos), "pos < range.end on increasing range should only return true once the range contains pos");
                }
                res
            })
            .map(usize_to_uspan)
    }

    /// Returns [`None`] if `pos` is not within the source.
    pub fn line_char(&self, pos: uspan) -> Option<(NonZero<uspan>, NonZero<uspan>)> {
        if pos == self.len() {
            return Some((
                NonZero::new(usize_to_uspan(self.lines.len()))
                    .expect("lines should not be empty; even an empty string has 1 line of len 0"),
                NonZero::new(
                    usize_to_uspan(
                        self.src
                            .split('\n')
                            .next_back()
                            .expect("should output at least one, even if empty")
                            .chars()
                            .count(),
                    )
                    .strict_add(1),
                )
                .expect("adding 1 should ensure non-zero"),
            ));
        }
        self.lines
            .iter()
            .copied()
            .enumerate()
            .map({
                let mut end: uspan = 0;
                move |(i, len)| {
                    let start = end;
                    end = end.strict_add(len);
                    (usize_to_uspan(i), Range { start, end })
                }
            })
            .find(|(_, range)| pos < range.end)
            .map(|(line_idx, range)| {
                debug_assert!(range.contains(&pos), "pos < range.end on increasing range should only return true once the range contains pos");
                let line_num =
                    NonZero::new(line_idx.strict_add(1)).expect("adding 1 should ensure non-zero");
                let char_num =
                    NonZero::new(usize_to_uspan(self[RelSpan(Range { start: range.start, end: pos })].chars().count()).strict_add(1))
                        .expect("adding 1 should ensure non-zero");
                (line_num, char_num)
            })
    }

    /// NOTE: Includes the WHOLE token, even if the `rel` argument is to a subtoken!
    pub fn span_tokens(&self, rel: RelSpan) -> impl Iterator<Item = (RelSpan, TokenKind)> {
        self.tokens()
            .into_iter()
            .skip_while(move |(span, _)| span.end <= rel.start)
            .take_while(move |(span, _)| span.start < rel.end)
    }
}
