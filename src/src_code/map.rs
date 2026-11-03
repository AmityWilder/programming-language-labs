use crate::src_code::{
    code::{ExpansionData, SourceCode},
    span::{FileId, Span, uspan_to_usize},
};

#[derive(Debug)]
pub struct SourceMap {
    /// Cannot be popped or reordered, so [`Span`]s are stable
    files: Vec<SourceCode>,
}

impl std::ops::Index<FileId> for SourceMap {
    type Output = SourceCode;

    fn index(&self, index: FileId) -> &Self::Output {
        &self.files[index.into_usize()]
    }
}

impl std::ops::IndexMut<FileId> for SourceMap {
    fn index_mut(&mut self, index: FileId) -> &mut Self::Output {
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
        (0..).map(FileId).zip(self.files.iter())
    }

    pub fn push(&mut self, code: SourceCode) {
        self.files.push(code);
    }

    pub fn get(&self, id: FileId) -> Option<&SourceCode> {
        self.files.get(id.into_usize())
    }

    pub fn source_of(&self, span: Span) -> &SourceCode {
        self.files
            .get(uspan_to_usize(span.file.0))
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
                .inspect(|x| span = x.call)
        })
    }
}
