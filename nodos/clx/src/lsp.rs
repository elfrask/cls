use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cls_core::config::TypesConfig;
use cls_core::error::diagnostic::Severity;
use cls_core::error::Diagnostic as ClsDiag;
use cls_core::frontend::{Lexer, Parser};
use cls_core::middleware::{NameResolver, TypeChecker};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService, Server};

use crate::lsp_index::{
    find_member, members_from_type_module, parse_file, parse_source, ImportRef, LspSymbol,
    MemberInfo, ParsedFile, WorkspaceIndex,
};
use crate::lsp_scope::local_symbols_at;
use crate::lsp_text::{
    char_index_to_utf16_col, line_text, member_access_at, member_access_before_cursor,
    position_to_byte_index, span_to_range as text_span_to_range, word_at,
};
use crate::type_defs::{self, TypeModule};

pub struct ClsLspBackend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
    workspace_root: Arc<Mutex<Option<PathBuf>>>,
    type_defs: Arc<Mutex<HashMap<String, TypeModule>>>,
    index: Arc<Mutex<WorkspaceIndex>>,
}

impl ClsLspBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            workspace_root: Arc::new(Mutex::new(None)),
            type_defs: Arc::new(Mutex::new(HashMap::new())),
            index: Arc::new(Mutex::new(WorkspaceIndex::default())),
        }
    }

    async fn reload_workspace(&self) {
        let root = self.workspace_root.lock().await.clone();
        let root_str = root.as_ref().and_then(|p| p.to_str());
        let defs = type_defs::load_all_type_definitions(root_str);
        *self.type_defs.lock().await = defs;

        let index = match root {
            Some(root) => WorkspaceIndex::build(&root),
            None => WorkspaceIndex::default(),
        };
        *self.index.lock().await = index;
    }

    async fn source_for_uri(&self, uri: &Url) -> Option<String> {
        if let Some(text) = self.documents.lock().await.get(uri) {
            return Some(text.clone());
        }
        let path = uri.to_file_path().ok()?;
        self.index
            .lock()
            .await
            .file(&path)
            .map(|f| f.source.clone())
    }

    async fn parsed_for_uri(&self, uri: &Url) -> Option<ParsedFile> {
        if let Some(text) = self.documents.lock().await.get(uri) {
            let path = uri.to_file_path().ok()?;
            return parse_file(&path, text.clone());
        }
        let path = uri.to_file_path().ok()?;
        self.index.lock().await.file(&path).cloned()
    }

    async fn send_diags(&self, uri: &Url) {
        let source = self.source_for_uri(uri).await.unwrap_or_default();
        let workspace = self.workspace_root.lock().await.clone();
        let diagnostics = Self::run_pipeline(&source, workspace.as_deref());
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn run_pipeline(source: &str, workspace: Option<&Path>) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let mut lexer = Lexer::new(source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                Self::push_diag(
                    &mut diags,
                    source,
                    &e.to_string(),
                    DiagnosticSeverity::ERROR,
                );
                return diags;
            }
        };
        let mut parser = Parser::new(tokens);
        let module = match parser.parse() {
            Ok(m) => m,
            Err(e) => {
                Self::push_diag(
                    &mut diags,
                    source,
                    &e.to_string(),
                    DiagnosticSeverity::ERROR,
                );
                return diags;
            }
        };
        let mut resolver = NameResolver::new();
        if let Err(e) = resolver.resolve(&module) {
            Self::push_diag(
                &mut diags,
                source,
                &e.to_string(),
                DiagnosticSeverity::ERROR,
            );
        }
        for d in resolver.diagnostics() {
            Self::push_cls_diag(&mut diags, source, d, DiagnosticSeverity::ERROR);
        }
        let types_config = workspace
            .and_then(|root| {
                let path = root.join("cls.json");
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
            Self::push_diag(
                &mut diags,
                source,
                &e.to_string(),
                DiagnosticSeverity::ERROR,
            );
        }
        for d in checker.diagnostics() {
            let sev = match d.severity {
                Severity::Error => DiagnosticSeverity::ERROR,
                Severity::Warning => DiagnosticSeverity::WARNING,
                _ => DiagnosticSeverity::INFORMATION,
            };
            Self::push_cls_diag(&mut diags, source, d, sev);
        }
        diags
    }

    fn push_diag(
        diags: &mut Vec<Diagnostic>,
        source: &str,
        msg: &str,
        severity: DiagnosticSeverity,
    ) {
        if let Some((line, col)) = cls_core::error::ClsError::extract_line_col(msg) {
            let line0 = line.saturating_sub(1) as u32;
            let text = line_text(source, line0).unwrap_or("");
            let start_char = col.saturating_sub(1);
            let end_char = col;
            diags.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line0,
                        character: char_index_to_utf16_col(text, start_char),
                    },
                    end: Position {
                        line: line0,
                        character: char_index_to_utf16_col(text, end_char),
                    },
                },
                severity: Some(severity),
                message: msg.to_string(),
                source: Some("clx".into()),
                ..Default::default()
            });
        } else if !msg.is_empty() {
            diags.push(Diagnostic {
                range: Range::default(),
                severity: Some(severity),
                message: msg.to_string(),
                source: Some("clx".into()),
                ..Default::default()
            });
        }
    }

    fn push_cls_diag(
        diags: &mut Vec<Diagnostic>,
        source: &str,
        d: &ClsDiag,
        severity: DiagnosticSeverity,
    ) {
        diags.push(Diagnostic {
            range: text_span_to_range(source, &d.span),
            severity: Some(severity),
            message: d.message.clone(),
            source: Some("clx".into()),
            ..Default::default()
        });
    }

    async fn general_completions(
        &self,
        source: &str,
        uri: &Url,
        position: Position,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for sym in local_symbols_at(source, position) {
            push_symbol_completion(&mut items, &mut seen, &sym, "scope");
        }

        if let Some(parsed) = self.parsed_for_uri(uri).await {
            for import in &parsed.imports {
                if let Some(alias) = &import.alias {
                    if seen.insert(alias.clone()) {
                        items.push(CompletionItem {
                            label: alias.clone(),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some(format!("import {}", import.path)),
                            ..Default::default()
                        });
                    }
                }
                for (name, alias) in &import.names {
                    let label = alias.as_ref().unwrap_or(name);
                    if seen.insert(label.clone()) {
                        items.push(CompletionItem {
                            label: label.clone(),
                            kind: Some(CompletionItemKind::VARIABLE),
                            detail: Some(format!("from {}", import.path)),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        let docs = self.documents.lock().await.clone();
        for (doc_uri, text) in docs {
            if &doc_uri == uri {
                continue;
            }
            for sym in parse_source(&text) {
                push_symbol_completion(&mut items, &mut seen, &sym, "open-document");
            }
        }

        for kw in [
            "var",
            "let",
            "const",
            "function",
            "if",
            "else",
            "while",
            "for",
            "return",
            "import",
            "from",
            "as",
            "export",
            "structure",
            "interface",
            "class",
            "enum",
            "alias",
            "true",
            "false",
            "null",
            "break",
            "continue",
            "loop",
            "switch",
            "async",
            "await",
            "static",
            "public",
            "private",
            "protected",
            "readonly",
            "is",
        ] {
            if seen.insert(kw.to_string()) {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    ..Default::default()
                });
            }
        }

        let defs = self.type_defs.lock().await.clone();
        if let Some(core) = defs.get("core") {
            for m in &core.members {
                if seen.insert(m.name.clone()) {
                    items.push(CompletionItem {
                        label: m.name.clone(),
                        kind: Some(completion_kind_for_member(&m.kind)),
                        detail: Some(m.signature.clone()),
                        documentation: format_doc(&m.doc),
                        ..Default::default()
                    });
                }
            }
        }
        for (name, tm) in &defs {
            if name == "core" || !seen.insert(name.clone()) {
                continue;
            }
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some(tm.description.clone()),
                ..Default::default()
            });
        }

        {
            let index = self.index.lock().await;
            for file in index.all_files() {
                let name = file.module_name();
                if name.is_empty() || !seen.insert(name.clone()) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name,
                    kind: Some(CompletionItemKind::FILE),
                    detail: Some(format!("workspace/{}", relative_name(&file.path))),
                    ..Default::default()
                });
            }
        }

        items
    }

    async fn complete_members(
        &self,
        source: &str,
        position: Position,
        uri: &Url,
        segments: &[String],
    ) -> Option<Vec<CompletionItem>> {
        let mut parsed = self.parsed_for_uri(uri).await?;
        // Incluye parámetros y variables locales visibles en el cursor.
        parsed.symbols = local_symbols_at(source, position);
        parsed.source = source.to_string();
        let defs = self.type_defs.lock().await.clone();
        let index = self.index.lock().await;

        let first = segments.first()?;
        let mut members = receiver_members(&parsed, &defs, &index, first)?;
        for segment in &segments[1..] {
            let member = find_member(&members, segment)?;
            members = member_members(&parsed, &defs, &index, member)?;
        }

        Some(members.iter().map(member_to_completion).collect())
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for ClsLspBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok())
            .or_else(|| {
                params
                    .root_uri
                    .as_ref()
                    .and_then(|uri| uri.to_file_path().ok())
            });

        if let Some(root) = root {
            *self.workspace_root.lock().await = Some(root);
        }
        self.reload_workspace().await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), ":".into(), "\"".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "clx-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents
            .lock()
            .await
            .insert(uri.clone(), params.text_document.text);
        self.send_diags(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut text = self
            .documents
            .lock()
            .await
            .get(&uri)
            .cloned()
            .unwrap_or_default();
        for change in params.content_changes {
            apply_content_change(&mut text, change);
        }
        self.documents.lock().await.insert(uri.clone(), text);
        self.send_diags(&uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Ok(path) = params.text_document.uri.to_file_path() {
            let reload_defs = matches!(path.extension().and_then(|e| e.to_str()), Some("clsi"))
                || path.file_name().and_then(|n| n.to_str()) == Some("cls.json");
            if reload_defs {
                self.reload_workspace().await;
            } else {
                self.index.lock().await.load_path(&path);
            }
        }
        self.send_diags(&params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.lock().await.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let affects_index = params.changes.iter().any(|change| {
            let Ok(path) = change.uri.to_file_path() else {
                return false;
            };
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("clsx" | "clsi")
            ) || path.file_name().and_then(|n| n.to_str()) == Some("cls.json")
        });
        if affects_index {
            self.reload_workspace().await;
            let open_docs: Vec<Url> = {
                let docs = self.documents.lock().await;
                docs.keys().cloned().collect()
            };
            for uri in open_docs {
                self.send_diags(&uri).await;
            }
        }
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        if let Some(folder) = params.event.added.first() {
            if let Ok(path) = folder.uri.to_file_path() {
                *self.workspace_root.lock().await = Some(path);
                self.reload_workspace().await;
            }
        }
    }

    // ─── Completion ──────────────────────────────────────────────────────

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let source = self.source_for_uri(&uri).await.unwrap_or_default();

        if let Some((segments, _partial)) = member_access_before_cursor(&source, pos) {
            let items = self
                .complete_members(&source, pos, &uri, &segments)
                .await
                .unwrap_or_default();
            return Ok(Some(completion_response(items)));
        }

        let items = self.general_completions(&source, &uri, pos).await;
        Ok(Some(completion_response(items)))
    }

    // ─── Hover ───────────────────────────────────────────────────────────

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = self.source_for_uri(&uri).await.unwrap_or_default();
        let Some((word, range)) = word_at(&source, pos) else {
            return Ok(None);
        };

        let local = local_symbols_at(&source, pos);
        {
            let index = self.index.lock().await;
            if let Some(target) = resolve_member_definition(&source, pos, &local, &index, &uri) {
                return Ok(Some(symbol_hover(&target.symbol, range)));
            }
        }
        if let Some(sym) = local.into_iter().find(|sym| sym.name == word) {
            return Ok(Some(symbol_hover(&sym, range)));
        }

        let defs = self.type_defs.lock().await.clone();
        if let Some(parsed) = self.parsed_for_uri(&uri).await {
            for import in &parsed.imports {
                if import.alias.as_deref() == Some(word.as_str()) {
                    if let Some(tm) = defs.get(&import.path) {
                        return Ok(Some(module_hover(tm, range)));
                    }
                }
                if let Some((name, _)) = import
                    .names
                    .iter()
                    .find(|(name, alias)| alias.as_deref().unwrap_or(name) == word)
                {
                    if let Some(tm) = defs.get(&import.path) {
                        if let Some(member) = tm.members.iter().find(|m| &m.name == name) {
                            return Ok(Some(member_hover(tm, member, range)));
                        }
                    }
                }
            }
        }
        if let Some(tm) = defs.get(&word) {
            return Ok(Some(module_hover(tm, range)));
        }
        for tm in defs.values() {
            if let Some(member) = tm.members.iter().find(|m| m.name == word) {
                return Ok(Some(member_hover(tm, member, range)));
            }
        }

        if let Some(parsed) = self.parsed_for_uri(&uri).await {
            let index = self.index.lock().await;
            if let Some((file, target_name)) = imported_symbol_file(&parsed, &index, &word) {
                if let Some(sym) = file.find_symbol(&target_name) {
                    return Ok(Some(symbol_hover(sym, range)));
                }
            }
            if let Some((_file, sym)) = index.find_global(&word).into_iter().next() {
                return Ok(Some(symbol_hover(sym, range)));
            }
        }

        Ok(None)
    }

    // ─── Go-to-definition ────────────────────────────────────────────────

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let source = self.source_for_uri(&uri).await.unwrap_or_default();
        let Some((word, _)) = word_at(&source, pos) else {
            return Ok(None);
        };
        let Some(parsed) = self.parsed_for_uri(&uri).await else {
            return Ok(None);
        };

        let local = local_symbols_at(&source, pos);
        {
            let index = self.index.lock().await;
            if let Some(target) = resolve_member_definition(&source, pos, &local, &index, &uri) {
                return Ok(Some(definition_response(
                    target.uri,
                    &target.source,
                    &target.symbol,
                )));
            }
        }
        if let Some(sym) = local.into_iter().find(|sym| sym.name == word) {
            return Ok(Some(definition_response(uri.clone(), &source, &sym)));
        }

        let index = self.index.lock().await;
        for import in &parsed.imports {
            let target_name = import
                .names
                .iter()
                .find(|(name, alias)| alias.as_deref().unwrap_or(name) == word)
                .map(|(name, _)| name.as_str());

            if import.alias.as_deref() == Some(word.as_str()) {
                if let Some(file) = index.resolve_import(&parsed, &import.path) {
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: file.uri.clone(),
                        range: Range::default(),
                    })));
                }
            }

            if let Some(target_name) = target_name {
                if let Some(file) = index.resolve_import(&parsed, &import.path) {
                    if let Some(sym) = file.find_symbol(target_name) {
                        return Ok(Some(definition_response(
                            file.uri.clone(),
                            &file.source,
                            sym,
                        )));
                    }
                }
            }
        }

        let matches = index.find_global(&word);
        if matches.len() == 1 {
            let (file, sym) = matches[0];
            return Ok(Some(definition_response(
                file.uri.clone(),
                &file.source,
                sym,
            )));
        }

        Ok(None)
    }

    // ─── Document symbols ────────────────────────────────────────────────

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let source = self
            .source_for_uri(&params.text_document.uri)
            .await
            .unwrap_or_default();
        let symbols = parse_source(&source);
        if symbols.is_empty() {
            return Ok(None);
        }
        Ok(Some(DocumentSymbolResponse::Nested(
            symbols
                .iter()
                .map(|sym| to_document_symbol(sym, &source))
                .collect(),
        )))
    }
}

