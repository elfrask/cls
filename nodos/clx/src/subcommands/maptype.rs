use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use cls_core::frontend::{Lexer, Parser};
use cls_core::frontend::ast::*;

#[derive(Debug, Clone, Serialize)]
pub struct TypeEntry {
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub doc: String,
    pub version: Option<String>,
    pub deprecated: Option<String>,
    pub signature: Option<String>,
    pub params: Vec<ParamInfo>,
    pub return_type: Option<String>,
    pub return_doc: Option<String>,
    pub fields: Vec<FieldInfo>,
    pub members: Vec<String>,
    pub type_: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamInfo {
    pub name: String,
    pub type_: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeMap {
    pub source: String,
    pub entries: Vec<TypeEntry>,
}

/// Parsea documentación `# @prefijo ...` antes de una declaración en el source.
/// Busca por nombre de declaración para evitar problemas con spans del parser.
fn parse_doc_for(source: &str, decl_name: &str, decl_kind: &str) -> (String, Option<String>, Option<String>) {
    let lines: Vec<&str> = source.lines().collect();
    let mut target_line = 0;

    // Buscar la línea de la declaración: "function NOMBRE" o "var NOMBRE"
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let pattern = format!("{} {}", decl_kind, decl_name);
        let alt_pattern = format!("{}{}", decl_kind, decl_name);
        if trimmed.starts_with(&pattern) || trimmed.contains(&alt_pattern) {
            target_line = i + 1; // 1-indexed
            break;
        }
    }

    if target_line == 0 {
        // Fallback a buscar por nombre de función/variable
        for (i, line) in lines.iter().enumerate() {
            if line.contains(decl_name) && (line.contains("function ") || line.contains("var ") || line.contains("const ")) {
                target_line = i + 1;
                break;
            }
        }
    }

    if target_line == 0 { return (String::new(), None, None); }

    let mut docs = Vec::new();
    let mut version = None;
    let mut deprecated = None;
    let idx = (target_line as usize).saturating_sub(1);

    for i in (0..idx).rev() {
        let l = lines[i].trim();
        if l.starts_with("# @title") {
            // Module title separates module header from function docs
            break;
        }
        if l.starts_with("# @") || l.starts_with("#@") {
            docs.insert(0, l.to_string());
            let content = l.trim_start_matches("# @").trim_start_matches("#@");
            if let Some(v) = content.strip_prefix("version ") { version = Some(v.trim().to_string()); }
            if let Some(d) = content.strip_prefix("deprecated ") { deprecated = Some(d.trim().to_string()); }
        } else if l.starts_with('#') || l.is_empty() {
            continue;
        } else {
            break;
        }
    }

    (docs.join("\n"), version, deprecated)
}

/// Extrae descripción desde doc (solo el último @description, el más cercano a la declaración)
fn extract_description(doc: &str) -> String {
    let mut last = String::new();
    for line in doc.lines() {
        let clean = line.trim_start_matches('#').trim();
        if let Some(rest) = clean.strip_prefix("@description ") {
            last = rest.to_string();
        }
    }
    last
}

/// Extrae documentación de @params
fn extract_param_doc(doc: &str, param_name: &str) -> Option<String> {
    for line in doc.lines() {
        let clean = line.trim_start_matches('#').trim();
        if let Some(rest) = clean.strip_prefix("@params ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() >= 2 && parts[0] == param_name {
                return Some(parts[1].to_string());
            }
        }
    }
    None
}

/// Extrae documentación de @return
fn extract_return_doc(doc: &str) -> Option<String> {
    for line in doc.lines() {
        let clean = line.trim_start_matches('#').trim();
        if let Some(rest) = clean.strip_prefix("@return ") {
            // "tipo descripcion" o solo "descripcion"
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                return Some(parts[1..].join(" "));
            }
        }
    }
    None
}

fn type_ann_to_string(ann: &Option<TypeAnnotation>) -> Option<String> {
    ann.as_ref().map(|a| match &a.kind {
        TypeKind::Named(n, _) => n.clone(),
        TypeKind::Int => "int".to_string(), TypeKind::Float => "float".to_string(),
        TypeKind::String => "String".to_string(), TypeKind::Bool => "bool".to_string(),
        TypeKind::Any => "Any".to_string(), TypeKind::Void => "void".to_string(),
        TypeKind::Array(inner) => format!("Array<{}>", type_ann_to_string(&Some(*inner.clone())).unwrap_or_default()),
        _ => "Any".to_string(),
    })
}

