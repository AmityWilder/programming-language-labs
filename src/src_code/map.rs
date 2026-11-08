use crate::{
    lex::token::TokenKind,
    src_code::{
        code::{ExpansionData, SourceCode, SourceLoc},
        span::{FileId, RelSpan, Span},
    },
};

#[derive(Debug, PartialEq)]
pub struct SourceMap {
    /// Cannot be popped or reordered, so [`Span`]s are stable
    files: Vec<SourceCode>,
}

impl FromIterator<SourceCode> for SourceMap {
    fn from_iter<I: IntoIterator<Item = SourceCode>>(iter: I) -> Self {
        Self {
            files: Vec::from_iter(iter),
        }
    }
}

impl std::ops::Index<FileId> for SourceMap {
    type Output = SourceCode;

    #[track_caller]
    fn index(&self, index: FileId) -> &Self::Output {
        #[expect(
            clippy::indexing_slicing,
            reason = "FileId promises to be a legitimate file index"
        )]
        &self.files[index.into_usize()]
    }
}

impl std::ops::IndexMut<FileId> for SourceMap {
    #[track_caller]
    fn index_mut(&mut self, index: FileId) -> &mut Self::Output {
        #[expect(
            clippy::indexing_slicing,
            reason = "FileId promises to be a legitimate file index"
        )]
        &mut self.files[index.into_usize()]
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
        (0..).map(FileId::new).zip(self.files.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (FileId, &mut SourceCode)> {
        (0..).map(FileId::new).zip(self.files.iter_mut())
    }

    pub fn push(&mut self, code: SourceCode) {
        self.files.push(code);
    }

    pub fn get(&self, id: FileId) -> Option<&SourceCode> {
        self.files.get(id.into_usize())
    }

    pub fn get_mut(&mut self, id: FileId) -> Option<&mut SourceCode> {
        self.files.get_mut(id.into_usize())
    }

    #[track_caller]
    pub fn source_of(&self, span: Span) -> &SourceCode {
        self.files
            .get(span.file.into_usize())
            .expect("span should not refer to a file that does not exist")
    }

    #[track_caller]
    pub fn span_data(&self, span: Span) -> (&SourceCode, &str) {
        let file = self.source_of(span);

        let s = file
            .get(span.rel)
            .expect("span should be a valid range within the file it refers to");

        (file, s)
    }

    #[track_caller]
    pub fn span_str(&self, span: Span) -> &str {
        self.span_data(span).1
    }

    #[track_caller]
    pub fn span_tokens(&self, span: Span) -> impl Iterator<Item = (RelSpan, TokenKind)> {
        self.source_of(span).span_tokens(span.rel)
    }

    pub fn lookup_macro_recursive(&self, mut span: Span) -> impl Iterator<Item = &ExpansionData> {
        std::iter::from_fn(move || {
            self.source_of(span)
                .expansion(span.rel)
                .inspect(|x| span = x.call)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanLoc<'src> {
    source: &'src SourceMap,
    span: Span,
}

impl Span {
    pub const fn display_loc(self, source: &SourceMap) -> SpanLoc<'_> {
        SpanLoc { source, span: self }
    }
}

impl std::fmt::Display for SpanLoc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.source.source_of(self.span);
        match code.loc() {
            SourceLoc::Repl | SourceLoc::PreprocRepl => write!(f, "terminal")?,
            SourceLoc::File(path) | SourceLoc::PreprocFile(path) => path.display().fmt(f)?,
        }
        if matches!(
            code.loc(),
            SourceLoc::PreprocRepl | SourceLoc::PreprocFile(_)
        ) {
            write!(f, " (preprocessed)")?;
        }
        Ok({
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
        })
    }
}
