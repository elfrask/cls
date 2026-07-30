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
use cls_core::middleware::{TypeChecker, NameResolver};
use cls_core::config::TypesConfig;
use cls_runtime::{Value, VfsResolver, LocalFs};
use cls_runtime::stdlib::{math, json};
use crate::modules::{fs, http};

#[derive(Debug, Clone)]
struct SymEntry { name: String, span: Span, kind: SymKind }

#[derive(Debug, Clone)]
enum SymKind { Function, Variable, Parameter }

fn build_symbols(module: &Module) -> Vec<SymEntry> {
    let mut symbols = Vec::new();
    for stmt in &module.statements {
        match stmt {
            Statement::FunctionDecl(f) => {
                symbols.push(SymEntry { name: f.name.clone(), span: f.span, kind: SymKind::Function });
                for p in &f.params { symbols.push(SymEntry { name: p.name.clone(), span: p.span, kind: SymKind::Parameter }); }
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                symbols.push(SymEntry { name: v.name.clone(), span: v.span, kind: SymKind::Variable });
            }
            _ => {}
        }
    }
    symbols
}

pub struct ClsLspBackend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
    workspace_root: Arc<Mutex<Option<String>>>,
}

impl ClsLspBackend {
    pub fn new(client: Client) -> Self {
        Self { client, documents: Arc::new(Mutex::new(HashMap::new())), workspace_root: Arc::new(Mutex::new(None)) }
    }

    async fn get_doc(&self, uri: &Url) -> Option<String> {
        self.documents.lock().await.get(uri).cloned()
    }

    pub async fn send_diags(&self, uri: &Url) {
        let source = self.get_doc(uri).await.unwrap_or_default();
        let ws = self.workspace_root.lock().await.clone();
        let diagnostics = Self::run_pipeline(&source, ws.as_deref());
        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
    }