// ─── Resolución de miembros ─────────────────────────────────────────────

fn receiver_members(
    parsed: &ParsedFile,
    defs: &HashMap<String, TypeModule>,
    index: &WorkspaceIndex,
    name: &str,
) -> Option<Vec<MemberInfo>> {
    for import in &parsed.imports {
        if !import_matches(import, name) {
            continue;
        }
        if let Some(tm) = defs.get(&import.path) {
            return Some(members_from_type_module(tm));
        }
        if let Some(file) = index.resolve_import(parsed, &import.path) {
            return Some(file.symbols.iter().map(MemberInfo::from).collect());
        }
    }

    if let Some(tm) = defs.get(name) {
        return Some(members_from_type_module(tm));
    }
    if let Some(file) = index.file_by_module_name(name) {
        return Some(file.symbols.iter().map(MemberInfo::from).collect());
    }
    if let Some(sym) = parsed.find_symbol(name) {
        if !sym.children.is_empty() {
            return Some(sym.children.iter().map(MemberInfo::from).collect());
        }
        if let Some(ty) = &sym.ty {
            return type_members(parsed, defs, index, ty);
        }
    }
    type_members(parsed, defs, index, name)
}

fn member_members(
    parsed: &ParsedFile,
    defs: &HashMap<String, TypeModule>,
    index: &WorkspaceIndex,
    member: &MemberInfo,
) -> Option<Vec<MemberInfo>> {
    if !member.children.is_empty() {
        return Some(member.children.clone());
    }
    member
        .ty
        .as_deref()
        .and_then(|ty| type_members(parsed, defs, index, ty))
}