fn generate_type_map(source: &str, src_path: &str) -> TypeMap {
    let mut entries = Vec::new();
    let toks = match Lexer::new(source).tokenize() { Ok(t) => t, Err(_) => return TypeMap { source: src_path.to_string(), entries } };
    let module = match Parser::new(toks).parse() { Ok(m) => m, Err(_) => return TypeMap { source: src_path.to_string(), entries } };

    for stmt in &module.statements {
        match stmt {
            Statement::FunctionDecl(f) => {
                let (raw_doc, version, deprecated) = parse_doc_for(source, &f.name, "function");
                let return_doc_val = extract_return_doc(&raw_doc);
                let params: Vec<ParamInfo> = f.params.iter().map(|p| ParamInfo {
                    name: p.name.clone(),
                    type_: type_ann_to_string(&p.type_ann),
                    doc: extract_param_doc(&raw_doc, &p.name),
                }).collect();
                let async_kw = if f.modifiers.iter().any(|m| matches!(m, FunctionModifier::Async)) { "async " } else { "" };
                let sig = format!("{}{}({}){}", async_kw, f.name,
                    params.iter().map(|p| format!("{}: {}", p.name, p.type_.as_deref().unwrap_or("Any"))).collect::<Vec<_>>().join(", "),
                    type_ann_to_string(&f.return_type).map(|rt| format!(" -> {}", rt)).unwrap_or_default());
                entries.push(TypeEntry {
                    name: f.name.clone(),
                    kind: if async_kw.is_empty() { "function".to_string() } else { "async function".to_string() },
                    line: f.span.start_line, col: f.span.start_col, end_line: f.span.end_line, end_col: f.span.end_col,
                    doc: extract_description(&raw_doc), version, deprecated,
                    signature: Some(sig), params, return_type: type_ann_to_string(&f.return_type),
                    return_doc: return_doc_val, fields: vec![], members: vec![], type_: None, value: None,
                });
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                let (raw_doc, version, deprecated) = parse_doc_for(source, &v.name, "var");
                let kind_str = match stmt { Statement::ConstDecl(_) => "constant", _ => "variable" };
                entries.push(TypeEntry {
                    name: v.name.clone(), kind: kind_str.to_string(),
                    line: v.span.start_line, col: v.span.start_col, end_line: v.span.end_line, end_col: v.span.end_col,
                    doc: extract_description(&raw_doc), version, deprecated,
                    signature: None, params: vec![], return_type: None, return_doc: None,
                    fields: vec![], members: vec![], type_: type_ann_to_string(&v.type_ann), value: None,
                });
            }
            Statement::StructureDecl(s) => {
                let (raw_doc, version, deprecated) = parse_doc_for(source, &s.name, "structure");
                let fields: Vec<FieldInfo> = s.fields.iter().map(|f| FieldInfo {
                    name: f.name.clone(), type_: type_ann_to_string(&Some(f.type_ann.clone())),
                }).collect();
                entries.push(TypeEntry {
                    name: s.name.clone(), kind: "structure".to_string(),
                    line: s.span.start_line, col: s.span.start_col, end_line: s.span.end_line, end_col: s.span.end_col,
                    doc: extract_description(&raw_doc), version, deprecated,
                    signature: None, params: vec![], return_type: None, return_doc: None,
                    fields, members: vec![], type_: None, value: None,
                });
            }
            Statement::InterfaceDecl(i) => {
                let (raw_doc, _, _) = parse_doc_for(source, &i.name, "interface");
                let members: Vec<String> = i.signatures.iter().map(|s| s.name.clone()).collect();
                entries.push(TypeEntry {
                    name: i.name.clone(), kind: "interface".to_string(),
                    line: i.span.start_line, col: i.span.start_col, end_line: i.span.end_line, end_col: i.span.end_col,
                    doc: extract_description(&raw_doc), version: None, deprecated: None,
                    signature: None, params: vec![], return_type: None, return_doc: None,
                    fields: vec![], members, type_: None, value: None,
                });
            }
            Statement::Import(i) => {
                let alias = i.alias.as_deref().unwrap_or(&i.path);
                entries.push(TypeEntry {
                    name: alias.to_string(), kind: "import".to_string(),
                    line: i.span.start_line, col: i.span.start_col, end_line: i.span.end_line, end_col: i.span.end_col,
                    doc: String::new(), version: None, deprecated: None,
                    signature: Some(format!("import \"{}\" as {}", i.path, alias)),
                    params: vec![], return_type: None, return_doc: None, fields: vec![], members: vec![],
                    type_: None, value: None,
                });
            }
            Statement::FromImport(fi) => {
                for im in &fi.names {
                    let alias = im.alias.as_deref().unwrap_or(&im.name);
                    entries.push(TypeEntry {
                        name: alias.to_string(), kind: "import".to_string(),
                        line: fi.span.start_line, col: fi.span.start_col,
                        end_line: fi.span.end_line, end_col: fi.span.end_col,
                        doc: String::new(), version: None, deprecated: None,
                        signature: Some(format!("from \"{}\" import {}", fi.path, alias)),
                        params: vec![], return_type: None, return_doc: None, fields: vec![], members: vec![],
                        type_: None, value: None,
                    });
                }
            }
            Statement::ClassDecl(c) => {
                let (raw_doc, version, deprecated) = parse_doc_for(source, &c.name, "class");
                // Extraer propiedades y métodos de la clase
                let mut fields: Vec<FieldInfo> = Vec::new();
                let mut members: Vec<String> = Vec::new();
                for member in &c.body {
                    match member {
                        ClassMember::Property(v) => {
                            fields.push(FieldInfo { name: v.name.clone(), type_: type_ann_to_string(&v.type_ann) });
                            members.push(v.name.clone());
                        }
                        ClassMember::Method(f) | ClassMember::Constructor(f) => {
                            members.push(f.name.clone());
                        }
                    }
                }
                let extends = c.extends.as_ref().map(|e| e.clone()).unwrap_or_default();
                entries.push(TypeEntry {
                    name: c.name.clone(), kind: "class".to_string(),
                    line: c.span.start_line, col: c.span.start_col, end_line: c.span.end_line, end_col: c.span.end_col,
                    doc: extract_description(&raw_doc), version, deprecated,
                    signature: if extends.is_empty() { None } else { Some(format!("class {} extends {}", c.name, extends)) },
                    params: vec![], return_type: None, return_doc: None,
                    fields, members, type_: None, value: None,
                });
            }
            Statement::ModuleDecl(md) => {
                let (raw_doc, version, deprecated) = parse_doc_for(source, &md.name, "module");
                let members: Vec<String> = md.body.iter()
                    .filter_map(|s| match s {
                        Statement::FunctionDecl(f) => Some(f.name.clone()),
                        Statement::VarDecl(v) | Statement::ConstDecl(v) => Some(v.name.clone()),
                        _ => None,
                    }).collect();
                entries.push(TypeEntry {
                    name: md.name.clone(), kind: "module".to_string(),
                    line: md.span.start_line, col: md.span.start_col, end_line: md.span.end_line, end_col: md.span.end_col,
                    doc: extract_description(&raw_doc), version, deprecated,
                    signature: None, params: vec![], return_type: None, return_doc: None,
                    fields: vec![], members, type_: None, value: None,
                });
            }
            Statement::NamespaceDecl(ns) => {
                let (raw_doc, version, deprecated) = parse_doc_for(source, &ns.name, "namespace");
                let members: Vec<String> = ns.body.iter()
                    .filter_map(|s| match s {
                        Statement::FunctionDecl(f) => Some(f.name.clone()),
                        Statement::VarDecl(v) | Statement::ConstDecl(v) => Some(v.name.clone()),
                        _ => None,
                    }).collect();
                entries.push(TypeEntry {
                    name: ns.name.clone(), kind: "namespace".to_string(),
                    line: ns.span.start_line, col: ns.span.start_col, end_line: ns.span.end_line, end_col: ns.span.end_col,
                    doc: extract_description(&raw_doc), version, deprecated,
                    signature: None, params: vec![], return_type: None, return_doc: None,
                    fields: vec![], members, type_: None, value: None,
                });
            }
            _ => {}
        }
    }
    TypeMap { source: src_path.to_string(), entries }
}

