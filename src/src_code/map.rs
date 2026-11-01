use crate::src_code::span::{RelSpan, Span, uspan_to_usize};
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
    PreprocRepl,
    PreprocFile(PathBuf),
}

impl SourceLoc {
    pub const fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::File(path_buf) | Self::PreprocFile(path_buf) => Some(path_buf),
            Self::Repl | Self::PreprocRepl => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExpansionData {
    /// The span of bytes in this file that were expanded from a macro
    pub exp: RelSpan,
    /// The macro call that expanded into `exp`
    pub cal: Span,
    /// The original definition of the macro used in `cal`
    pub def: Span,
    /// The span of the macro argument that was substituted - same file as `def`.
    /// [`None`] if from the macro body instead of an argument
    pub arg: Option<RelSpan>,
}

impl ExpansionData {
    /// The span of `arg`, absolute to its file
    pub const fn arg_span(&self) -> Option<Span> {
        if let Some(rel) = self.arg {
            Some(Span {
                file: self.def.file,
                rel,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SourceCode {
    pub loc: SourceLoc,
    pub src: String,
    /// Macro expansion ranges
    pub mac: Vec<ExpansionData>,
    /// The length of each line in bytes
    pub lines: Vec<usize>,
}

impl SourceCode {
    pub fn new(loc: SourceLoc, src: String) -> Self {
        Self::with_expansion(loc, src, Vec::new())
    }

    pub fn with_expansion(loc: SourceLoc, src: String, mac: Vec<ExpansionData>) -> Self {
        Self {
            loc,
            lines: src.lines().map(str::len).collect(),
            src,
            mac,
        }
    }

    /// Finds the most specific expansion containing `span`
    pub fn expansion(&self, span: RelSpan) -> Option<&ExpansionData> {
        self.mac
            .iter()
            .filter(|x| x.exp.contains(span))
            .reduce(|best, x| if best.exp.contains(x.exp) { x } else { best })
    }

    pub fn get(&self, span: RelSpan) -> Option<&str> {
        self.src.get(<Range<usize>>::from(span))
    }

    /// Returns [`None`] if `pos` is not within the source.
    pub fn charnum_of(&self, pos: usize) -> Option<usize> {
        charnum(&self.src, pos)
    }

    /// Returns [`None`] if `span` is not within the source.
    pub fn charnum_of_span(&self, span: RelSpan) -> Option<Range<usize>> {
        charnum_range(&self.src, span.into())
    }

    /// Returns [`None`] if `pos` is not within the source.
    pub fn linenum_of(&self, pos: usize) -> Option<NonZero<usize>> {
        self.lines
            .iter()
            .copied()
            .position({
                let mut prev_end = 0;
                move |len| {
                    let is_beyond = pos >= prev_end;
                    prev_end = prev_end.strict_add(len);
                    is_beyond
                }
            })
            .map(|n| NonZero::new(n.strict_add(1)).expect("adding 1 should ensure non-zero"))
    }
}

#[derive(Debug)]
pub struct SourceMap {
    /// Cannot be popped or reordered, so [`Span`]s are stable
    files: Vec<SourceCode>,
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMap {
    pub const fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn push(&mut self, code: SourceCode) {
        self.files.push(code);
    }

    pub fn source_of(&self, span: Span) -> &SourceCode {
        self.files
            .get(uspan_to_usize(span.file))
            .expect("span should not refer to a file that does not exist")
    }

    pub fn span_data(&self, span: Span) -> (&SourceCode, &str) {
        let file = self.source_of(span);

        let s = file
            .get(span.rel)
            .expect("span should be a valid range within the file it refers to");

        (file, s)
    }

    pub fn span_str(&self, span: Span) -> &str {
        self.span_data(span).1
    }

    pub fn lookup_macro_recursive(&self, mut span: Span) -> impl Iterator<Item = &ExpansionData> {
        std::iter::from_fn(move || {
            self.source_of(span)
                .expansion(span.rel)
                .inspect(|x| span = x.cal)
        })
    }
}
