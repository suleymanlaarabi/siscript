#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_sem.si").unwrap()
}

#[tokio::test]
async fn test_semantic_tokens_non_empty() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = "fn main() { let x = 10; return x; }".to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .semantic_tokens_full(SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
    if let SemanticTokensResult::Tokens(tokens) = res.unwrap() {
        assert!(!tokens.data.is_empty());
    } else {
        panic!("expected tokens result");
    }
}
