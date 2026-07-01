#![forbid(unsafe_code)]

use super::TestClient;
use lsp_types::Url;

fn test_uri() -> Url {
    Url::parse("file:///test_diag.si").unwrap()
}

#[tokio::test]
async fn test_empty_file_no_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    client.open_doc(&uri, "".to_string(), 1).await;
    assert_eq!(client.published_len(&uri), Some(0));
}

#[tokio::test]
async fn test_valid_file_no_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = "fn main() { let x = 10; return x; }".to_string();
    client.open_doc(&uri, code, 1).await;
    assert_eq!(client.published_len(&uri), Some(0));
}

#[tokio::test]
async fn test_syntax_error_publishes_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    client.open_doc(&uri, "fn main( {".to_string(), 1).await;
    assert!(client.published_len(&uri).unwrap_or_default() > 0);
}

#[tokio::test]
async fn test_type_error_publishes_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = "fn main() { let x: i32 = 1; let r = &mut x; }".to_string();
    client.open_doc(&uri, code, 1).await;
    assert!(client.published_len(&uri).unwrap_or_default() > 0);
}

#[tokio::test]
async fn test_borrow_error_publishes_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = r#"
        struct Data { x: i32 }
        fn test(d: &mut Data) {
            let r1 = &mut d.x;
            let r2 = &mut d.x;
        }
    "#
    .to_string();
    client.open_doc(&uri, code, 1).await;
    assert!(client.published_len(&uri).unwrap_or_default() > 0);
}

#[tokio::test]
async fn test_mutation_without_mut_publishes_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    let code = "fn main() { let x = 10; x = 20; }".to_string();
    client.open_doc(&uri, code, 1).await;
    assert!(client.published_len(&uri).unwrap_or_default() > 0);
}

#[tokio::test]
async fn test_change_clears_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    client.open_doc(&uri, "fn main( {".to_string(), 1).await;
    assert!(client.published_len(&uri).unwrap_or_default() > 0);

    client.change_doc(&uri, "fn main() {}".to_string(), 2).await;
    assert_eq!(client.published_len(&uri), Some(0));
}

#[tokio::test]
async fn test_close_clears_diagnostics() {
    let client = TestClient::new();
    let uri = test_uri();
    client.open_doc(&uri, "fn main( {".to_string(), 1).await;
    assert!(client.published_len(&uri).unwrap_or_default() > 0);

    client.close_doc(&uri).await;
    assert_eq!(client.published_len(&uri), Some(0));
}
