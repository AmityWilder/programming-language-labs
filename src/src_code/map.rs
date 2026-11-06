use crate::src_code::{
    code::{ExpansionData, SourceCode},
    span::{FileId, Span},
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

    pub fn lookup_macro_recursive(&self, mut span: Span) -> impl Iterator<Item = &ExpansionData> {
        std::iter::from_fn(move || {
            self.source_of(span)
                .expansion(span.rel)
                .inspect(|x| span = x.call)
        })
    }
}