fn type_members(
    parsed: &ParsedFile,
    defs: &HashMap<String, TypeModule>,
    index: &WorkspaceIndex,
    type_name: &str,
) -> Option<Vec<MemberInfo>> {
    let base = type_name
        .split('<')
        .next()
        .unwrap_or(type_name)
        .trim()
        .trim_end_matches("[]");

    if let Some(sym) = parsed.find_symbol(base) {
        if !sym.children.is_empty() {
            return Some(sym.children.iter().map(MemberInfo::from).collect());
        }
    }
    for (_file, sym) in index.find_global(base) {
        if !sym.children.is_empty() {
            return Some(sym.children.iter().map(MemberInfo::from).collect());
        }
    }
    defs.get(base).map(members_from_type_module)
}

fn import_matches(import: &ImportRef, name: &str) -> bool {
    if import.alias.as_deref() == Some(name) {
        return true;
    }
    if import.alias.is_none() {
        if import
            .names
            .iter()
            .any(|(original, alias)| alias.as_deref().unwrap_or(original) == name)
        {
            return true;
        }
        return path_leaf(&import.path) == name;
    }
    false
}

fn path_leaf(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .trim_end_matches(".clsx")
        .trim_end_matches(".clsi")
}

#[derive(Clone)]
struct DefinitionTarget {
    uri: Url,
    source: String,
    symbol: LspSymbol,
}

