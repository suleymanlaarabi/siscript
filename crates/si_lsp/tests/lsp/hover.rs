#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::*;
use tower_lsp::LanguageServer;

fn test_uri() -> Url {
    Url::parse("file:///test_hover.si").unwrap()
}

#[tokio::test]
async fn test_hover_on_let_and_params() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        fn main(arg: i32) {
            let mut x = 10;
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    // Hover on 'arg' param
    let res = client
        .backend
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(1, 17), // 'arg'
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());

    // Hover on 'x' local variable
    let res = client
        .backend
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 22), // 'x'
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
}

#[tokio::test]
async fn test_hover_on_structs_and_enums() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        struct Point { x: i32, y: i32 }
        enum Color { Red, Green }
        fn test() {
            let p = Point { x: 1, y: 2 };
            let c = Color::Red;
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;

    // Hover on 'Point' struct definition
    let res = client
        .backend
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(1, 17),
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());

    // Hover on 'Color' enum variant Red
    let res = client
        .backend
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(5, 29),
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(res.is_some());
}