    fn run_pipeline(source: &str, workspace: Option<&str>) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t, Err(e) => { Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR); return diags; }
        };
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m, Err(e) => { Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR); return diags; }
        };
        let mut resolver = NameResolver::new();
        if let Err(e) = resolver.resolve(&module) { Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR); }
        for d in resolver.diagnostics() { Self::push_cls_diag(&mut diags, d, DiagnosticSeverity::ERROR); }
        let types_config = workspace.and_then(|root| {
            let path = Path::new(root).join("cls.json");
            if path.exists() { cls_core::config::ModuleManifest::from_file(&path).ok().map(|m| m.compiler.types) } else { None }
        }).unwrap_or_else(|| TypesConfig { check: true, strict: false, ..Default::default() });
        let mut checker = TypeChecker::new(types_config);
        if let Err(e) = checker.check(&module) { Self::push_diag(&mut diags, &e.to_string(), DiagnosticSeverity::ERROR); }
        for d in checker.diagnostics() {
            let sev = match d.severity { Severity::Error => DiagnosticSeverity::ERROR, Severity::Warning => DiagnosticSeverity::WARNING, _ => DiagnosticSeverity::INFORMATION };
            Self::push_cls_diag(&mut diags, d, sev);
        }
        diags
    }

    fn push_diag(diags: &mut Vec<Diagnostic>, msg: &str, severity: DiagnosticSeverity) {
        if let Some((line, col)) = cls_core::error::ClsError::extract_line_col(msg) {
            diags.push(Diagnostic {
                range: Range { start: Position { line: line.saturating_sub(1) as u32, character: col.saturating_sub(1) as u32 }, end: Position { line: line.saturating_sub(1) as u32, character: col as u32 } },
                severity: Some(severity), message: msg.to_string(), source: Some("clx".into()), ..Default::default()
            });
        } else if !msg.is_empty() {
            diags.push(Diagnostic { range: Range { start: Position::default(), end: Position::default() }, severity: Some(severity), message: msg.to_string(), source: Some("clx".into()), ..Default::default() });
        }
    }

    fn push_cls_diag(diags: &mut Vec<Diagnostic>, d: &ClsDiag, severity: DiagnosticSeverity) {
        diags.push(Diagnostic {
            range: Range { start: Position { line: d.span.start_line.saturating_sub(1), character: d.span.start_col.saturating_sub(1) }, end: Position { line: d.span.end_line.saturating_sub(1), character: d.span.end_col.saturating_sub(1) } },
            severity: Some(severity), message: d.message.clone(), source: Some("clx".into()), ..Default::default()
        });
    }

    fn scan_workspace_clsx(root: &str) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    for sub in std::fs::read_dir(&path).unwrap().flatten() {
                        let sp = sub.path();
                        if sp.extension().map(|e| e == "clsx").unwrap_or(false) { files.push(sp.to_string_lossy().to_string()); }
                    }
                } else if path.extension().map(|e| e == "clsx").unwrap_or(false) { files.push(path.to_string_lossy().to_string()); }
            }
        }
        files
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for ClsLspBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(uri) = params.root_uri {
            if let Ok(path) = uri.to_file_path() {
                *self.workspace_root.lock().await = Some(path.to_string_lossy().to_string());
            }
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions { trigger_characters: Some(vec![".".into(), "\"".into(), "/".into()]), ..Default::default() }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo { name: "clx-lsp".into(), version: Some("2.0.0".into()) }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}
    async fn shutdown(&self) -> Result<()> { Ok(()) }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.lock().await.insert(params.text_document.uri.clone(), params.text_document.text);
        self.send_diags(&params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.lock().await.insert(params.text_document.uri.clone(), change.text);
            self.send_diags(&params.text_document.uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.lock().await.remove(&params.text_document.uri);
        self.client.publish_diagnostics(params.text_document.uri, vec![], None).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let source = self.get_doc(&uri).await.unwrap_or_default();

        let is_member = params.context.as_ref().and_then(|c| c.trigger_character.as_deref()).map(|c| c == ".").unwrap_or(false);
        if is_member {
            return Ok(complete_member(&source, pos));
        }

        let mut items: Vec<CompletionItem> = Vec::new();

        // Scope symbols
        let scope = scope_symbols(&source);
        for (name, kind) in &scope {
            let icon = match kind { SymKind::Function => CompletionItemKind::FUNCTION, _ => CompletionItemKind::VARIABLE };
            items.push(CompletionItem { label: name.clone(), kind: Some(icon), detail: Some("scope".into()), ..Default::default() });
        }

        for kw in &["var", "function", "if", "else", "while", "for", "return", "import", "from", "as", "export", "structure", "interface", "true", "false", "null", "break", "continue", "loop", "switch"] {
            items.push(CompletionItem { label: kw.to_string(), kind: Some(CompletionItemKind::KEYWORD), ..Default::default() });
        }
        for f in &["print", "input", "toString", "int", "float", "str", "bool", "len", "type", "now", "exit", "sleep", "throw"] {
            items.push(CompletionItem { label: f.to_string(), kind: Some(CompletionItemKind::FUNCTION), detail: Some("intrinsic".into()), ..Default::default() });
        }
        for m in &["math", "json", "fs", "http", "Lib"] {
            items.push(CompletionItem { label: m.to_string(), kind: Some(CompletionItemKind::MODULE), detail: Some("built-in".into()), ..Default::default() });
        }

        if let Some(root) = self.workspace_root.lock().await.clone() {
            for path in Self::scan_workspace_clsx(&root) {
                let p = Path::new(&path);
                if let Some(mod_name) = p.file_stem().and_then(|s| s.to_str()) {
                    if !mod_name.is_empty() {
                        items.push(CompletionItem { label: mod_name.to_string(), kind: Some(CompletionItemKind::FILE), detail: Some(format!("workspace/{}", path)), ..Default::default() });
                    }
                }
            }
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let source = self.get_doc(&params.text_document_position_params.text_document.uri).await.unwrap_or_default();
        let pos = params.text_document_position_params.position;
        let word = extract_word_at(&source, pos.line as usize, pos.character as usize).unwrap_or_default();
        if !word.is_empty() {
            return Ok(Some(Hover { contents: HoverContents::Scalar(MarkedString::String(format!("`{}`", word))), range: None }));
        }
        Ok(None)
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = self.get_doc(&uri).await.unwrap_or_default();
        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() { Ok(t) => t, Err(_) => return Ok(None) };
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() { Ok(m) => m, Err(_) => return Ok(None) };
        let symbols = build_symbols(&module);
        if let Some(word) = extract_word_at(&source, pos.line as usize, pos.character as usize) {
            for sym in &symbols {
                if sym.name == word {
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range { start: Position { line: sym.span.start_line.saturating_sub(1), character: sym.span.start_col.saturating_sub(1) }, end: Position { line: sym.span.end_line.saturating_sub(1), character: sym.span.end_col.saturating_sub(1) } },
                    })));
                }
            }
        }
        Ok(None)
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let source = self.get_doc(&params.text_document.uri).await.unwrap_or_default();
        let mut lexer = Lexer::new(&source);
        let tokens = match lexer.tokenize() { Ok(t) => t, Err(_) => return Ok(None) };
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() { Ok(m) => m, Err(_) => return Ok(None) };
        let symbols = build_symbols(&module);
        let info: Vec<SymbolInformation> = symbols.iter().map(|s| {
            SymbolInformation {
                name: s.name.clone(),
                kind: match s.kind { SymKind::Function => SymbolKind::FUNCTION, _ => SymbolKind::VARIABLE },
                location: Location { uri: params.text_document.uri.clone(), range: Range { start: Position { line: s.span.start_line.saturating_sub(1), character: s.span.start_col.saturating_sub(1) }, end: Position { line: s.span.end_line.saturating_sub(1), character: s.span.end_col.saturating_sub(1) } } },
                container_name: None, tags: None, deprecated: None,
            }
        }).collect();
        Ok(Some(DocumentSymbolResponse::Flat(info)))
    }
}

