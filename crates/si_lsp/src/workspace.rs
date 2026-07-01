#![forbid(unsafe_code)]

use dashmap::DashMap;
use lsp_types::Url;
use std::sync::Arc;

use crate::analysis::AnalysisResult;
use crate::document::Document;

#[derive(Debug, Default)]
pub struct Workspace {
    documents: DashMap<Url, Document>,
    analyses: DashMap<Url, Arc<AnalysisResult>>,
}

impl Workspace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&self, uri: Url, text: String, version: Option<i32>) {
        self.documents.insert(uri.clone(), Document::new(uri, version, text));
    }

    pub fn change(
        &self,
        uri: &Url,
        changes: Vec<lsp_types::TextDocumentContentChangeEvent>,
        version: Option<i32>,
    ) {
        if let Some(mut document) = self.documents.get_mut(uri) {
            document.version = version;
            for change in changes {
                document.apply_change(&change);
            }
        } else if let Some(last_change) = changes.last() {
            self.open(uri.clone(), last_change.text.clone(), version);
        }
    }

    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
        self.analyses.remove(uri);
    }

    pub fn get_text(&self, uri: &Url) -> Option<(Arc<String>, Option<i32>)> {
        self.documents.get(uri).map(|document| (document.text(), document.version))
    }

    pub fn set_analysis(&self, uri: Url, analysis: Arc<AnalysisResult>) {
        self.analyses.insert(uri, analysis);
    }

    pub fn get_analysis(&self, uri: &Url) -> Option<Arc<AnalysisResult>> {
        self.analyses.get(uri).map(|analysis| analysis.clone())
    }

    pub fn all_analyses(&self) -> Vec<Arc<AnalysisResult>> {
        self.analyses.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn get_document(&self, uri: &Url) -> Option<Document> {
        self.documents.get(uri).map(|entry| entry.value().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri() -> Url {
        Url::parse("file:///workspace.si").unwrap()
    }

    #[test]
    fn open_change_close_document() {
        let workspace = Workspace::new();
        let uri = uri();

        workspace.open(uri.clone(), "fn main() {}".into(), Some(1));
        assert_eq!(workspace.get_text(&uri).unwrap().0.as_str(), "fn main() {}");

        workspace.change(
            &uri,
            vec![lsp_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "fn main() { return 1; }".into(),
            }],
            Some(2),
        );
        assert_eq!(workspace.get_text(&uri).unwrap().0.as_str(), "fn main() { return 1; }");

        workspace.close(&uri);
        assert_eq!(workspace.get_text(&uri), None);
    }
}
