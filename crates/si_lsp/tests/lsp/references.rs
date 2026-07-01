#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_refs.si").unwrap()
}

#[tokio::test]
async fn test_references_for_locals() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        fn main() {
            let x = 10;
            let y = x;
            let z = x + 1;
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .references(ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 16), // 'x' definition
            },
            context: ReferenceContext { include_declaration: true },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
    let refs = res.unwrap();
    assert!(refs.len() >= 3);
}