// ─── Free helpers ──────────────────────────────────────────────────────

/// Obtiene las claves de un Value::Record (nombres de miembros exportados)
fn record_keys(v: &Value) -> Vec<String> {
    if let Value::Record(map) = v { map.keys().cloned().collect() } else { vec![] }
}

/// Carga un módulo runtime y devuelve sus exportaciones
fn load_module_exports(name: &str) -> Option<Vec<String>> {
    match name {
        "math" => Some(record_keys(&math::module())),
        "json" => Some(record_keys(&json::module())),
        "fs" => {
            let vfs = Arc::new(VfsResolver::new());
            Some(record_keys(&fs::module(vfs)))
        }
        "http" => Some(record_keys(&http::module())),
        _ => None,
    }
}

/// Construye mapa: nombre_de_variable → nombre_del_módulo para imports en el source
fn build_import_map(source: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let toks = match Lexer::new(source).tokenize() { Ok(t) => t, Err(_) => return map };
    let module = match Parser::new(toks).parse() { Ok(m) => m, Err(_) => return map };
    for stmt in &module.statements {
        match stmt {
            Statement::Import(i) => {
                // import "math" as m  →  m → math
                if let Some(alias) = &i.alias {
                    map.insert(alias.clone(), i.path.clone());
                } else {
                    let name = i.path.rsplit('/').next().unwrap_or(&i.path);
                    let name = name.trim_end_matches(".clsx");
                    map.insert(name.to_string(), i.path.clone());
                }
            }
            Statement::FromImport(fi) => {
                // from "math" import abs, PI  →  abs → math, PI → math
                for im in &fi.names {
                    let alias = im.alias.as_deref().unwrap_or(&im.name);
                    map.insert(alias.to_string(), fi.path.clone());
                }
            }
            _ => {}
        }
    }
    map
}

fn extract_word_at(source: &str, line: usize, col: usize) -> Option<String> {
    let line_text = source.lines().nth(line)?;
    if col >= line_text.len() { return None; }
    let chars: Vec<char> = line_text.chars().collect();
    let mut start = col;
    let mut end = col;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') { start -= 1; }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') { end += 1; }
    if start < end { Some(chars[start..end].iter().collect()) } else { None }
}

