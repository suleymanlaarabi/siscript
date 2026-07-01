#![forbid(unsafe_code)]

use dashmap::DashMap;
use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionOptions, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentHighlight, DocumentHighlightParams, DocumentSymbolParams, DocumentSymbolResponse,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, MessageType, OneOf, ReferenceParams,
    RenameOptions, RenameParams, SemanticTokensParams, SemanticTokensResult, ServerCapabilities,
    SignatureHelp, SignatureHelpParams, SymbolInformation, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkspaceSymbolParams,
};
use tower_lsp::async_trait;
use tower_lsp::{Client, LanguageServer};

use crate::analysis::{AnalysisResult, analyze_source};
use crate::config::LspConfig;
use crate::diagnostics::report_to_lsp;
use crate::document::Document;
use crate::workspace::Workspace;

#[derive(Debug)]
pub struct Backend {
    client: Option<Client>,
    workspace: Workspace,
    config: LspConfig,
    published: DashMap<Url, (Option<i32>, Vec<lsp_types::Diagnostic>)>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self::with_client(Some(client))
    }

    fn with_client(client: Option<Client>) -> Self {
        Self {
            client,
            workspace: Workspace::new(),
            config: LspConfig::default(),
            published: DashMap::new(),
        }
    }

    pub fn for_tests() -> Self {
        Self::with_client(None)
    }

    fn capabilities() -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![
                    ".".to_string(),
                    ":".to_string(),
                    "(".to_string(),
                    "&".to_string(),
                ]),
                work_done_progress_options: Default::default(),
                ..CompletionOptions::default()
            }),
            signature_help_provider: Some(lsp_types::SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                retrigger_characters: Some(vec![",".to_string()]),
                work_done_progress_options: Default::default(),
            }),
            definition_provider: Some(OneOf::Left(true)),
            declaration_provider: Some(lsp_types::DeclarationCapability::Simple(true)),
            references_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(false),
                work_done_progress_options: Default::default(),
            })),
            // semantic_tokens_provider: Some(
            //     SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
            //         work_done_progress_options: Default::default(),
            //         legend: crate::semantic_tokens::legend(),
            //         range: None,
            //         full: Some(SemanticTokensFullOptions::Bool(true)),
            //     }),
            // ),
            document_highlight_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
            ..ServerCapabilities::default()
        }
    }

    async fn analyze_and_publish(&self, uri: Url, text: String, version: Option<i32>) {
        if text.len() > self.config.max_file_size {
            self.publish(uri, Vec::new(), version).await;
            return;
        }

        let uri_clone = uri.clone();
        let text_clone = text.clone();
        let analysis = tokio::task::spawn_blocking({
            let uri_clone = uri_clone.clone();
            let text_clone = text_clone.clone();
            move || analyze_source(&uri_clone, version, std::sync::Arc::new(text_clone))
        })
        .await
        .unwrap_or_else(|_| {
            crate::analysis::AnalysisResult::empty(uri_clone, version, text_clone.len())
        });
        let diagnostics = report_to_lsp(&uri, &text, &analysis.diagnostics);
        let arc_analysis = std::sync::Arc::new(analysis);
        self.workspace.set_analysis(uri.clone(), arc_analysis);
        if self.workspace.get_text(&uri).is_some_and(|(_, current)| current == version) {
            self.publish(uri, diagnostics, version).await;
        }
    }

    async fn document_and_analysis(
        &self,
        params: &TextDocumentPositionParams,
    ) -> Option<(Document, std::sync::Arc<AnalysisResult>)> {
        let uri = &params.text_document.uri;
        let (text, version) = self.workspace.get_text(uri)?;
        let document = Document::with_arc(uri.clone(), version, text.clone());
        let analysis = if let Some(analysis) =
            self.workspace.get_analysis(uri).filter(|analysis| analysis.version == version)
        {
            analysis
        } else {
            let uri_clone = uri.clone();
            let text_clone = text.clone();
            let analysis = tokio::task::spawn_blocking({
                let uri_clone = uri_clone.clone();
                let text_clone = text_clone.clone();
                move || analyze_source(&uri_clone, version, text_clone)
            })
            .await
            .unwrap_or_else(|_| {
                crate::analysis::AnalysisResult::empty(uri_clone, version, text_clone.len())
            });
            let arc_analysis = std::sync::Arc::new(analysis);
            self.workspace.set_analysis(uri.clone(), arc_analysis.clone());
            arc_analysis
        };
        Some((document, analysis))
    }

    async fn publish(
        &self,
        uri: Url,
        diagnostics: Vec<lsp_types::Diagnostic>,
        version: Option<i32>,
    ) {
        self.published.insert(uri.clone(), (version, diagnostics.clone()));
        if let Some(client) = &self.client {
            client.publish_diagnostics(uri, diagnostics, version).await;
        }
    }

    pub fn published_len(&self, uri: &Url) -> Option<usize> {
        self.published.get(uri).map(|entry| entry.value().1.len())
    }
}

