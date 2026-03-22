use crate::{
    code_lens::{
        build_curl_for_entry, code_lenses_with_context, extract_entry_text, COPY_AS_CURL_COMMAND,
        NOOP_COMMAND, RUN_CHAIN_COMMAND, RUN_ENTRY_COMMAND, RUN_ENTRY_WITH_VARS_COMMAND,
        RUN_FILE_COMMAND,
    },
    completion::completions_with_external,
    definition::definition_with_external,
    diagnostics::collect_diagnostics_with_external,
    execution::{execution_diagnostics_for_entry_failure, parse_run_summary, RunSummary},
    formatting::format_document,
    hover::hover_with_external,
    metadata::{infer_entry_dependencies, HurlMetaParser},
    openapi::{
        load_openapi_paths_with_roots, load_openapi_request_body_fields_with_roots,
        load_openapi_response_fields_with_roots,
    },
    symbols::document_symbols,
    syntax::method_from_line,
    variables::{load_workspace_variables_with_roots, pick_variable_file_with_roots},
};
use dashmap::DashMap;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer};
use tracing::{error, info, warn};
use url::Url;

const REQUEST_LOG_PREFIX: &str = "[hurl-request] ";

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
    execution_diagnostics: DashMap<Url, Vec<Diagnostic>>,
    execution_summaries: DashMap<Url, BTreeMap<u32, RunSummary>>,
    workspace_roots: Arc<RwLock<Vec<PathBuf>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DocumentStore::default()),
            execution_diagnostics: DashMap::new(),
            execution_summaries: DashMap::new(),
            workspace_roots: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn document_text(&self, uri: &Url) -> Option<String> {
        self.documents.get(uri)
    }

    async fn workspace_roots(&self) -> Vec<PathBuf> {
        self.workspace_roots.read().await.clone()
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str) {
        let roots = self.workspace_roots().await;
        let external = load_workspace_variables_with_roots(&uri, &roots);
        let external_names: BTreeSet<String> = external.into_iter().map(|item| item.name).collect();
        let mut diagnostics = collect_diagnostics_with_external(text, &external_names);
        if let Some(execution) = self.execution_diagnostics.get(&uri) {
            diagnostics.extend(execution.iter().cloned());
        }
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn log_execution(&self, message: impl Into<String>) {
        let text = message.into();
        info!("{text}");
        self.client.log_message(MessageType::INFO, text).await;
    }

    async fn log_request(&self, message: impl Into<String>) {
        let text = message.into();
        info!("{text}");
        self.client
            .log_message(MessageType::INFO, format!("{REQUEST_LOG_PREFIX}{text}"))
            .await;
    }
}

fn apply_document_change(
    documents: &DocumentStore,
    execution_diagnostics: &DashMap<Url, Vec<Diagnostic>>,
    execution_summaries: &DashMap<Url, BTreeMap<u32, RunSummary>>,
    uri: Url,
    text: String,
) {
    execution_diagnostics.remove(&uri);
    execution_summaries.remove(&uri);
    documents.insert(uri, text);
}

fn apply_run_summary(
    execution_summaries: &DashMap<Url, BTreeMap<u32, RunSummary>>,
    uri: &Url,
    line: u32,
    summary: RunSummary,
) {
    if let Some(mut existing) = execution_summaries.get_mut(uri) {
        existing.insert(line, summary);
    } else {
        let mut value = BTreeMap::new();
        value.insert(line, summary);
        execution_summaries.insert(uri.clone(), value);
    }
}

fn extract_file_text(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let first = lines
        .iter()
        .position(|line| method_from_line(line.trim()).is_some())?;
    let last = lines
        .iter()
        .rposition(|line| method_from_line(line.trim()).is_some())?;
    let mut end = lines.len();
    for (idx, line) in lines.iter().enumerate().skip(last + 1) {
        if method_from_line(line.trim()).is_some() {
            end = idx;
            break;
        }
    }
    Some(lines[first..end].join("\n"))
}

