use sixu_lsp::create_lsp_service;
use tower_lsp_server::Server;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = create_lsp_service();
    Server::new(stdin, stdout, socket).serve(service).await;
}
