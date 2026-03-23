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
mod version;

use backend::Backend;
use tower_lsp::{LspService, Server};
use tracing::info;
use tracing_subscriber::EnvFilter;
use version::display_version;

struct Cli {
    show_help: bool,
    show_version: bool,
    log_level: Option<String>,
}

fn parse_cli_args(args: &[String]) -> Cli {
    let mut cli = Cli {
        show_help: false,
        show_version: false,
        log_level: None,
    };
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "-h" || arg == "--help" {
            cli.show_help = true;
            index += 1;
            continue;
        }
        if arg == "-V" || arg == "--version" {
            cli.show_version = true;
            index += 1;
            continue;
        }
        if let Some(level) = arg.strip_prefix("--log-level=") {
            if !level.trim().is_empty() {
                cli.log_level = Some(level.trim().to_string());
            }
            index += 1;
            continue;
        }
        if arg == "--log-level" {
            if let Some(next) = args.get(index + 1) {
                if !next.starts_with('-') {
                    cli.log_level = Some(next.trim().to_string());
                    index += 2;
                    continue;
                }
            }
            index += 1;
            continue;
        }
        index += 1;
    }
    cli
}

fn print_help() {
    println!("hurl-lsp {}", display_version());
    println!("Language Server Protocol implementation for Hurl");
    println!();
    println!("Usage:");
    println!("  hurl-lsp [--log-level <level>]");
    println!("  hurl-lsp --help");
    println!("  hurl-lsp --version");
    println!();
    println!("Options:");
    println!("  -h, --help               Show this help message");
    println!("  -V, --version            Show version");
    println!("      --log-level <level>  Set server log level (trace|debug|info|warn|error)");
    println!();
    println!("Note: unknown transport args (e.g. --stdio) are ignored for editor compatibility.");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = parse_cli_args(&args);
    if cli.show_help {
        print_help();
        return;
    }
    if cli.show_version {
        println!("hurl-lsp {}", display_version());
        return;
    }

    let env_filter = if let Some(level) = cli.log_level {
        EnvFilter::new(format!("hurl_lsp={level}"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("hurl_lsp=info"))
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_ignores_lsp_transport_arguments() {
        let args = vec![
            "--stdio".to_string(),
            "--clientProcessId".to_string(),
            "100".to_string(),
        ];
        let cli = parse_cli_args(&args);
        assert!(!cli.show_help);
        assert!(!cli.show_version);
        assert!(cli.log_level.is_none());
    }

    #[test]
    fn parser_supports_log_level_argument_forms() {
        let args = vec!["--log-level".to_string(), "debug".to_string()];
        let cli = parse_cli_args(&args);
        assert_eq!(cli.log_level.as_deref(), Some("debug"));

        let args = vec!["--log-level=trace".to_string()];
        let cli = parse_cli_args(&args);
        assert_eq!(cli.log_level.as_deref(), Some("trace"));
    }

    #[test]
    fn parser_supports_help_and_version_flags() {
        let args = vec!["--help".to_string(), "--version".to_string()];
        let cli = parse_cli_args(&args);
        assert!(cli.show_help);
        assert!(cli.show_version);
    }
}
