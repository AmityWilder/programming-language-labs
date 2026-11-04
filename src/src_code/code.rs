use crate::{
    lex::token::TokenTable,
    src_code::span::{RelSpan, Span, uspan},
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
pub struct CreateSourceCodeError(usize);

impl std::error::Error for CreateSourceCodeError {}

impl std::fmt::Display for CreateSourceCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "file too big: cannot exceed {} bytes, found {} bytes",
            uspan::MAX,
            self.0
        )
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
    tokens: Option<TokenTable>,
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
                lines: src.split_inclusive('\n').map(str::len).collect(),
                src,
                mac,
                tokens: None,
            })
        } else {
            Err(CreateSourceCodeError(len))
        }
    }

    pub const fn set_tokens(&mut self, tokens: TokenTable) {
        assert!(self.tokens.is_none(), "should not reassign tokens");
        self.tokens = Some(tokens);
    }

    pub const fn tokens(&self) -> &TokenTable {
        self.tokens
            .as_ref()
            .expect("should not access tokens before initialization")
    }

    pub const fn loc(&self) -> &SourceLoc {
        &self.loc
    }

    pub const fn content(&self) -> &str {
        self.src.as_str()
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
    pub fn line_char(&self, pos: usize) -> Option<(NonZero<usize>, NonZero<usize>)> {
        if pos == self.src.len() {
            return Some((
                NonZero::new(self.lines.len())
                    .expect("lines should not be empty; even an empty string has 1 line of len 0"),
                NonZero::new(
                    self.src
                        .split('\n')
                        .next_back()
                        .expect("should output at least one, even if empty")
                        .chars()
                        .count()
                        .strict_add(1),
                )
                .expect("adding 1 should ensure non-zero"),
            ));
        }
        self.lines
            .iter()
            .copied()
            .map({
                let mut end: usize = 0;
                move |len| {
                    let start = end;
                    end = end.strict_add(len);
                    Range { start, end }
                }
            })
            .enumerate()
            .find(|(_, range)| range.contains(&pos))
            .map(|(line_idx, range)| {
                let line_num =
                    NonZero::new(line_idx.strict_add(1)).expect("adding 1 should ensure non-zero");
                let char_num =
                    NonZero::new(self.src[range.start..pos].chars().count().strict_add(1))
                        .expect("adding 1 should ensure non-zero");
                (line_num, char_num)
            })
    }
}
