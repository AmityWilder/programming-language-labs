use crate::src_code::span::{RelSpan, Span, uspan};
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
    /// The span of the macro argument that was substituted - same file as `cal`.
    pub arg: RelSpan,

    /// The span of the macro parameter that was substituted - same file as `def`.
    pub param: RelSpan,
}

impl ExpansionData {
    /// The span of `arg`, absolute to its file
    pub const fn arg(&self) -> Option<Span> {
        if let Some(ExpansionField { arg: rel, .. }) = self.field {
            let file = self.def.file;
            Some(Span { file, rel })
        } else {
            None
        }
    }

    /// The span of `par`, absolute to its file
    pub const fn par(&self) -> Option<Span> {
        if let Some(ExpansionField { param: rel, .. }) = self.field {
            let file = self.call.file;
            Some(Span { file, rel })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreateSourceCodeError;

impl std::error::Error for CreateSourceCodeError {}

impl std::fmt::Display for CreateSourceCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "file too big, cannot exceed {} bytes", uspan::MAX)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SourceCode {
    loc: SourceLoc,
    src: String,
    /// Macro expansion ranges
    mac: Vec<ExpansionData>,
    /// The length of each line in bytes
    lines: Vec<usize>,
}

impl std::ops::Index<RelSpan> for SourceCode {
    type Output = str;

    fn index(&self, index: RelSpan) -> &Self::Output {
        &self.src[<Range<usize>>::from(index)]
    }
}

impl std::ops::IndexMut<RelSpan> for SourceCode {
    fn index_mut(&mut self, index: RelSpan) -> &mut Self::Output {
        &mut self.src[<Range<usize>>::from(index)]
    }
}

impl SourceCode {
    pub fn new(loc: SourceLoc, src: String) -> Result<Self, CreateSourceCodeError> {
        if uspan::try_from(src.len()).is_ok() {
            Ok(Self::with_expansion(loc, src, Vec::new()))
        } else {
            Err(CreateSourceCodeError)
        }
    }

    pub const fn loc(&self) -> &SourceLoc {
        &self.loc
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
