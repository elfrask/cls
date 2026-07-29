use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::{Client, LspService, Server, jsonrpc::Result};
use tower_lsp::lsp_types::*;
use cls_core::frontend::{Lexer, Parser};

pub struct ClsLspBackend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
}

impl ClsLspBackend {
    pub fn new(client: Client) -> Self {
        Self { client, documents: Arc::new(Mutex::new(HashMap::new())) }
    }

    fn kv() -> CompletionItemKind { CompletionItemKind::KEYWORD }
    fn fn_kind() -> CompletionItemKind { CompletionItemKind::FUNCTION }
    fn mod_kind() -> CompletionItemKind { CompletionItemKind::MODULE }

    async fn get_doc(&self, uri: &Url) -> Option<String> {
        self.documents.lock().await.get(uri).cloned()
    }

    async fn send_diags(&self, uri: &Url) {
        let source = self.get_doc(uri).await.unwrap_or_default();
        let diagnostics = Self::run_diagnostics(&source);
        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
    }

    fn run_diagnostics(source: &str) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let mut lexer = Lexer::new(source);
        match lexer.tokenize() {
            Ok(tokens) => {
                let mut parser = Parser::new(tokens);
                if let Err(e) = parser.parse() {
                    Self::add_diag(&mut diags, &e.to_string());
                }
            }
            Err(e) => Self::add_diag(&mut diags, &e.to_string()),
        }
        diags
    }

    fn add_diag(diags: &mut Vec<Diagnostic>, msg: &str) {
        if let Some((line, col)) = cls_core::error::ClsError::extract_line_col(msg) {
            diags.push(Diagnostic {
                range: Range {
                    start: Position { line: line.saturating_sub(1) as u32, character: col.saturating_sub(1) as u32 },
                    end: Position { line: line.saturating_sub(1) as u32, character: col as u32 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: msg.to_string(),
                source: Some("clx".to_string()),
                ..Default::default()
            });
        }
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for ClsLspBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), "\"".into(), "/".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "clx-lsp".into(),
                version: Some("2.0.0".into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        eprintln!("[clx lsp] Servidor LSP listo");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.lock().await.insert(
            params.text_document.uri.clone(),
            params.text_document.text,
        );
        self.send_diags(&params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.lock().await.insert(
                params.text_document.uri.clone(),
                change.text,
            );
            self.send_diags(&params.text_document.uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.lock().await.remove(&params.text_document.uri);
        self.client.publish_diagnostics(params.text_document.uri, vec![], None).await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = Vec::new();
        for kw in &["var", "function", "if", "else", "while", "for", "return",
                     "import", "from", "as", "export", "structure", "interface",
                     "true", "false", "null", "break", "continue", "loop", "switch"] {
            items.push(CompletionItem { label: kw.to_string(), kind: Some(Self::kv()), ..Default::default() });
        }
        for f in &["print", "input", "toString", "int", "float", "str", "bool",
                    "len", "type", "now", "exit", "sleep", "throw"] {
            items.push(CompletionItem { label: f.to_string(), kind: Some(Self::fn_kind()), detail: Some("intrinsic".into()), ..Default::default() });
        }
        for m in &["math", "json", "fs", "http", "Lib"] {
            items.push(CompletionItem { label: m.to_string(), kind: Some(Self::mod_kind()), detail: Some("module".into()), ..Default::default() });
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("CLS Language — .clsx".into())),
            range: None,
        }))
    }

    async fn goto_definition(&self, _: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        Ok(None)
    }

    async fn document_symbol(&self, _: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        Ok(None)
    }
}

pub fn run_server() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| ClsLspBackend::new(client));
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}