fn extract_chain_text(text: &str, entry_line: usize) -> Option<String> {
    let meta = HurlMetaParser::parse(text);
    let deps = infer_entry_dependencies(text, &meta);
    let parsed = crate::diagnostics::parse_document(text);
    let mut entry_lines: Vec<usize> = parsed
        .entries
        .iter()
        .map(|entry| entry.line as usize)
        .collect();
    entry_lines.sort_unstable();
    if !entry_lines.contains(&entry_line) {
        return None;
    }

    let mut parents = BTreeMap::<usize, BTreeSet<usize>>::new();
    for dep in deps {
        parents
            .entry(dep.to_line as usize)
            .or_default()
            .insert(dep.from_line as usize);
    }

    let mut needed = BTreeSet::<usize>::new();
    let mut queue = VecDeque::from([entry_line]);
    while let Some(line) = queue.pop_front() {
        if !needed.insert(line) {
            continue;
        }
        if let Some(incoming) = parents.get(&line) {
            for parent in incoming {
                queue.push_back(*parent);
            }
        }
    }

    let mut blocks = Vec::new();
    for line in entry_lines {
        if !needed.contains(&line) {
            continue;
        }
        let Some(block) = extract_entry_text(text, line) else {
            continue;
        };
        blocks.push(block);
    }
    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        *self.workspace_roots.write().await = extract_workspace_roots(&params);
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
                    commands: vec![
                        RUN_ENTRY_COMMAND.to_string(),
                        RUN_ENTRY_WITH_VARS_COMMAND.to_string(),
                        RUN_CHAIN_COMMAND.to_string(),
                        RUN_FILE_COMMAND.to_string(),
                        COPY_AS_CURL_COMMAND.to_string(),
                        NOOP_COMMAND.to_string(),
                    ],
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
        info!("hurl-lsp initialized");
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
            apply_document_change(
                &self.documents,
                &self.execution_diagnostics,
                &self.execution_summaries,
                uri.clone(),
                change.text.clone(),
            );
            self.publish_diagnostics(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
        self.execution_diagnostics.remove(&params.text_document.uri);
        self.execution_summaries.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let roots = self.workspace_roots().await;
        let external = load_workspace_variables_with_roots(&uri, &roots);
        let external_names: BTreeSet<String> = external.into_iter().map(|item| item.name).collect();
        let openapi_paths = load_openapi_paths_with_roots(&uri, &roots);
        let openapi_body_fields = load_openapi_request_body_fields_with_roots(&uri, &roots);
        let openapi_response_fields = load_openapi_response_fields_with_roots(&uri, &roots);

        Ok(Some(CompletionResponse::Array(completions_with_external(
            &text,
            params.text_document_position.position,
            &external_names,
            &openapi_paths,
            &openapi_body_fields,
            &openapi_response_fields,
        ))))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let roots = self.workspace_roots().await;
        let external = load_workspace_variables_with_roots(&uri, &roots);
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
        let run_summaries = self
            .execution_summaries
            .get(&uri)
            .map(|item| item.clone())
            .unwrap_or_default();
        Ok(Some(code_lenses_with_context(&uri, &text, &run_summaries)))
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
        let roots = self.workspace_roots().await;
        let external = load_workspace_variables_with_roots(&uri, &roots);

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
        if params.command != RUN_ENTRY_COMMAND
            && params.command != RUN_ENTRY_WITH_VARS_COMMAND
            && params.command != RUN_CHAIN_COMMAND
            && params.command != RUN_FILE_COMMAND
            && params.command != COPY_AS_CURL_COMMAND
        {
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
        let line = arguments
            .get(1)
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(0);
        let verbosity = arguments
            .get(2)
            .and_then(|value| value.as_str())
            .unwrap_or("verbose");
        if params.command == COPY_AS_CURL_COMMAND {
            let Some(text) = self.document_text(&uri) else {
                return Ok(None);
            };
            let Some(curl) = build_curl_for_entry(&text, line) else {
                self.client
                    .show_message(MessageType::WARNING, "Unable to build curl for this entry.")
                    .await;
                return Ok(None);
            };
            self.client
                .show_message(
                    MessageType::INFO,
                    format!("Copy as curl (manual copy):\n{curl}"),
                )
                .await;
            return Ok(Some(serde_json::Value::String(curl)));
        }

        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };
        let run_target = if params.command == RUN_FILE_COMMAND {
            "file"
        } else if params.command == RUN_CHAIN_COMMAND {
            "chain"
        } else {
            "entry"
        };
        let entry_text = if params.command == RUN_FILE_COMMAND {
            let Some(value) = extract_file_text(&text) else {
                self.client
                    .show_message(MessageType::WARNING, "Unable to resolve file run scope.")
                    .await;
                return Ok(None);
            };
            value
        } else if params.command == RUN_CHAIN_COMMAND {
            let Some(value) = extract_chain_text(&text, line) else {
                self.client
                    .show_message(
                        MessageType::WARNING,
                        "Unable to resolve chain for this entry. Try Run file instead.",
                    )
                    .await;
                return Ok(None);
            };
            value
        } else {
            let Some(value) = extract_entry_text(&text, line) else {
                self.client
                    .show_message(
                        MessageType::WARNING,
                        "Unable to resolve request entry for run command.",
                    )
                    .await;
                return Ok(None);
            };
            value
        };
        let temp_file = uri.to_file_path().ok().and_then(|path| {
            path.parent()
                .map(|parent| tempfile::Builder::new().suffix(".hurl").tempfile_in(parent))
        });
        let mut temp = match temp_file.unwrap_or_else(NamedTempFile::new) {
            Ok(file) => file,
            Err(error) => {
                self.client
                    .show_message(
                        MessageType::ERROR,
                        format!("Failed to create temp file: {error}"),
                    )
                    .await;
                return Ok(None);
            }
        };
        if let Err(error) = temp.write_all(entry_text.as_bytes()) {
            self.client
                .show_message(
                    MessageType::ERROR,
                    format!("Failed to write temp file: {error}"),
                )
                .await;
            return Ok(None);
        }
        let path = temp.path().to_path_buf();

        let mut cmd = TokioCommand::new("hurl");
        if verbosity == "very-verbose" {
            cmd.arg("--very-verbose");
        } else {
            cmd.arg("--verbose");
        }
        cmd.arg(&path);
        if params.command == RUN_ENTRY_WITH_VARS_COMMAND {
            let roots = self.workspace_roots().await;
            if let Some(vars_file) = pick_variable_file_with_roots(&uri, &roots) {
                cmd.arg("--variables-file").arg(vars_file);
            } else {
                warn!("no variable file found for {}", uri);
                self.client
                    .show_message(
                        MessageType::WARNING,
                        "No variable file found (.hurl-vars, vars.env, hurl.env, .env). Running without vars file.",
                    )
                    .await;
                self.log_request("no variable file found; running without --variables-file")
                    .await;
            }
        }
        let command_preview = if params.command == RUN_ENTRY_WITH_VARS_COMMAND {
            format!("hurl --{verbosity} <tempfile> [--variables-file <detected>]")
        } else {
            format!("hurl --{verbosity} <tempfile>")
        };
        self.log_request(format!(
            "run target={run_target} uri={} line={} command={}",
            uri, line, command_preview
        ))
        .await;
        self.log_execution(format!(
            "hurl run started ({run_target}) for {} at line {}",
            uri, line
        ))
        .await;

        let output = cmd.output().await;
        match output {
            Ok(output) if output.status.success() => {
                self.execution_diagnostics.remove(&uri);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                apply_run_summary(
                    &self.execution_summaries,
                    &uri,
                    line as u32,
                    parse_run_summary(stderr.as_ref(), stdout.as_ref(), true),
                );
                let message = if stdout.trim().is_empty() {
                    "hurl run succeeded for selected entry.".to_string()
                } else {
                    format!("hurl run succeeded:\n{}", truncate_message(stdout.as_ref()))
                };
                if !stdout.trim().is_empty() {
                    self.log_request(format!("stdout:\n{}", truncate_message(stdout.as_ref())))
                        .await;
                }
                if !stderr.trim().is_empty() {
                    self.log_request(format!("stderr:\n{}", truncate_message(stderr.as_ref())))
                        .await;
                }
                self.log_execution(format!("hurl run succeeded ({run_target}) for {}", uri))
                    .await;
                self.client.show_message(MessageType::INFO, message).await;
                self.publish_diagnostics(uri, &text).await;
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let detail = if stderr.trim().is_empty() {
                    format!("exit status: {}", output.status)
                } else {
                    truncate_message(stderr.as_ref())
                };
                self.execution_diagnostics.insert(
                    uri.clone(),
                    execution_diagnostics_for_entry_failure(&text, line as u32, &detail),
                );
                apply_run_summary(
                    &self.execution_summaries,
                    &uri,
                    line as u32,
                    parse_run_summary(stderr.as_ref(), stdout.as_ref(), false),
                );
                if !stdout.trim().is_empty() {
                    self.log_request(format!("stdout:\n{}", truncate_message(stdout.as_ref())))
                        .await;
                }
                if !stderr.trim().is_empty() {
                    self.log_request(format!("stderr:\n{}", truncate_message(stderr.as_ref())))
                        .await;
                }
                error!("hurl run failed ({run_target}) for {}: {}", uri, detail);
                self.log_execution(format!(
                    "hurl run failed ({run_target}) for {}: {}",
                    uri, detail
                ))
                .await;
                self.client
                    .show_message(MessageType::ERROR, format!("hurl run failed: {detail}"))
                    .await;
                self.publish_diagnostics(uri, &text).await;
            }
            Err(error) => {
                let err_text = error.to_string();
                self.execution_diagnostics.insert(
                    uri.clone(),
                    execution_diagnostics_for_entry_failure(&text, line as u32, &err_text),
                );
                apply_run_summary(
                    &self.execution_summaries,
                    &uri,
                    line as u32,
                    parse_run_summary(&err_text, "", false),
                );
                self.log_request(format!("spawn error:\n{err_text}")).await;
                error!(
                    "failed to execute hurl ({run_target}) for {}: {}",
                    uri, error
                );
                self.log_execution(format!(
                    "failed to execute hurl ({run_target}) for {}: {}",
                    uri, error
                ))
                .await;
                self.client
                    .show_message(
                        MessageType::ERROR,
                        format!("Failed to execute hurl: {error}"),
                    )
                    .await;
                self.publish_diagnostics(uri, &text).await;
            }
        }
        Ok(None)
    }
}