fn is_type_symbol(symbol: &LspSymbol) -> bool {
    symbol.kind == SymbolKind::CLASS
        || symbol.kind == SymbolKind::STRUCT
        || symbol.kind == SymbolKind::INTERFACE
        || symbol.kind == SymbolKind::ENUM
}

fn base_type_name(ty: &str) -> &str {
    ty.split('<')
        .next()
        .unwrap_or(ty)
        .trim()
        .trim_end_matches("[]")
}

fn type_context(
    local: &[LspSymbol],
    index: &WorkspaceIndex,
    current_uri: &Url,
    current_source: &str,
    type_name: &str,
) -> Option<DefinitionTarget> {
    if let Some(symbol) = local
        .iter()
        .find(|symbol| is_type_symbol(symbol) && symbol.name == type_name)
    {
        return Some(DefinitionTarget {
            uri: current_uri.clone(),
            source: current_source.to_string(),
            symbol: symbol.clone(),
        });
    }
    for (file, symbol) in index.find_global(type_name) {
        if is_type_symbol(symbol) {
            return Some(DefinitionTarget {
                uri: file.uri.clone(),
                source: file.source.clone(),
                symbol: symbol.clone(),
            });
        }
    }
    None
}

fn resolve_member_definition(
    source: &str,
    position: Position,
    local: &[LspSymbol],
    index: &WorkspaceIndex,
    current_uri: &Url,
) -> Option<DefinitionTarget> {
    let (segments, member) = member_access_at(source, position)?;
    let first = segments.first()?;
    let receiver = local.iter().find(|symbol| &symbol.name == first)?;
    let receiver_type = base_type_name(receiver.ty.as_deref()?);
    let mut target = type_context(local, index, current_uri, source, receiver_type)?;

    for segment in &segments[1..] {
        let child = target
            .symbol
            .children
            .iter()
            .find(|child| &child.name == segment)?
            .clone();
        let next_type = base_type_name(child.ty.as_deref()?);
        target = type_context(local, index, &target.uri, &target.source, next_type)?;
    }

    let symbol = target
        .symbol
        .children
        .iter()
        .find(|child| child.name == member)?
        .clone();
    Some(DefinitionTarget {
        uri: target.uri,
        source: target.source,
        symbol,
    })
}

