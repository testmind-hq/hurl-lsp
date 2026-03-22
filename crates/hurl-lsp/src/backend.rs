use crate::{
    completion::completions,
    diagnostics::{collect_diagnostics, parse_document, ParsedDocument},
    formatting::format_document,
    hover::hover,
    symbols::document_symbols,
};
use dashmap::DashMap;
use std::sync::Arc;
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
        let diagnostics = collect_diagnostics(text);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn parsed_document(&self, uri: &Url) -> Option<ParsedDocument> {
        self.document_text(uri).map(|text| parse_document(&text))
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

        Ok(Some(CompletionResponse::Array(completions(
            &text,
            params.text_document_position.position,
        ))))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.document_text(&uri) else {
            return Ok(None);
        };

        Ok(hover(&text, position))
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
        let Some(parsed) = self.parsed_document(&params.text_document.uri) else {
            return Ok(None);
        };

        Ok(Some(DocumentSymbolResponse::Nested(document_symbols(
            &parsed,
        ))))
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
