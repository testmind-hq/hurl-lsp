use crate::{
    code_lens::{code_lenses, NOOP_COMMAND, RUN_ENTRY_COMMAND},
    completion::completions_with_external,
    definition::definition_with_external,
    diagnostics::collect_diagnostics_with_external,
    formatting::format_document,
    hover::hover_with_external,
    symbols::document_symbols,
    variables::load_workspace_variables,
};
use dashmap::DashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer};
use url::Url;

#[derive(Default)]
pub struct DocumentStore {
    docs: DashMap<Url, String>,
}

impl DocumentStore {
    pub fn get(&self, uri: &Url) -> Option<String> {
        self.docs.get(uri).map(|entry| entry.clone())
    }

    pub fn insert(&self, uri: Url, text: String) {
        self.docs.insert(uri, text);
    }

    pub fn remove(&self, uri: &Url) {
        self.docs.remove(uri);
    }
}

pub struct Backend {
    client: Client,
    documents: Arc<DocumentStore>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DocumentStore::default()),
        }
    }

    fn document_text(&self, uri: &Url) -> Option<String> {
        self.documents.get(uri)
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str) {
        let external = load_workspace_variables(&uri);
        let external_names: BTreeSet<String> = external.into_iter().map(|item| item.name).collect();
        let diagnostics = collect_diagnostics_with_external(text, &external_names);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["[".into(), "{".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![RUN_ENTRY_COMMAND.to_string(), NOOP_COMMAND.to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "hurl-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "hurl-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            let uri = params.text_document.uri;
            self.documents.insert(uri.clone(), change.text.clone());
            self.publish_diagnostics(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let external = load_workspace_variables(&uri);
        let external_names: BTreeSet<String> = external.into_iter().map(|item| item.name).collect();

        Ok(Some(CompletionResponse::Array(completions_with_external(
            &text,
            params.text_document_position.position,
            &external_names,
        ))))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let external = load_workspace_variables(&uri);
        let external_map: BTreeMap<String, String> = external
            .into_iter()
            .map(|item| (item.name, item.value))
            .collect();

        Ok(hover_with_external(&text, position, &external_map))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let Some(formatted) = format_document(&text) else {
            return Ok(None);
        };
        if formatted == text {
            return Ok(Some(Vec::new()));
        }

        let end_position = position_for_end(&text);
        Ok(Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), end_position),
            new_text: formatted,
        }]))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(text) = self.document_text(&params.text_document.uri) else {
            return Ok(None);
        };

        Ok(Some(DocumentSymbolResponse::Nested(document_symbols(
            &text,
        ))))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        Ok(Some(code_lenses(&uri, &text)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let text_document_position = params.text_document_position_params;
        let uri = text_document_position.text_document.uri.clone();
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let external = load_workspace_variables(&uri);

        Ok(definition_with_external(
            &uri,
            &text,
            &text_document_position,
            &external,
        ))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        if params.command == NOOP_COMMAND {
            return Ok(None);
        }
        if params.command != RUN_ENTRY_COMMAND {
            return Ok(None);
        }
        let arguments = params.arguments;
        if arguments.is_empty() {
            return Ok(None);
        }
        let Some(uri_value) = arguments.first() else {
            return Ok(None);
        };
        let Some(uri_str) = uri_value.as_str() else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Ok(path) = uri.to_file_path() else {
            self.client
                .show_message(
                    MessageType::ERROR,
                    "Run is only supported for local file documents.",
                )
                .await;
            return Ok(None);
        };

        let output = TokioCommand::new("hurl").arg(&path).output().await;
        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let message = if stdout.trim().is_empty() {
                    format!("hurl run succeeded: {}", path.display())
                } else {
                    format!("hurl run succeeded:\n{}", truncate_message(stdout.as_ref()))
                };
                self.client.show_message(MessageType::INFO, message).await;
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let detail = if stderr.trim().is_empty() {
                    format!("exit status: {}", output.status)
                } else {
                    truncate_message(stderr.as_ref())
                };
                self.client
                    .show_message(MessageType::ERROR, format!("hurl run failed: {detail}"))
                    .await;
            }
            Err(error) => {
                self.client
                    .show_message(
                        MessageType::ERROR,
                        format!("Failed to execute hurl: {error}"),
                    )
                    .await;
            }
        }
        Ok(None)
    }
}

fn truncate_message(input: &str) -> String {
    const MAX: usize = 600;
    if input.len() <= MAX {
        input.to_string()
    } else {
        format!("{}...", &input[..MAX])
    }
}

fn position_for_end(text: &str) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;

    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position::new(line, character)
}