fn find_declaration_line(source: &str, name: &str) -> Option<u32> {
    for (line_no, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        for keyword in [
            "class",
            "structure",
            "interface",
            "enum",
            "alias",
            "module",
            "namespace",
            "function",
            "var",
            "const",
        ] {
            let pattern = format!("{} {}", keyword, name);
            if trimmed.starts_with(&pattern) || trimmed.contains(&pattern) {
                return Some(line_no as u32);
            }
        }
    }
    None
}

fn definition_range(source: &str, symbol: &LspSymbol) -> Range {
    let fallback = text_span_to_range(source, &symbol.span);
    let line = line_text(source, fallback.start.line).unwrap_or("");
    let trimmed = line.trim_start();

    // Los spans de clase/structure/interface/enum suelen apuntar al cierre.
    if is_type_symbol(symbol) {
        if let Some(line_no) = find_declaration_line(source, &symbol.name) {
            return line_range(source, line_no);
        }
    }

    let span_has_name = line.contains(&symbol.name);
    if !trimmed.is_empty() && !trimmed.starts_with('}') && span_has_name {
        return fallback;
    }

    let type_name = symbol.ty.as_deref().map(base_type_name);
    let declaration_line = type_name
        .and_then(|name| find_declaration_line(source, name))
        .or_else(|| find_declaration_line(source, &symbol.name));
    declaration_line
        .map(|line_no| line_range(source, line_no))
        .unwrap_or(fallback)
}

