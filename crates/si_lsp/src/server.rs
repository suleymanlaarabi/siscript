#![forbid(unsafe_code)]

use tokio::io::{stdin, stdout};
use tower_lsp::{LspService, Server};

use crate::backend::Backend;

pub async fn run_stdio() {
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin(), stdout(), socket).serve(service).await;
}