#[async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        _params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult { capabilities: Self::capabilities(), server_info: None })
    }

    async fn initialized(&self, _params: InitializedParams) {
        if let Some(client) = &self.client {
            client.log_message(MessageType::INFO, "siscript LSP initialized").await;
        }
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document = params.text_document;
        let uri = document.uri;
        let version = Some(document.version);
        let text = document.text;
        self.workspace.open(uri.clone(), text.clone(), version);
        if self.config.diagnostics_on_open {
            self.analyze_and_publish(uri, text, version).await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = Some(params.text_document.version);
        self.workspace.change(&uri, params.content_changes, version);
        if self.config.diagnostics_on_change {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if let Some((text, current)) = self.workspace.get_text(&uri) {
                if Some(current) == Some(version) {
                    self.analyze_and_publish(uri, text.as_ref().clone(), version).await;
                }
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.workspace.close(&uri);
        self.publish(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        if !self.config.hover {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position_params).await.and_then(
            |(document, analysis)| {
                crate::hover::hover(
                    &document,
                    params.text_document_position_params.position,
                    &analysis,
                )
            },
        ))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        if !self.config.completion {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position).await.and_then(
            |(document, analysis)| {
                crate::completion::completion(
                    &document,
                    params.text_document_position.position,
                    &analysis,
                    &self.config,
                )
            },
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        if !self.config.goto_definition {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position_params).await.and_then(
            |(document, analysis)| {
                crate::goto::goto_definition(
                    &document,
                    params.text_document_position_params.position,
                    &analysis,
                )
            },
        ))
    }

    async fn goto_declaration(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        if !self.config.goto_definition {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position_params).await.and_then(
            |(document, analysis)| {
                crate::goto::goto_declaration(
                    &document,
                    params.text_document_position_params.position,
                    &analysis,
                )
            },
        ))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SignatureHelp>> {
        if !self.config.signature_help {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position_params).await.and_then(
            |(document, analysis)| {
                crate::signature_help::signature_help(
                    &document,
                    params.text_document_position_params.position,
                    &analysis,
                )
            },
        ))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        Ok(self.document_and_analysis(&params.text_document_position_params).await.and_then(
            |(document, analysis)| {
                crate::document_highlight::document_highlight(
                    &document,
                    params.text_document_position_params.position,
                    &analysis,
                )
            },
        ))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        Ok(Some(crate::analysis::workspace_symbols(&self.workspace, &params.query)))
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<lsp_types::Location>>> {
        if !self.config.references {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position).await.and_then(
            |(document, analysis)| {
                crate::references::references(
                    &document,
                    params.text_document_position.position,
                    &analysis,
                    params.context.include_declaration,
                )
            },
        ))
    }

    async fn rename(
        &self,
        params: RenameParams,
    ) -> tower_lsp::jsonrpc::Result<Option<lsp_types::WorkspaceEdit>> {
        if !self.config.rename {
            return Ok(None);
        }
        Ok(self.document_and_analysis(&params.text_document_position).await.and_then(
            |(document, analysis)| {
                crate::rename::rename(
                    &document,
                    params.text_document_position.position,
                    &params.new_name,
                    &analysis,
                    &self.config,
                )
            },
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SemanticTokensResult>> {
        if !self.config.semantic_tokens {
            return Ok(None);
        }
        let Some((text, version)) = self.workspace.get_text(&params.text_document.uri) else {
            return Ok(None);
        };
        let document = Document::with_arc(params.text_document.uri.clone(), version, text.clone());
        let analysis = if let Some(analysis) = self
            .workspace
            .get_analysis(&params.text_document.uri)
            .filter(|analysis| analysis.version == version)
        {
            analysis
        } else {
            let uri_clone = params.text_document.uri.clone();
            let text_clone = text.clone();
            let analysis = tokio::task::spawn_blocking({
                let uri_clone = uri_clone.clone();
                let text_clone = text_clone.clone();
                move || analyze_source(&uri_clone, version, text_clone)
            })
            .await
            .unwrap_or_else(|_| {
                crate::analysis::AnalysisResult::empty(uri_clone, version, text_clone.len())
            });
            let arc_analysis = std::sync::Arc::new(analysis);
            self.workspace.set_analysis(params.text_document.uri.clone(), arc_analysis.clone());
            arc_analysis
        };
        Ok(Some(SemanticTokensResult::Tokens(crate::semantic_tokens::semantic_tokens(
            &document,
            &analysis,
            self.config.fallback_lexer_tokens,
        ))))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        if !self.config.symbols {
            return Ok(None);
        }
        let Some((text, version)) = self.workspace.get_text(&params.text_document.uri) else {
            return Ok(None);
        };
        let document = Document::with_arc(params.text_document.uri.clone(), version, text.clone());
        let analysis =
            if let Some(analysis) = self.workspace.get_analysis(&params.text_document.uri) {
                analysis
            } else {
                let uri_clone = params.text_document.uri.clone();
                let text_clone = text.clone();
                let analysis = tokio::task::spawn_blocking({
                    let uri_clone = uri_clone.clone();
                    let text_clone = text_clone.clone();
                    move || analyze_source(&uri_clone, version, text_clone)
                })
                .await
                .unwrap_or_else(|_| {
                    crate::analysis::AnalysisResult::empty(uri_clone, version, text_clone.len())
                });
                let arc_analysis = std::sync::Arc::new(analysis);
                self.workspace.set_analysis(params.text_document.uri.clone(), arc_analysis.clone());
                arc_analysis
            };
        Ok(crate::symbols::document_symbols(&document, &analysis))
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CodeActionResponse>> {
        if !self.config.code_actions {
            return Ok(None);
        }
        let Some((text, version)) = self.workspace.get_text(&params.text_document.uri) else {
            return Ok(None);
        };
        let analysis =
            if let Some(analysis) = self.workspace.get_analysis(&params.text_document.uri) {
                analysis
            } else {
                let uri_clone = params.text_document.uri.clone();
                let text_clone = text.clone();
                let analysis = tokio::task::spawn_blocking({
                    let uri_clone = uri_clone.clone();
                    let text_clone = text_clone.clone();
                    move || analyze_source(&uri_clone, version, text_clone)
                })
                .await
                .unwrap_or_else(|_| {
                    crate::analysis::AnalysisResult::empty(uri_clone, version, text_clone.len())
                });
                let arc_analysis = std::sync::Arc::new(analysis);
                self.workspace.set_analysis(params.text_document.uri.clone(), arc_analysis.clone());
                arc_analysis
            };
        Ok(crate::code_actions::code_actions(params.range, &analysis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        InitializeParams, TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
        VersionedTextDocumentIdentifier,
    };

    fn uri() -> Url {
        Url::parse("file:///lsp.si").unwrap()
    }

    #[tokio::test]
    async fn initialize_returns_minimal_capabilities() {
        let backend = Backend::for_tests();
        let result = backend.initialize(InitializeParams::default()).await.unwrap();

        assert!(matches!(
            result.capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL))
        ));
        assert!(result.capabilities.hover_provider.is_some());
        assert!(result.capabilities.completion_provider.is_some());
        assert!(result.capabilities.rename_provider.is_some());
    }

    #[tokio::test]
    async fn did_open_valid_file_publishes_no_diagnostics() {
        let backend = Backend::for_tests();
        let uri = uri();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "siscript".into(),
                    version: 1,
                    text: "fn main() { let x = 1; return x; }".into(),
                },
            })
            .await;

        assert_eq!(backend.published_len(&uri), Some(0));
    }

    #[tokio::test]
    async fn did_open_syntax_error_publishes_diagnostics() {
        let backend = Backend::for_tests();
        let uri = uri();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "siscript".into(),
                    version: 1,
                    text: "fn main( {".into(),
                },
            })
            .await;

        assert!(backend.published_len(&uri).unwrap_or_default() > 0);
    }

    #[tokio::test]
    async fn did_change_correction_clears_diagnostics() {
        let backend = Backend::for_tests();
        let uri = uri();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "siscript".into(),
                    version: 1,
                    text: "fn main( {".into(),
                },
            })
            .await;

        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier { uri: uri.clone(), version: 2 },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "fn main() { return 1; }".into(),
                }],
            })
            .await;

        assert_eq!(backend.published_len(&uri), Some(0));
    }

    #[tokio::test]
    async fn did_close_clears_diagnostics() {
        let backend = Backend::for_tests();
        let uri = uri();
        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        assert_eq!(backend.published_len(&uri), Some(0));
    }
}