fn process_file(input: &Path, output: &Path) -> bool {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", input.display(), e); return false; }
    };
    let map = generate_type_map(&source, &input.to_string_lossy());
    if let Some(parent) = output.parent() { fs::create_dir_all(parent).ok(); }
    let json = serde_json::to_string_pretty(&map).unwrap();
    fs::write(output, &json).unwrap();
    println!("  {} -> {} ({} entradas)", input.display(), output.display(), map.entries.len());
    true
}

fn process_dir(input_dir: &Path, output_dir: &Path) {
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Ok(entries) = fs::read_dir(input_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy();
                if name.starts_with('.') || name == "modules" || name == "dist" || name == "libs" || name == "target" { continue; }
                process_dir(&path, output_dir);
            } else if matches!(path.extension().and_then(|e| e.to_str()), Some("clsx" | "clsi")) {
                let stem = path.file_stem().unwrap().to_string_lossy();
                // Ruta relativa desde CWD para preservar estructura
                let rel = path.strip_prefix(&cwd).unwrap_or(&path);
                let parent = rel.parent().unwrap_or(Path::new(""));
                let full_out = output_dir.join(parent).join(format!("{}.type.json", stem));
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(output_dir.join(parent)).ok();
                }
                process_file(&path, &full_out);
            }
        }
    }
}