fn line_range(source: &str, line_no: u32) -> Range {
    let text = line_text(source, line_no).unwrap_or("");
    Range {
        start: Position {
            line: line_no,
            character: 0,
        },
        end: Position {
            line: line_no,
            character: char_index_to_utf16_col(text, text.chars().count()),
        },
    }
}

fn imported_symbol_file<'a>(
    parsed: &ParsedFile,
    index: &'a WorkspaceIndex,
    word: &str,
) -> Option<(&'a ParsedFile, String)> {
    for import in &parsed.imports {
        if import.alias.as_deref() == Some(word) {
            if let Some(file) = index.resolve_import(parsed, &import.path) {
                return Some((file, word.to_string()));
            }
        }
        if let Some((name, _)) = import
            .names
            .iter()
            .find(|(name, alias)| alias.as_deref().unwrap_or(name) == word)
        {
            if let Some(file) = index.resolve_import(parsed, &import.path) {
                return Some((file, name.clone()));
            }
        }
    }
    None
}

// ─── Completion helpers ─────────────────────────────────────────────────

fn apply_content_change(text: &mut String, change: TextDocumentContentChangeEvent) {
    let Some(range) = change.range else {
        *text = change.text;
        return;
    };
    let Some(start) = position_to_byte_index(text, range.start) else {
        *text = change.text;
        return;
    };
    let Some(end) = position_to_byte_index(text, range.end) else {
        *text = change.text;
        return;
    };
    if start <= end && end <= text.len() {
        text.replace_range(start..end, &change.text);
    } else {
        *text = change.text;
    }
}

fn completion_response(items: Vec<CompletionItem>) -> CompletionResponse {
    CompletionResponse::List(CompletionList {
        // El scope cambia con cada edición; pedir re-consulta al cliente en
        // vez de dejar que cachee la lista hasta cerrar el popup.
        is_incomplete: true,
        items,
    })
}

fn push_symbol_completion(
    items: &mut Vec<CompletionItem>,
    seen: &mut HashSet<String>,
    sym: &LspSymbol,
    detail_prefix: &str,
) {
    if seen.insert(sym.name.clone()) {
        items.push(CompletionItem {
            label: sym.name.clone(),
            kind: Some(completion_kind(sym.kind)),
            detail: Some(format!("{}: {}", detail_prefix, sym.detail)),
            documentation: format_doc(&sym.doc),
            ..Default::default()
        });
    }
}

fn member_to_completion(member: &MemberInfo) -> CompletionItem {
    CompletionItem {
        label: member.name.clone(),
        kind: Some(completion_kind(member.kind)),
        detail: Some(member.detail.clone()),
        documentation: format_doc(&member.doc),
        ..Default::default()
    }
}

fn completion_kind(kind: SymbolKind) -> CompletionItemKind {
    if kind == SymbolKind::FUNCTION {
        CompletionItemKind::FUNCTION
    } else if kind == SymbolKind::METHOD {
        CompletionItemKind::METHOD
    } else if kind == SymbolKind::CONSTRUCTOR {
        CompletionItemKind::CONSTRUCTOR
    } else if kind == SymbolKind::CLASS {
        CompletionItemKind::CLASS
    } else if kind == SymbolKind::STRUCT {
        CompletionItemKind::STRUCT
    } else if kind == SymbolKind::INTERFACE {
        CompletionItemKind::INTERFACE
    } else if kind == SymbolKind::ENUM {
        CompletionItemKind::ENUM
    } else if kind == SymbolKind::ENUM_MEMBER {
        CompletionItemKind::ENUM_MEMBER
    } else if kind == SymbolKind::PROPERTY {
        CompletionItemKind::PROPERTY
    } else if kind == SymbolKind::FIELD {
        CompletionItemKind::FIELD
    } else if kind == SymbolKind::CONSTANT {
        CompletionItemKind::CONSTANT
    } else if kind == SymbolKind::MODULE || kind == SymbolKind::NAMESPACE {
        CompletionItemKind::MODULE
    } else if kind == SymbolKind::TYPE_PARAMETER {
        CompletionItemKind::TYPE_PARAMETER
    } else {
        CompletionItemKind::VARIABLE
    }
}