fn truncate_message(input: &str) -> String {
    const MAX_CHARS: usize = 600;
    if input.chars().count() <= MAX_CHARS {
        input.to_string()
    } else {
        let prefix: String = input.chars().take(MAX_CHARS).collect();
        format!("{prefix}...")
    }
}

fn extract_workspace_roots(params: &InitializeParams) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(folders) = &params.workspace_folders {
        for folder in folders {
            if let Ok(path) = folder.uri.to_file_path() {
                roots.push(path);
            }
        }
    }
    if roots.is_empty() {
        if let Some(uri) = &params.root_uri {
            if let Ok(path) = uri.to_file_path() {
                roots.push(path);
            }
        }
    }
    roots
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_document_change_clears_execution_diagnostics() {
        let documents = DocumentStore::default();
        let execution_diagnostics = DashMap::new();
        let execution_summaries = DashMap::new();
        let uri = Url::parse("file:///tmp/test.hurl").expect("uri");
        execution_diagnostics.insert(uri.clone(), vec![Diagnostic::default()]);
        execution_summaries.insert(uri.clone(), BTreeMap::from([(0, RunSummary::default())]));

        apply_document_change(
            &documents,
            &execution_diagnostics,
            &execution_summaries,
            uri.clone(),
            "GET /health".to_string(),
        );

        assert!(execution_diagnostics.get(&uri).is_none());
        assert!(execution_summaries.get(&uri).is_none());
        assert_eq!(documents.get(&uri).as_deref(), Some("GET /health"));
    }

    #[test]
    fn extracts_chain_text_with_dependencies() {
        let text = "# step_id=setup\nPOST /users\nHTTP 201\n[Captures]\nuser_id: jsonpath \"$.id\"\n\n# step_id=test\nGET /users/{{user_id}}\nHTTP 200\n";
        let chain = extract_chain_text(text, 7).expect("chain");
        assert!(chain.contains("POST /users"));
        assert!(chain.contains("GET /users/{{user_id}}"));
    }

    #[test]
    fn extracts_file_text_from_first_request() {
        let text = "# header\n\nGET /health\nHTTP 200\n";
        let file_text = extract_file_text(text).expect("file text");
        assert_eq!(file_text, "GET /health\nHTTP 200");
    }
}
