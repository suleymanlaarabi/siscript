#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_rename.si").unwrap()
}

#[tokio::test]
async fn test_rename_local_variable() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        fn main() {
            let mut my_var = 10;
            my_var = 20;
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .rename(RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 22), // my_var definition
            },
            new_name: "renamed_var".to_string(),
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
    let edit = res.unwrap();
    assert!(edit.changes.is_some());
    let changes = edit.changes.unwrap();
    let document_changes = changes.get(&uri).unwrap();
    assert!(document_changes.len() >= 2);
}
