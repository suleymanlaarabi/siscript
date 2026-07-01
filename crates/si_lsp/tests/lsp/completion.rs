#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_completion.si").unwrap()
}

#[tokio::test]
async fn test_completion_keywords_and_locals() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        fn main() {
            let my_local = 42;
            m
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(3, 13), // position of 'm'
            },
            context: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
    let labels: Vec<String> = match res.unwrap() {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(labels.contains(&"my_local".to_string()));
}
