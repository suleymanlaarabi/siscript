#![forbid(unsafe_code)]

#[tokio::main]
async fn main() {
    si_lsp::server::run_stdio().await;
}
