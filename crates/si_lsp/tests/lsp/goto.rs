#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_goto.si").unwrap()
}

#[tokio::test]
async fn test_goto_definition_local_variable() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        fn main() {
            let x = 10;
            let y = x;
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    // Goto from 'x' on line 3 (index 20) to definition on line 2 (index 16)
    let res = client
        .backend
        .goto_definition(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(3, 20),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
}

#[tokio::test]
async fn test_goto_definition_struct_fields() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        struct Point { val: i32 }
        fn main() {
            let p = Point { val: 42 };
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    let res = client
        .backend
        .goto_definition(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(3, 29),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
}
