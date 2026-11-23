use super::span::uspan;
use crate::{
    lex::token::TokenKind,
    src_code::{
        code::{ExpansionData, SourceCode},
        span::{FileId, RelSpan, Span, usize_to_uspan},
    },
};

#[derive(Debug, PartialEq)]
pub struct SourceMap {
    /// Cannot be popped or reordered, so [`Span`]s are stable
    files: Vec<SourceCode>,
}

impl FromIterator<SourceCode> for SourceMap {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = SourceCode>,
    {
        Self {
            files: Vec::from_iter(iter),
        }
    }
}

impl std::ops::Index<FileId> for SourceMap {
    type Output = SourceCode;

    #[track_caller]
    fn index(&self, index: FileId) -> &Self::Output {
        let idx = index.into_usize();
        debug_assert!(
            idx < self.files.len(),
            "FileId can only be obtained from a valid file"
        );
        #[expect(clippy::indexing_slicing)]
        &self.files[idx]
    }
}

impl std::ops::IndexMut<FileId> for SourceMap {
    #[track_caller]
    fn index_mut(&mut self, index: FileId) -> &mut Self::Output {
        let idx = index.into_usize();
        debug_assert!(
            idx < self.files.len(),
            "FileId can only be obtained by pushing a valid file, and cannot be invalidated"
        );
        #[expect(clippy::indexing_slicing)]
        &mut self.files[idx]
    }
}

impl std::ops::Index<Span> for SourceMap {
    type Output = str;

    #[track_caller]
    fn index(&self, index: Span) -> &Self::Output {
        self.get(index).expect("Span.rel should be valid")
    }
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

    pub fn iter(&self) -> impl Iterator<Item = (FileId, &SourceCode)> {
        self.files
            .iter()
            .enumerate()
            // SAFETY: a file's ID is its index
            .map(|(i, file)| (unsafe { FileId::new(usize_to_uspan(i)) }, file))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (FileId, &mut SourceCode)> {
        self.files
            .iter_mut()
            .enumerate()
            // SAFETY: a file's ID is its index
            .map(|(i, file)| (unsafe { FileId::new(usize_to_uspan(i)) }, file))
    }

    pub fn push(&mut self, code: SourceCode) -> FileId {
        let id = uspan::try_from(self.files.len()).expect("too many files");
        self.files.push(code);
        // SAFETY: this is the index of the file we just pushed
        unsafe { FileId::new(id) }
    }

    /// Returns [`None`] if the `rel` field of `span` does not refer to a valid range within `file`
    pub fn get(&self, span: Span) -> Option<&str> {
        let Span { file, rel } = span;
        self[file].get(rel)
    }

    #[deprecated = "index with the span file"]
    #[track_caller]
    pub fn source_of(&self, span: Span) -> &SourceCode {
        self.files
            .get(span.file.into_usize())
            .expect("span should not refer to a file that does not exist")
    }

    #[track_caller]
    pub fn span_data(&self, span: Span) -> (&SourceCode, &str) {
        let code = &self[span.file];

        let s = code
            .get(span.rel)
            .expect("span should be a valid range within the file it refers to");

        (code, s)
    }

    #[deprecated = "use indexer"]
    #[track_caller]
    pub fn span_str(&self, span: Span) -> &str {
        self.span_data(span).1
    }

    #[track_caller]
    pub fn span_tokens(&self, span: Span) -> impl Iterator<Item = (RelSpan, TokenKind)> {
        self[span.file].span_tokens(span.rel)
    }

    pub fn lookup_macro_recursive(&self, mut span: Span) -> impl Iterator<Item = &ExpansionData> {
        std::iter::from_fn(move || {
            let Span { file, rel } = span;
            self[file].expansion(rel).inspect(|x| span = x.call)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanLoc<'src> {
    source: &'src SourceCode,
    span: RelSpan,
}

impl RelSpan {
    pub const fn display_loc(self, source: &SourceCode) -> SpanLoc<'_> {
        SpanLoc { source, span: self }
    }
}

impl Span {
    pub fn display_loc(self, source: &SourceMap) -> SpanLoc<'_> {
        let Self { file, rel } = self;
        SpanLoc {
            source: &source[file],
            span: rel,
        }
    }
}

impl std::fmt::Display for SpanLoc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.source;
        write!(f, "{}", code.loc())?;
        let (start_linenum, start_charnum) = code
            .line_char(self.span.start)
            .expect("span should be in file it's from");
        let (end_linenum, end_charnum) = code
            .line_char(self.span.end)
            .expect("span should be in file it's from");
        write!(f, ":{start_linenum}:{start_charnum}")?;
        if end_linenum != start_linenum {
            write!(f, "-{end_linenum}:{end_charnum}")?;
        } else if end_charnum != start_charnum {
            write!(f, "-{end_charnum}")?;
        }
        Ok(())
    }
}