fn scope_symbols(source: &str) -> Vec<(String, SymKind)> {
    let mut lexer = match Lexer::new(source).tokenize() { Ok(t) => t, Err(_) => return vec![] };
    let module = match Parser::new(lexer).parse() { Ok(m) => m, Err(_) => return vec![] };
    let mut r = Vec::new();
    for stmt in &module.statements {
        match stmt {
            Statement::FunctionDecl(f) => {
                r.push((f.name.clone(), SymKind::Function));
                for p in &f.params { r.push((p.name.clone(), SymKind::Parameter)); }
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v) => { r.push((v.name.clone(), SymKind::Variable)); }
            _ => {}
        }
    }
    r
}

fn struct_fields(source: &str, struct_name: &str) -> Vec<(String, String)> {
    let mut lexer = match Lexer::new(source).tokenize() { Ok(t) => t, Err(_) => return vec![] };
    let module = match Parser::new(lexer).parse() { Ok(m) => m, Err(_) => return vec![] };
    for stmt in &module.statements {
        if let Statement::StructureDecl(s) = stmt {
            if s.name == struct_name {
                return s.fields.iter().map(|f| {
                    let hint = match &f.type_ann.kind { TypeKind::Named(n, _) => n.clone(), _ => "Any".to_string() };
                    (f.name.clone(), hint)
                }).collect();
            }
        }
    }
    vec![]
}

fn complete_member(source: &str, pos: Position) -> Option<CompletionResponse> {
    let line = pos.line as usize;
    let col = (pos.character.saturating_sub(2)) as usize;
    let word = if let Some(line_text) = source.lines().nth(line) {
        let bound = col.min(line_text.len());
        let before: String = line_text.chars().take(bound).collect();
        let chars: Vec<char> = before.chars().collect();
        let mut end = chars.len();
        while end > 0 && (chars[end - 1].is_alphanumeric() || chars[end - 1] == '_') { end -= 1; }
        let start = end;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') { end += 1; }
        if start < end { Some(chars[start..end].iter().collect::<String>()) } else { None }
    } else { None };

    let obj = word.unwrap_or_default();
    if obj.is_empty() { return None; }

    let mut members: Vec<(String, CompletionItemKind, String)> = Vec::new();

    // 1. Buscar en módulos runtime via import map
    let import_map = build_import_map(source);
    if let Some(module_name) = import_map.get(&obj) {
        if let Some(keys) = load_module_exports(module_name) {
            for k in keys {
                members.push((k.clone(), CompletionItemKind::FUNCTION, format!("{}::{}", module_name, k)));
            }
        }
    }

    // 2. Buscar si es un nombre de módulo directo (sin import)
    if members.is_empty() {
        if let Some(keys) = load_module_exports(&obj) {
            for k in keys {
                members.push((k.clone(), CompletionItemKind::FUNCTION, format!("{}::{}", obj, k)));
            }
        }
    }

    // 3. Buscar fields de structure declarado en el source
    if members.is_empty() {
        let fields = struct_fields(source, &obj);
        for (name, detail) in fields {
            members.push((name, CompletionItemKind::PROPERTY, detail));
        }
    }

    // 4. Scope symbols: funciones, variables, parámetros
    if members.is_empty() {
        let scope = scope_symbols(source);
        for (name, kind) in &scope {
            if name == &obj { continue; }
            let icon = match kind { SymKind::Function => CompletionItemKind::FUNCTION, _ => CompletionItemKind::VARIABLE };
            members.push((name.clone(), icon, "scope".to_string()));
        }
    }

    if members.is_empty() { return None; }

    Some(CompletionResponse::Array(members.into_iter().map(|(name, kind, detail)| CompletionItem {
        label: name, kind: Some(kind), detail: Some(detail), ..Default::default()
    }).collect()))
}

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
