mod backend;
mod completion;
mod definition;
mod diagnostics;
mod formatting;
mod hover;
mod metadata;
mod symbols;
mod syntax;

use backend::Backend;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