fn completion_kind_for_member(kind: &type_defs::MemberKind) -> CompletionItemKind {
    use type_defs::MemberKind::*;
    match kind {
        Function => CompletionItemKind::FUNCTION,
        Variable => CompletionItemKind::VARIABLE,
        Constant => CompletionItemKind::CONSTANT,
        Class => CompletionItemKind::CLASS,
        Structure => CompletionItemKind::STRUCT,
        Interface => CompletionItemKind::INTERFACE,
        Module | Namespace => CompletionItemKind::MODULE,
        Type => CompletionItemKind::TYPE_PARAMETER,
        Enum => CompletionItemKind::ENUM,
    }
}

// ─── Hover / definition helpers ─────────────────────────────────────────

fn module_hover(tm: &TypeModule, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!(
                "**module {}**\n\n{}",
                tm.name,
                if tm.description.is_empty() {
                    "Módulo CLS".to_string()
                } else {
                    tm.description.clone()
                }
            ),
        }),
        range: Some(range),
    }
}

fn member_hover(tm: &TypeModule, member: &type_defs::TypeMember, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_doc_content(tm, member),
        }),
        range: Some(range),
    }
}

fn symbol_hover(sym: &LspSymbol, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_symbol_markdown(sym),
        }),
        range: Some(range),
    }
}

fn format_symbol_markdown(sym: &LspSymbol) -> String {
    let mut parts = vec![format!("**{}**\n\n`{}`", sym.name, sym.detail)];
    if !sym.doc.is_empty() {
        parts.push(format_doc_text(&sym.doc));
    }
    parts.join("\n\n")
}

fn format_doc_text(doc: &str) -> String {
    let mut parts = Vec::new();
    for line in doc.lines() {
        let text = line.trim_start_matches('#').trim();
        if let Some(rest) = text.strip_prefix("@description") {
            parts.push(rest.trim().to_string());
        } else if let Some(rest) = text.strip_prefix("@params") {
            parts.push(format!("- `{}`", rest.trim()));
        } else if let Some(rest) = text.strip_prefix("@return") {
            parts.push(format!("-> {}", rest.trim()));
        } else if let Some(rest) = text.strip_prefix("@deprecated") {
            parts.push(format!("~~DEPRECATED: {}~~", rest.trim()));
        }
    }
    parts.join("\n\n")
}

fn format_doc(doc: &str) -> Option<Documentation> {
    let text = format_doc_text(doc);
    if text.is_empty() {
        None
    } else {
        Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: text,
        }))
    }
}

fn format_doc_content(tm: &TypeModule, m: &type_defs::TypeMember) -> String {
    let mut parts = vec![format!("**{}**  `{}`", m.name, m.signature)];
    if !tm.description.is_empty() {
        parts.push(format!("_{}_", tm.description));
    }
    let doc = format_doc_text(&m.doc);
    if !doc.is_empty() {
        parts.push(doc);
    }
    parts.join("\n\n")
}

fn definition_response(uri: Url, source: &str, sym: &LspSymbol) -> GotoDefinitionResponse {
    GotoDefinitionResponse::Scalar(Location {
        uri,
        range: definition_range(source, sym),
    })
}

#[allow(deprecated)]
fn to_document_symbol(sym: &LspSymbol, source: &str) -> DocumentSymbol {
    let range = text_span_to_range(source, &sym.span);
    DocumentSymbol {
        name: sym.name.clone(),
        detail: if sym.detail.is_empty() {
            None
        } else {
            Some(sym.detail.clone())
        },
        kind: sym.kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if sym.children.is_empty() {
            None
        } else {
            Some(
                sym.children
                    .iter()
                    .map(|child| to_document_symbol(child, source))
                    .collect(),
            )
        },
    }
}

fn relative_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

pub fn run_server(silent: bool) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if !silent {
            eprintln!("[clx lsp] ready");
        }
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(|client| ClsLspBackend::new(client));
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}
