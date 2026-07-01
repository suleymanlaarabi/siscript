#![forbid(unsafe_code)]

use lsp_types::*;
use si_lsp::backend::Backend;
use std::sync::Arc;
use tower_lsp::LanguageServer;

pub mod code_actions;
pub mod completion;
pub mod diagnostics;
pub mod goto;
pub mod hover;
pub mod references;
pub mod rename;
pub mod semantic_tokens;
pub mod symbols;

pub struct TestClient {
    pub backend: Arc<Backend>,
}

impl TestClient {
    pub fn new() -> Self {
        Self { backend: Arc::new(Backend::for_tests()) }
    }

    pub async fn open_doc(&self, uri: &Url, text: String, version: i32) {
        self.backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "siscript".into(),
                    version,
                    text,
                },
            })
            .await;
    }

    pub async fn change_doc(&self, uri: &Url, text: String, version: i32) {
        self.backend
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier { uri: uri.clone(), version },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text,
                }],
            })
            .await;
    }

    pub async fn close_doc(&self, uri: &Url) {
        self.backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;
    }

    pub fn published_len(&self, uri: &Url) -> Option<usize> {
        self.backend.published_len(uri)
    }
}
