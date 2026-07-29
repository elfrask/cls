use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::{Client, LspService, Server, jsonrpc::Result};
use tower_lsp::lsp_types::*;
use cls_core::error::{Diagnostic as ClsDiag, Span};
use cls_core::error::diagnostic::Severity;
use cls_core::frontend::{Lexer, Parser};
use cls_core::frontend::ast::*;
use cls_core::frontend::ast::*;
use cls_core::middleware::{TypeChecker, NameResolver};
use cls_core::config::TypesConfig;

/// Símbolo con su ubicación (para go-to-definition)
#[derive(Debug, Clone)]
struct SymEntry {
    name: String,
    span: Span,
    kind: SymKind,
}

#[derive(Debug, Clone)]
enum SymKind {
    Function,
    Variable,
    Parameter,
}

/// Construye tabla de símbolos desde un módulo parsed
fn build_symbols(module: &Module) -> Vec<SymEntry> {
    let mut symbols = Vec::new();
    for stmt in &module.statements {
        match stmt {
            Statement::FunctionDecl(f) => {
                symbols.push(SymEntry {
                    name: f.name.clone(),
                    span: f.span,
                    kind: SymKind::Function,
                });
                for param in &f.params {
                    symbols.push(SymEntry {
                        name: param.name.clone(),
                        span: param.span,
                        kind: SymKind::Parameter,
                    });
                }
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                symbols.push(SymEntry {
                    name: v.name.clone(),
                    span: v.span,
                    kind: SymKind::Variable,
                });
            }
            _ => {}
        }
    }
    symbols
}

/// Backend LSP. El nodo (clx) provee acceso a filesystem via el external hook
/// de ModuleResolver. Core/runtime nunca tocan el disco.
pub struct ClsLspBackend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
    workspace_root: Arc<Mutex<Option<String>>>,
}

