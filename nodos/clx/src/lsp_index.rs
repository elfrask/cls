//! Índice de símbolos del workspace para el LSP.
//!
//! El índice es deliberadamente simple: recorre `.clsx`/`.clsi`, parsea cada
//! archivo y guarda declaraciones, imports y documentación. Se reconstruye al
//! inicializar y cuando el cliente avisa cambios en archivos observados.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use cls_core::config::ModuleManifest;
use cls_core::error::Span;
use cls_core::frontend::ast::*;
use cls_core::frontend::{Lexer, Parser};
use tower_lsp::lsp_types::{SymbolKind, Url};

use crate::type_defs::{MemberKind, TypeModule};

#[derive(Debug, Clone)]
pub struct LspSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub detail: String,
    pub doc: String,
    pub span: Span,
    /// Tipo declarado (variables/campos/parámetros) o tipo de retorno.
    pub ty: Option<String>,
    pub children: Vec<LspSymbol>,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub detail: String,
    pub doc: String,
    pub ty: Option<String>,
    pub children: Vec<MemberInfo>,
}

impl From<&LspSymbol> for MemberInfo {
    fn from(s: &LspSymbol) -> Self {
        Self {
            name: s.name.clone(),
            kind: s.kind,
            detail: s.detail.clone(),
            doc: s.doc.clone(),
            ty: s.ty.clone(),
            children: s.children.iter().map(MemberInfo::from).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportRef {
    pub path: String,
    pub alias: Option<String>,
    pub names: Vec<(String, Option<String>)>,
    #[allow(dead_code)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub uri: Url,
    pub source: String,
    pub symbols: Vec<LspSymbol>,
    pub imports: Vec<ImportRef>,
    #[allow(dead_code)]
    pub exports: HashSet<String>,
}

impl ParsedFile {
    #[allow(dead_code)]
    pub fn top_level(&self) -> &[LspSymbol] {
        &self.symbols
    }

    pub fn find_symbol(&self, name: &str) -> Option<&LspSymbol> {
        find_symbol_in(&self.symbols, name)
    }

    pub fn module_name(&self) -> String {
        module_name_for_path(&self.path)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceIndex {
    pub root: Option<PathBuf>,
    pub files: HashMap<PathBuf, ParsedFile>,
    pub manifest: Option<ModuleManifest>,
}

impl WorkspaceIndex {
    pub fn build(root: &Path) -> Self {
        let root = normalize_path(root);
        let manifest = fs::read_to_string(root.join("cls.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<ModuleManifest>(&s).ok());
        let mut index = Self {
            root: Some(root.clone()),
            files: HashMap::new(),
            manifest,
        };
        index.scan_dir(&root, 0);
        index
    }

    #[allow(dead_code)]
    pub fn rebuild(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        self.files.clear();
        self.manifest = fs::read_to_string(root.join("cls.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<ModuleManifest>(&s).ok());
        self.scan_dir(&root, 0);
    }

    fn scan_dir(&mut self, dir: &Path, depth: usize) {
        if depth > 24 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if should_skip_dir(&name) {
                    continue;
                }
                self.scan_dir(&path, depth + 1);
            } else if is_cls_path(&path) {
                self.load_path(&path);
            }
        }
    }

    pub fn load_path(&mut self, path: &Path) {
        let path = normalize_path(path);
        if !is_cls_path(&path) {
            return;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            return;
        };
        if let Some(parsed) = parse_file(&path, source) {
            self.files.insert(path, parsed);
        }
    }

    #[allow(dead_code)]
    pub fn remove_path(&mut self, path: &Path) {
        self.files.remove(&normalize_path(path));
    }

    pub fn file(&self, path: &Path) -> Option<&ParsedFile> {
        self.files.get(&normalize_path(path))
    }

    pub fn file_by_module_name(&self, name: &str) -> Option<&ParsedFile> {
        let name = name.trim_end_matches(".clsx").trim_end_matches(".clsi");
        self.files.values().find(|f| {
            f.module_name() == name || f.path.file_stem().and_then(|s| s.to_str()) == Some(name)
        })
    }

    /// Resuelve un import desde un archivo del índice.
    pub fn resolve_import(&self, from: &ParsedFile, import_path: &str) -> Option<&ParsedFile> {
        let from_dir = from.path.parent()?;
        let path = Path::new(import_path);
        let mut candidates = Vec::new();

        if path.is_absolute()
            || import_path.contains('/')
            || import_path.contains('\\')
            || import_path.starts_with('.')
        {
            let base = if path.is_absolute() {
                path.to_path_buf()
            } else {
                from_dir.join(path)
            };
            push_file_candidates(&base, &mut candidates);
        } else {
            push_file_candidates(&from_dir.join(import_path), &mut candidates);
            if let Some(root) = &self.root {
                push_file_candidates(&root.join(import_path), &mut candidates);
                push_file_candidates(&root.join("src").join(import_path), &mut candidates);
                push_file_candidates(&root.join("modules").join(import_path), &mut candidates);
                if let Some(m) = &self.manifest {
                    for p in &m.path_modules {
                        let base = if Path::new(p).is_absolute() {
                            PathBuf::from(p)
                        } else {
                            root.join(p)
                        };
                        push_file_candidates(&base.join(import_path), &mut candidates);
                    }
                }
            }
        }

        for candidate in candidates {
            if let Some(file) = self.file(&candidate) {
                return Some(file);
            }
        }
        None
    }

    pub fn all_files(&self) -> impl Iterator<Item = &ParsedFile> {
        self.files.values()
    }

    /// Busca un símbolo por nombre en todo el workspace. Devuelve todas las
    /// coincidencias para que el caller pueda decidir si es única.
    pub fn find_global(&self, name: &str) -> Vec<(&ParsedFile, &LspSymbol)> {
        let mut out = Vec::new();
        for file in self.files.values() {
            if let Some(sym) = find_symbol_in(&file.symbols, name) {
                out.push((file, sym));
            }
        }
        out
    }

    #[allow(dead_code)]
    pub fn find_in_file<'a>(&self, file: &'a ParsedFile, name: &str) -> Option<&'a LspSymbol> {
        find_symbol_in(&file.symbols, name)
    }
}

pub fn parse_file(path: &Path, source: String) -> Option<ParsedFile> {
    let uri = path_to_uri(path)?;
    let mut lexer = Lexer::new(&source);
    let symbols = match lexer.tokenize() {
        Ok(tokens) => {
            let mut parser = Parser::new(tokens);
            match parser.parse() {
                Ok(module) => symbols_from_statements(&module.statements, &source),
                Err(_) => Vec::new(),
            }
        }
        Err(_) => Vec::new(),
    };
    let imports = imports_from_source(&source);
    let exports = exports_from_source(&source);

    Some(ParsedFile {
        path: normalize_path(path),
        uri,
        source,
        symbols,
        imports,
        exports,
    })
}

pub fn parse_source(source: &str) -> Vec<LspSymbol> {
    let mut lexer = Lexer::new(source);
    let Ok(tokens) = lexer.tokenize() else {
        return Vec::new();
    };
    let mut parser = Parser::new(tokens);
    let Ok(module) = parser.parse() else {
        return Vec::new();
    };
    symbols_from_statements(&module.statements, source)
}

fn imports_from_source(source: &str) -> Vec<ImportRef> {
    let mut lexer = Lexer::new(source);
    let Ok(tokens) = lexer.tokenize() else {
        return Vec::new();
    };
    let mut parser = Parser::new(tokens);
    let Ok(module) = parser.parse() else {
        return Vec::new();
    };
    imports_from_statements(&module.statements)
}

fn exports_from_source(source: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut lexer = Lexer::new(source);
    let Ok(tokens) = lexer.tokenize() else {
        return out;
    };
    let mut parser = Parser::new(tokens);
    let Ok(module) = parser.parse() else {
        return out;
    };
    for stmt in &module.statements {
        if let Statement::ExportList(el) = stmt {
            for spec in &el.specs {
                let name = spec
                    .exported
                    .clone()
                    .unwrap_or_else(|| export_ref_name(&spec.reference));
                if !name.is_empty() {
                    out.insert(name);
                }
            }
        }
    }
    out
}

pub fn imports_from_statements(statements: &[Statement]) -> Vec<ImportRef> {
    let mut out = Vec::new();
    for stmt in statements {
        match stmt {
            Statement::Import(i) => out.push(ImportRef {
                path: i.path.clone(),
                alias: i.alias.clone(),
                names: Vec::new(),
                span: i.span,
            }),
            Statement::FromImport(fi) => out.push(ImportRef {
                path: fi.path.clone(),
                alias: fi.namespace.clone(),
                names: fi
                    .names
                    .iter()
                    .map(|n| (n.name.clone(), n.alias.clone()))
                    .collect(),
                span: fi.span,
            }),
            Statement::ExportList(el) => {
                if let Some(path) = &el.from {
                    out.push(ImportRef {
                        path: path.clone(),
                        alias: None,
                        names: el
                            .specs
                            .iter()
                            .map(|s| (export_ref_name(&s.reference), s.exported.clone()))
                            .collect(),
                        span: el.span,
                    });
                }
            }
            _ => {}
        }
    }
    out
}

fn export_ref_name(r: &ExportRef) -> String {
    match r {
        ExportRef::Name(n) => n.clone(),
        ExportRef::NamespaceMember(_, n) => n.clone(),
        ExportRef::NamespaceAll(n) => n.clone(),
    }
}

pub fn symbols_from_statements(statements: &[Statement], source: &str) -> Vec<LspSymbol> {
    let mut out = Vec::new();
    for stmt in statements {
        match stmt {
            Statement::FunctionDecl(f) => out.push(function_symbol(f, source)),
            Statement::VarDecl(v) => out.push(var_symbol(v, source, SymbolKind::VARIABLE)),
            Statement::ConstDecl(v) => out.push(var_symbol(v, source, SymbolKind::CONSTANT)),
            Statement::StructureDecl(s) => {
                let children = s
                    .fields
                    .iter()
                    .map(|f| LspSymbol {
                        name: f.name.clone(),
                        kind: SymbolKind::FIELD,
                        detail: format!("{}: {}", f.name, type_ann_to_string(Some(&f.type_ann))),
                        doc: String::new(),
                        span: f.span,
                        ty: Some(type_ann_to_string(Some(&f.type_ann))),
                        children: Vec::new(),
                    })
                    .collect();
                out.push(LspSymbol {
                    name: s.name.clone(),
                    kind: SymbolKind::STRUCT,
                    detail: format!("structure {}", s.name),
                    doc: doc_before(source, s.span.start_line),
                    span: s.span,
                    ty: Some(s.name.clone()),
                    children,
                });
            }
            Statement::ClassDecl(c) => {
                let children = c
                    .body
                    .iter()
                    .map(|m| match m {
                        ClassMember::Property(v) => LspSymbol {
                            name: v.name.clone(),
                            kind: SymbolKind::PROPERTY,
                            detail: format!(
                                "{}: {}",
                                v.name,
                                type_ann_to_string(v.type_ann.as_ref())
                            ),
                            doc: doc_before(source, v.span.start_line),
                            span: v.span,
                            ty: v.type_ann.as_ref().map(|a| type_ann_to_string(Some(a))),
                            children: Vec::new(),
                        },
                        ClassMember::Method(f) => LspSymbol {
                            kind: SymbolKind::METHOD,
                            ..function_symbol(f, source)
                        },
                        ClassMember::Constructor(f) => LspSymbol {
                            kind: SymbolKind::CONSTRUCTOR,
                            ..function_symbol(f, source)
                        },
                    })
                    .collect();
                out.push(LspSymbol {
                    name: c.name.clone(),
                    kind: SymbolKind::CLASS,
                    detail: {
                        let ext = c.extends.as_deref().unwrap_or("");
                        if ext.is_empty() {
                            format!("class {}", c.name)
                        } else {
                            format!("class {} extends {}", c.name, ext)
                        }
                    },
                    doc: doc_before(source, c.span.start_line),
                    span: c.span,
                    ty: Some(c.name.clone()),
                    children,
                });
            }
            Statement::InterfaceDecl(i) => {
                let mut children = Vec::new();
                for f in &i.fields {
                    children.push(LspSymbol {
                        name: f.name.clone(),
                        kind: SymbolKind::FIELD,
                        detail: format!("{}: {}", f.name, type_ann_to_string(Some(&f.type_ann))),
                        doc: String::new(),
                        span: f.span,
                        ty: Some(type_ann_to_string(Some(&f.type_ann))),
                        children: Vec::new(),
                    });
                }
                for s in &i.signatures {
                    children.push(LspSymbol {
                        name: s.name.clone(),
                        kind: SymbolKind::METHOD,
                        detail: signature_decl_signature(s),
                        doc: doc_before(source, s.span.start_line),
                        span: s.span,
                        ty: s.return_type.as_ref().map(|a| type_ann_to_string(Some(a))),
                        children: Vec::new(),
                    });
                }
                out.push(LspSymbol {
                    name: i.name.clone(),
                    kind: SymbolKind::INTERFACE,
                    detail: format!("interface {}", i.name),
                    doc: doc_before(source, i.span.start_line),
                    span: i.span,
                    ty: Some(i.name.clone()),
                    children,
                });
            }
            Statement::EnumDecl(e) => {
                let children = e
                    .variants
                    .iter()
                    .map(|v| LspSymbol {
                        name: v.name.clone(),
                        kind: SymbolKind::ENUM_MEMBER,
                        detail: format!("{}::{}", e.name, v.name),
                        doc: String::new(),
                        span: v.span,
                        ty: Some(e.name.clone()),
                        children: Vec::new(),
                    })
                    .collect();
                out.push(LspSymbol {
                    name: e.name.clone(),
                    kind: SymbolKind::ENUM,
                    detail: format!("enum {}", e.name),
                    doc: doc_before(source, e.span.start_line),
                    span: e.span,
                    ty: Some(e.name.clone()),
                    children,
                });
            }
            Statement::TypeAlias(t) => out.push(LspSymbol {
                name: t.name.clone(),
                kind: SymbolKind::TYPE_PARAMETER,
                detail: format!(
                    "alias {} = {}",
                    t.name,
                    type_ann_to_string(Some(&t.type_ann))
                ),
                doc: doc_before(source, t.span.start_line),
                span: t.span,
                ty: Some(type_ann_to_string(Some(&t.type_ann))),
                children: Vec::new(),
            }),
            Statement::ModuleDecl(m) => out.push(LspSymbol {
                name: m.name.clone(),
                kind: SymbolKind::MODULE,
                detail: format!("module {}", m.name),
                doc: doc_before(source, m.span.start_line),
                span: m.span,
                ty: None,
                children: symbols_from_statements(&m.body, source),
            }),
            Statement::NamespaceDecl(ns) => out.push(LspSymbol {
                name: ns.name.clone(),
                kind: SymbolKind::NAMESPACE,
                detail: format!("namespace {}", ns.name),
                doc: doc_before(source, ns.span.start_line),
                span: ns.span,
                ty: None,
                children: symbols_from_statements(&ns.body, source),
            }),
            _ => {}
        }
    }
    out
}

pub(crate) fn function_symbol(f: &FunctionDecl, source: &str) -> LspSymbol {
    LspSymbol {
        name: f.name.clone(),
        kind: SymbolKind::FUNCTION,
        detail: function_signature(f),
        doc: doc_before(source, f.span.start_line),
        span: f.span,
        ty: f.return_type.as_ref().map(|a| type_ann_to_string(Some(a))),
        children: Vec::new(),
    }
}

pub(crate) fn infer_simple_type(expression: &Expression) -> Option<String> {
    match expression {
        Expression::Literal(literal) => match &literal.kind {
            LiteralKind::Int(_) => Some("int".to_string()),
            LiteralKind::Float(_) => Some("float".to_string()),
            LiteralKind::String(_) => Some("String".to_string()),
            LiteralKind::Bool(_) => Some("bool".to_string()),
            LiteralKind::Char(_) => Some("char".to_string()),
            LiteralKind::Null => Some("null".to_string()),
            LiteralKind::Unknown => None,
        },
        Expression::Call(call) => match &*call.callee {
            Expression::Identifier(name, _) if starts_uppercase(name) => Some(name.clone()),
            Expression::NamespaceAccess(_, member, _) if starts_uppercase(member) => {
                Some(member.clone())
            }
            _ => None,
        },
        Expression::Array(array) => {
            let mut element_type = None;
            for element in &array.elements {
                let ty = infer_simple_type(element)?;
                if let Some(previous) = &element_type {
                    if previous != &ty {
                        return None;
                    }
                } else {
                    element_type = Some(ty);
                }
            }
            element_type.map(|ty| format!("{}[]", ty))
        }
        Expression::Tuple(tuple) => {
            let types = tuple
                .elements
                .iter()
                .map(infer_simple_type)
                .collect::<Option<Vec<_>>>()?;
            Some(format!("({})", types.join(", ")))
        }
        Expression::Await(inner, _) | Expression::Parenthesized(inner, _) => {
            infer_simple_type(inner)
        }
        Expression::Conditional(conditional) => {
            let then_ty = infer_simple_type(&conditional.then_expr)?;
            let else_ty = infer_simple_type(&conditional.else_expr)?;
            (then_ty == else_ty).then_some(then_ty)
        }
        Expression::Ternary(ternary) => {
            let then_ty = infer_simple_type(&ternary.then_expr)?;
            let else_ty = infer_simple_type(&ternary.else_expr)?;
            (then_ty == else_ty).then_some(then_ty)
        }
        _ => None,
    }
}

fn starts_uppercase(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

pub(crate) fn var_symbol(v: &VarDecl, source: &str, kind: SymbolKind) -> LspSymbol {
    let ty = v
        .type_ann
        .as_ref()
        .map(|a| type_ann_to_string(Some(a)))
        .or_else(|| v.value.as_ref().and_then(infer_simple_type));
    LspSymbol {
        name: v.name.clone(),
        kind,
        detail: match &ty {
            Some(t) => format!("{}: {}", v.name, t),
            None => v.name.clone(),
        },
        doc: doc_before(source, v.span.start_line),
        span: v.span,
        ty,
        children: Vec::new(),
    }
}

pub fn function_signature(f: &FunctionDecl) -> String {
    let params = f
        .params
        .iter()
        .map(|p| {
            let ty = p
                .type_ann
                .as_ref()
                .map(|a| type_ann_to_string(Some(a)))
                .unwrap_or_else(|| "Any".to_string());
            format!("{}: {}", p.name, ty)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret = f
        .return_type
        .as_ref()
        .map(|a| format!(" -> {}", type_ann_to_string(Some(a))))
        .unwrap_or_default();
    let async_kw = if f
        .modifiers
        .iter()
        .any(|m| matches!(m, FunctionModifier::Async))
    {
        "async "
    } else {
        ""
    };
    format!("{}{}({}){}", async_kw, f.name, params, ret)
}

fn signature_decl_signature(s: &SignatureDecl) -> String {
    let params = s
        .params
        .iter()
        .map(|p| {
            let ty = p
                .type_ann
                .as_ref()
                .map(|a| type_ann_to_string(Some(a)))
                .unwrap_or_else(|| "Any".to_string());
            format!("{}: {}", p.name, ty)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret = s
        .return_type
        .as_ref()
        .map(|a| format!(" -> {}", type_ann_to_string(Some(a))))
        .unwrap_or_default();
    format!("{}({}){}", s.name, params, ret)
}

pub fn type_ann_to_string(ann: Option<&TypeAnnotation>) -> String {
    let Some(ann) = ann else {
        return "Any".to_string();
    };
    match &ann.kind {
        TypeKind::Int => "int".into(),
        TypeKind::Float => "float".into(),
        TypeKind::String => "String".into(),
        TypeKind::Bool => "bool".into(),
        TypeKind::Char => "char".into(),
        TypeKind::Any => "Any".into(),
        TypeKind::Unknown => "Unknown".into(),
        TypeKind::Null => "null".into(),
        TypeKind::Void => "void".into(),
        TypeKind::Empty => "".into(),
        TypeKind::Json => "Json".into(),
        TypeKind::Value => "Value".into(),
        TypeKind::Array(inner) => format!("{}[]", type_ann_to_string(Some(inner.as_ref()))),
        TypeKind::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(|a| type_ann_to_string(Some(a)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypeKind::Union(items) => items
            .iter()
            .map(|a| type_ann_to_string(Some(a)))
            .collect::<Vec<_>>()
            .join(" | "),
        TypeKind::Fun(params, ret) => format!(
            "({}) -> {}",
            params
                .iter()
                .map(|a| type_ann_to_string(Some(a)))
                .collect::<Vec<_>>()
                .join(", "),
            type_ann_to_string(Some(ret.as_ref()))
        ),
        TypeKind::Named(name, args) => {
            if args.is_empty() {
                name.clone()
            } else {
                format!(
                    "{}<{}>",
                    name,
                    args.iter()
                        .map(|a| type_ann_to_string(Some(a)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        other => format!("{:?}", other),
    }
}

pub fn doc_before(source: &str, line_1based: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut idx = line_1based.saturating_sub(2) as isize;
    let mut docs = Vec::new();
    while idx >= 0 {
        let text = lines.get(idx as usize).copied().unwrap_or("").trim();
        if text.starts_with("# @") || text.starts_with("#@") {
            docs.push(text.to_string());
        } else if text.is_empty() || text.starts_with('#') {
            // Comentario normal o línea en blanco: seguir subiendo.
        } else {
            break;
        }
        idx -= 1;
    }
    docs.reverse();
    docs.join("\n")
}

pub fn members_from_type_module(tm: &TypeModule) -> Vec<MemberInfo> {
    tm.members
        .iter()
        .map(|m| MemberInfo {
            name: m.name.clone(),
            kind: symbol_kind_for_member(&m.kind),
            detail: m.signature.clone(),
            doc: m.doc.clone(),
            ty: m.return_type.clone(),
            children: Vec::new(),
        })
        .collect()
}

pub fn symbol_kind_for_member(kind: &MemberKind) -> SymbolKind {
    match kind {
        MemberKind::Function => SymbolKind::FUNCTION,
        MemberKind::Variable => SymbolKind::VARIABLE,
        MemberKind::Constant => SymbolKind::CONSTANT,
        MemberKind::Class => SymbolKind::CLASS,
        MemberKind::Structure => SymbolKind::STRUCT,
        MemberKind::Interface => SymbolKind::INTERFACE,
        MemberKind::Module => SymbolKind::MODULE,
        MemberKind::Namespace => SymbolKind::NAMESPACE,
        MemberKind::Type => SymbolKind::TYPE_PARAMETER,
        MemberKind::Enum => SymbolKind::ENUM,
    }
}

pub fn find_symbol_in<'a>(symbols: &'a [LspSymbol], name: &str) -> Option<&'a LspSymbol> {
    for symbol in symbols {
        if symbol.name == name {
            return Some(symbol);
        }
        if let Some(found) = find_symbol_in(&symbol.children, name) {
            return Some(found);
        }
    }
    None
}

pub fn find_member<'a>(members: &'a [MemberInfo], name: &str) -> Option<&'a MemberInfo> {
    members.iter().find(|m| m.name == name)
}

fn is_cls_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("clsx" | "clsi")
    )
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".vscode"
            | ".cls-cache"
            | ".cls-types"
            | "node_modules"
            | "target"
            | "out"
            | "dist"
            | ".resources"
    )
}

fn push_file_candidates(base: &Path, out: &mut Vec<PathBuf>) {
    if base.extension().is_some() {
        out.push(base.to_path_buf());
        return;
    }
    if base.is_dir() {
        push_manifest_entry(base, out);
    }
    out.push(base.with_extension("clsx"));
    out.push(base.with_extension("clsi"));
    out.push(base.join("mod.clsx"));
    out.push(base.join("mod.clsi"));
    out.push(base.join("index.clsx"));
    out.push(base.join("index.clsi"));
}

fn push_manifest_entry(dir: &Path, out: &mut Vec<PathBuf>) {
    let path = dir.join("cls.json");
    if !path.exists() {
        return;
    }
    if let Ok(manifest) = ModuleManifest::from_file(&path) {
        if let Some(export) = manifest.export_config {
            if !export.entry.is_empty() {
                out.push(dir.join(export.entry));
            }
        }
    }
}

pub fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn path_to_uri(path: &Path) -> Option<Url> {
    Url::from_file_path(path).ok().or_else(|| {
        let text = path.to_string_lossy();
        let stripped = text.strip_prefix(r"\\?\").unwrap_or(&text);
        Url::from_file_path(stripped).ok()
    })
}

fn module_name_for_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if stem == "mod" || stem == "index" {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or(&stem)
            .to_string()
    } else {
        stem
    }
}
