#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_symbols.si").unwrap()
}

#[tokio::test]
async fn test_document_symbols() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        struct Point { x: i32 }
        fn main() {}
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .document_symbol(DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
    if let DocumentSymbolResponse::Nested(symbols) = res.unwrap() {
        let names: Vec<String> = symbols.into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"Point".to_string()));
        assert!(names.contains(&"main".to_string()));
    } else {
        panic!("expected nested symbols");
    }
}