impl ClsLspBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            workspace_root: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_doc(&self, uri: &Url) -> Option<String> {
        self.documents.lock().await.get(uri).cloned()
    }

    fn uri_to_path(uri: &Url) -> Option<String> {
        let path = uri.to_file_path().ok()?;
        Some(path.to_string_lossy().to_string())
    }

    /// Pipeline completo sobre el source: lex → parse → name resolve → type check → diagnostics
    pub async fn send_diags(&self, uri: &Url) {
        let source = self.get_doc(uri).await.unwrap_or_default();
        let ws = self.workspace_root.lock().await.clone();
        let diagnostics = Self::run_pipeline(&source, ws.as_deref());
        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
    }

    fn run_pipeline(source: &str, workspace: Option<&str>) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        // 1. Lexer
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR);
                return diags;
            }
        };

        // 2. Parser
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m,
            Err(e) => {
                Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR);
                return diags;
            }
        };

        // 3. Name resolver
        let mut resolver = NameResolver::new();
        if let Err(e) = resolver.resolve(&module) {
            Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR);
        }
        for d in resolver.diagnostics() {
            Self::push_cls_diag(&mut diags, d, DiagnosticSeverity::ERROR);
        }

        // 4. Type checker (config desde cls.json si existe en workspace)
        let types_config = workspace
            .and_then(|root| {
                let path = Path::new(root).join("cls.json");
                if path.exists() {
                    cls_core::config::ModuleManifest::from_file(&path)
                        .ok()
                        .map(|m| m.compiler.types)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| TypesConfig {
                check: true,
                strict: false,
                ..Default::default()
            });

        let mut checker = TypeChecker::new(types_config);
        if let Err(e) = checker.check(&module) {
            Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR);
        }
        for d in checker.diagnostics() {
            let sev = match d.severity {
                Severity::Error => DiagnosticSeverity::ERROR,
                Severity::Warning => DiagnosticSeverity::WARNING,
                _ => DiagnosticSeverity::INFORMATION,
            };
            Self::push_cls_diag(&mut diags, d, sev);
        }

        diags
    }

    fn push_diag(diags: &mut Vec<Diagnostic>, msg: &str, severity: DiagnosticSeverity) {
        if let Some((line, col)) = cls_core::error::ClsError::extract_line_col(msg) {
            diags.push(Diagnostic {
                range: Range {
                    start: Position { line: line.saturating_sub(1) as u32, character: col.saturating_sub(1) as u32 },
                    end: Position { line: line.saturating_sub(1) as u32, character: col as u32 },
                },
                severity: Some(severity),
                message: msg.to_string(),
                source: Some("clx".into()),
                ..Default::default()
            });
        } else if !msg.is_empty() {
            diags.push(Diagnostic {
                range: Range { start: Position::default(), end: Position::default() },
                severity: Some(severity),
                message: msg.to_string(),
                source: Some("clx".into()),
                ..Default::default()
            });
        }
    }

    fn push_cls_diag(diags: &mut Vec<Diagnostic>, d: &ClsDiag, severity: DiagnosticSeverity) {
        diags.push(Diagnostic {
            range: Range {
                start: Position { line: d.span.start_line.saturating_sub(1), character: d.span.start_col.saturating_sub(1) },
                end: Position { line: d.span.end_line.saturating_sub(1), character: d.span.end_col.saturating_sub(1) },
            },
            severity: Some(severity),
            message: d.message.clone(),
            source: Some("clx".into()),
            ..Default::default()
        });
    }

    /// Escanea el workspace en busca de archivos .clsx (para completar imports)
    fn scan_workspace_clsx(root: &str) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    for sub in std::fs::read_dir(&path).unwrap().flatten() {
                        let sp = sub.path();
                        if sp.extension().map(|e| e == "clsx").unwrap_or(false) {
                            files.push(sp.to_string_lossy().to_string());
                        }
                    }
                } else if path.extension().map(|e| e == "clsx").unwrap_or(false) {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
        files
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for ClsLspBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Guardar workspace root para resolucion de archivos
        if let Some(uri) = params.root_uri {
            if let Ok(path) = uri.to_file_path() {
                *self.workspace_root.lock().await = Some(path.to_string_lossy().to_string());
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), "\"".into(), "/".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "clx-lsp".into(),
                version: Some("2.0.0".into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> { Ok(()) }

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

    // ─── Completion con modulos reales + workspace ───────────────────────

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items: Vec<CompletionItem> = Vec::new();

        // Keywords
        for kw in &["var", "function", "if", "else", "while", "for", "return",
                     "import", "from", "as", "export", "structure", "interface",
                     "true", "false", "null", "break", "continue", "loop", "switch"] {
            items.push(CompletionItem { label: kw.to_string(), kind: Some(CompletionItemKind::KEYWORD), ..Default::default() });
        }

        // Intrinsics
        for f in &["print", "input", "toString", "int", "float", "str", "bool",
                    "len", "type", "now", "exit", "sleep", "throw"] {
            items.push(CompletionItem { label: f.to_string(), kind: Some(CompletionItemKind::FUNCTION), detail: Some("intrinsic".into()), ..Default::default() });
        }

        // Modulos internos (siempre disponibles)
        for m in &["math", "json", "fs", "http", "Lib"] {
            items.push(CompletionItem { label: m.to_string(), kind: Some(CompletionItemKind::MODULE), detail: Some("built-in".into()), ..Default::default() });
        }

        // Modulos de usuario desde el workspace (scan por .clsx)
        let ws = self.workspace_root.lock().await.clone();
        if let Some(root) = ws {
            for clsx_path in Self::scan_workspace_clsx(&root) {
                let path = Path::new(&clsx_path);
                let mod_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if !mod_name.is_empty() {
                    items.push(CompletionItem {
                        label: mod_name.to_string(),
                        kind: Some(CompletionItemKind::FILE),
                        detail: Some(format!("workspace/{}", path.display())),
                        ..Default::default()
                    });
                }
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    // ─── Hover ───────────────────────────────────────────────────────────

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = self.get_doc(&uri).await.unwrap_or_default();
        let line = pos.line as usize;
        let col = pos.character as usize;
        let word = extract_word_at(&source, line, col).unwrap_or_default();
        if !word.is_empty() {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(
                    format!("`{}` — CLS symbol", word)
                )),
                range: None,
            }));
        }
        Ok(None)
    }

    // ─── Go-to-definition ────────────────────────────────────────────────

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = self.get_doc(&uri).await.unwrap_or_default();

        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let symbols = build_symbols(&module);
        let line = pos.line as usize;
        let col = pos.character as usize;

        if let Some(word) = extract_word_at(&source, line, col) {
            for sym in &symbols {
                if sym.name == word {
                    let loc = Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position { line: sym.span.start_line.saturating_sub(1), character: sym.span.start_col.saturating_sub(1) },
                            end: Position { line: sym.span.end_line.saturating_sub(1), character: sym.span.end_col.saturating_sub(1) },
                        },
                    };
                    return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                }
            }
        }
        Ok(None)
    }

    // ─── Document symbols ────────────────────────────────────────────────

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let source = self.get_doc(&params.text_document.uri).await.unwrap_or_default();
        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let symbols = build_symbols(&module);
        let info: Vec<SymbolInformation> = symbols.iter().map(|s| {
            let kind = match s.kind {
                SymKind::Function => tower_lsp::lsp_types::SymbolKind::FUNCTION,
                SymKind::Variable | SymKind::Parameter => tower_lsp::lsp_types::SymbolKind::VARIABLE,
            };
            SymbolInformation {
                name: s.name.clone(),
                kind,
                location: Location {
                    uri: params.text_document.uri.clone(),
                    range: Range {
                        start: Position { line: s.span.start_line.saturating_sub(1), character: s.span.start_col.saturating_sub(1) },
                        end: Position { line: s.span.end_line.saturating_sub(1), character: s.span.end_col.saturating_sub(1) },
                    },
                },
                container_name: None,
                tags: None,
                deprecated: None,
            }
        }).collect();
        Ok(Some(DocumentSymbolResponse::Flat(info)))
    }
}

/// Extrae el identificador bajo una posición (line, col) del source
fn extract_word_at(source: &str, line: usize, col: usize) -> Option<String> {
    let line_text = source.lines().nth(line)?;
    if col >= line_text.len() { return None; }
    let chars: Vec<char> = line_text.chars().collect();
    let mut start = col;
    let mut end = col;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    if start < end { Some(chars[start..end].iter().collect()) } else { None }
}

/// Inicia el servidor LSP usando stdin/stdout.
pub fn run_server(silent: bool) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if !silent { eprintln!("[clx lsp] ready"); }
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(|client| ClsLspBackend::new(client));
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}
