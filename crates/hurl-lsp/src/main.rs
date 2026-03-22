mod backend;
mod code_lens;
mod completion;
mod definition;
mod diagnostics;
mod execution;
mod formatting;
mod hover;
mod metadata;
mod openapi;
mod symbols;
mod syntax;
mod variables;

use backend::Backend;
use clap::Parser;
use tower_lsp::{LspService, Server};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "hurl-lsp",
    version,
    about = "Language Server Protocol implementation for Hurl"
)]
struct Cli {
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("hurl_lsp={}", cli.log_level)));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
    info!("hurl-lsp process started");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