fn get_mtime(path: &Path) -> u64 {
    fs::metadata(path).and_then(|m| m.modified().or_else(|_| Ok(SystemTime::UNIX_EPOCH)))
        .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
        .unwrap_or(0)
}

fn watch_dir(input_dir: &Path, output_dir: &Path) {
    eprintln!("  Watch mode activo (polling cada 2s)...");
    let mut last: HashMap<PathBuf, u64> = HashMap::new();
    loop {
        scan_and_process(input_dir, output_dir, &mut last);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn scan_and_process(dir: &Path, out_base: &Path, last: &mut HashMap<PathBuf, u64>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(std::env::current_dir().unwrap_or_default()).unwrap_or(&path);
            if path.is_dir() {
                let name = path.file_name().unwrap();
                let n = name.to_string_lossy();
                if n.starts_with('.') || n == "modules" || n == "dist" || n == "libs" { continue; }
                scan_and_process(&path, out_base, last);
            } else if matches!(path.extension().and_then(|e| e.to_str()), Some("clsx" | "clsi")) {
                let mtime = get_mtime(&path);
                let prev = *last.get(&path).unwrap_or(&0);
                if mtime > prev {
                    // Preservar estructura relativa al workspace root
                    let rel_path = path.strip_prefix(std::env::current_dir().unwrap_or_default())
                        .unwrap_or(&path).to_path_buf();
                    let parent = rel_path.parent().unwrap_or(Path::new(""));
                    let stem = path.file_stem().unwrap().to_string_lossy();
                    let out = out_base.join(parent).join(format!("{}.type.json", stem));
                    process_file(&path, &out);
                    last.insert(path, mtime);
                }
            }
        }
    }
}

pub fn execute(args: &[String]) -> i32 {
    let input = args.iter().find(|a| !a.starts_with("-") && *a != "." && !args.iter().position(|x| x == "-o").map_or(false, |i| args.get(i+1).map_or(false, |v| v == a.as_str()))).cloned().unwrap_or_else(|| ".".to_string());
    let output = args.iter().position(|a| a == "-o" || a == "--out").and_then(|i| args.get(i+1)).cloned().unwrap_or_else(|| "./.cls-types".to_string());
    let watch = args.iter().any(|a| a == "--watch" || a == "-w");

    let input_path = Path::new(&input);
    let output_path = Path::new(&output);

    if input_path.is_dir() {
        eprintln!("Generando type maps desde '{}' -> '{}'...", input, output);
        let out_dir = output_path.to_path_buf();
        process_dir(input_path, &out_dir);
        if watch { watch_dir(input_path, &out_dir); }
    } else {
        if matches!(input_path.extension().and_then(|e| e.to_str()), Some("clsx" | "clsi")) {
            let target = if output_path.is_dir() {
                let stem = input_path.file_stem().unwrap().to_string_lossy();
                output_path.join(format!("{}.type.json", stem))
            } else { output_path.to_path_buf() };
            process_file(input_path, &target);
            if watch {
                eprintln!("  Watch mode activo en '{}'...", input);
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let mtime = get_mtime(input_path);
                    let prev = get_mtime(&target);
                    if mtime > prev { process_file(input_path, &target); }
                }
            }
        } else {
            eprintln!("Error: '{}' no es un archivo .clsx o .clsi", input);
            return 1;
        }
    }

    eprintln!("Completado.");
    0
}
