#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_actions.si").unwrap()
}

#[tokio::test]
async fn test_code_actions_are_empty_without_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = "fn main() { let x = 10; return x; }".to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(0, 0), Position::new(0, 10)),
            context: CodeActionContext { diagnostics: Vec::new(), only: None, trigger_kind: None },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
    let actions = res.unwrap();
    assert!(actions.is_empty());
}
