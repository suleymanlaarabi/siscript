#![forbid(unsafe_code)]

use crate::id::FileId;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub id: FileId,
    pub path: String,
    pub text: Arc<String>,
}

impl SourceFile {
    pub fn new(id: FileId, path: impl Into<String>, text: impl Into<String>) -> Self {
        Self { id, path: path.into(), text: Arc::new(text.into()) }
    }

    pub fn with_arc(id: FileId, path: impl Into<String>, text: Arc<String>) -> Self {
        Self { id, path: path.into(), text }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_file_stores_identity_path_and_text() {
        let file = SourceFile::new(FileId::new(7), "main.si", "fn main() {}");

        assert_eq!(file.id, FileId::new(7));
        assert_eq!(file.path, "main.si");
        assert_eq!(&*file.text, "fn main() {}");
    }
}
