use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
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
    pub signature: Option<String>,
    pub params: Vec<ParamInfo>,
    pub return_type: Option<String>,
    pub fields: Vec<FieldInfo>,
    pub members: Vec<String>,
    pub type_: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamInfo {
    pub name: String,
    pub type_: Option<String>,
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

fn extract_doc(source: &str, line: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if line == 0 || line as usize > lines.len() { return String::new(); }
    let mut docs = Vec::new();
    let idx = line.saturating_sub(1) as usize;
    for i in (0..idx).rev() {
        let l = lines[i].trim();
        if l.starts_with("# @") || l.starts_with("#@") {
            docs.insert(0, l.to_string());
        } else if l.starts_with('#') || l.is_empty() {
            continue;
        } else {
            break;
        }
    }
    docs.join("\n")
}

fn type_ann_to_string(ann: &Option<TypeAnnotation>) -> Option<String> {
    ann.as_ref().map(|a| match &a.kind {
        TypeKind::Named(n, _) => n.clone(),
        TypeKind::Int => "int".to_string(),
        TypeKind::Float => "float".to_string(),
        TypeKind::String => "String".to_string(),
        TypeKind::Bool => "bool".to_string(),
        TypeKind::Any => "Any".to_string(),
        TypeKind::Void => "void".to_string(),
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
                let params: Vec<ParamInfo> = f.params.iter().map(|p| ParamInfo {
                    name: p.name.clone(),
                    type_: type_ann_to_string(&p.type_ann),
                }).collect();
                entries.push(TypeEntry {
                    name: f.name.clone(),
                    kind: if f.modifiers.iter().any(|m| matches!(m, FunctionModifier::Async)) { "async function".to_string() } else { "function".to_string() },
                    line: f.span.start_line, col: f.span.start_col,
                    end_line: f.span.end_line, end_col: f.span.end_col,
                    doc: extract_doc(source, f.span.start_line),
                    signature: Some(format!("{}({})", f.name, params.iter().map(|p| format!("{}: {}", p.name, p.type_.as_deref().unwrap_or("Any"))).collect::<Vec<_>>().join(", "))),
                    params,
                    return_type: type_ann_to_string(&f.return_type),
                    fields: vec![],
                    members: vec![],
                    type_: None,
                    value: None,
                });
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                entries.push(TypeEntry {
                    name: v.name.clone(),
                    kind: "variable".to_string(),
                    line: v.span.start_line, col: v.span.start_col,
                    end_line: v.span.end_line, end_col: v.span.end_col,
                    doc: extract_doc(source, v.span.start_line),
                    signature: None,
                    params: vec![], return_type: None,
                    fields: vec![],
                    members: vec![],
                    type_: type_ann_to_string(&v.type_ann),
                    value: None, // podriamos evaluar literales aqui
                });
            }
            Statement::StructureDecl(s) => {
                let fields: Vec<FieldInfo> = s.fields.iter().map(|f| FieldInfo {
                    name: f.name.clone(),
                    type_: type_ann_to_string(&Some(f.type_ann.clone())),
                }).collect();
                entries.push(TypeEntry {
                    name: s.name.clone(),
                    kind: "structure".to_string(),
                    line: s.span.start_line, col: s.span.start_col,
                    end_line: s.span.end_line, end_col: s.span.end_col,
                    doc: extract_doc(source, s.span.start_line),
                    signature: None,
                    params: vec![], return_type: None,
                    fields,
                    members: vec![],
                    type_: None, value: None,
                });
            }
            Statement::InterfaceDecl(i) => {
                let members: Vec<String> = i.signatures.iter().map(|s| s.name.clone()).collect();
                entries.push(TypeEntry {
                    name: i.name.clone(),
                    kind: "interface".to_string(),
                    line: i.span.start_line, col: i.span.start_col,
                    end_line: i.span.end_line, end_col: i.span.end_col,
                    doc: extract_doc(source, i.span.start_line),
                    signature: None,
                    params: vec![], return_type: None,
                    fields: vec![],
                    members,
                    type_: None, value: None,
                });
            }
            Statement::Import(i) => {
                let alias = i.alias.as_deref().unwrap_or(&i.path);
                entries.push(TypeEntry {
                    name: alias.to_string(),
                    kind: "import".to_string(),
                    line: i.span.start_line, col: i.span.start_col,
                    end_line: i.span.end_line, end_col: i.span.end_col,
                    doc: String::new(),
                    signature: Some(format!("import \"{}\" as {}", i.path, alias)),
                    params: vec![], return_type: None,
                    fields: vec![],
                    members: vec![],
                    type_: None, value: None,
                });
            }
            Statement::FromImport(fi) => {
                for im in &fi.names {
                    let alias = im.alias.as_deref().unwrap_or(&im.name);
                    entries.push(TypeEntry {
                        name: alias.to_string(),
                        kind: "import".to_string(),
                        line: fi.span.start_line, col: fi.span.start_col,
                        end_line: fi.span.end_line, end_col: fi.span.end_col,
                        doc: String::new(),
                        signature: Some(format!("from \"{}\" import {}", fi.path, alias)),
                        params: vec![], return_type: None,
                        fields: vec![],
                        members: vec![],
                        type_: None, value: None,
                    });
                }
            }
            Statement::ClassDecl(c) => {
                entries.push(TypeEntry {
                    name: c.name.clone(),
                    kind: "class".to_string(),
                    line: c.span.start_line, col: c.span.start_col,
                    end_line: c.span.end_line, end_col: c.span.end_col,
                    doc: extract_doc(source, c.span.start_line),
                    signature: None, params: vec![], return_type: None,
                    fields: vec![], members: vec![],
                    type_: None, value: None,
                });
            }
            Statement::ModuleDecl(md) => {
                entries.push(TypeEntry {
                    name: md.name.clone(),
                    kind: "module".to_string(),
                    line: md.span.start_line, col: md.span.start_col,
                    end_line: md.span.end_line, end_col: md.span.end_col,
                    doc: extract_doc(source, md.span.start_line),
                    signature: None, params: vec![], return_type: None,
                    fields: vec![], members: vec![],
                    type_: None, value: None,
                });
            }
            _ => {}
        }
    }

    TypeMap { source: src_path.to_string(), entries }
}

fn process_file(input: &Path, output: &Path) {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", input.display(), e); return; }
    };
    let map = generate_type_map(&source, &input.to_string_lossy());
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&map).unwrap();
    fs::write(output, &json).unwrap();
    println!("  {} -> {} ({} entradas)", input.display(), output.display(), map.entries.len());
}

fn process_dir(input_dir: &Path, output_dir: &Path) {
    if let Ok(entries) = fs::read_dir(input_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                process_dir(&path, &output_dir.join(path.file_name().unwrap()));
            } else {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "clsx" || ext == "clsi" {
                    let stem = path.file_stem().unwrap().to_string_lossy();
                    let out = output_dir.join(format!("{}.type.json", stem));
                    process_file(&path, &out);
                }
            }
        }
    }
}

pub fn execute(args: &[String]) -> i32 {
    let input = args.iter().find(|a| !a.starts_with("-") && *a != "." && !args.iter().position(|x| x == "-o").map_or(false, |i| args.get(i+1).map_or(false, |v| v == a.as_str())) ).cloned().unwrap_or_else(|| ".".to_string());
    let output = args.iter().position(|a| a == "-o" || a == "--out").and_then(|i| args.get(i+1)).cloned().unwrap_or_else(|| "./.clsi-types".to_string());
    let watch = args.iter().any(|a| a == "--watch" || a == "-w");

    let input_path = Path::new(&input);
    let output_path = Path::new(&output);

    if input_path.is_dir() {
        eprintln!("Generando type maps desde '{}' -> '{}'...", input, output);
        process_dir(input_path, &output_path.join(".clsi-types"));
        if watch {
            eprintln!("Watch mode no implementado aún (solo generación única)");
        }
    } else {
        if input_path.extension().map(|e| e == "clsx" || e == "clsi").unwrap_or(false) {
            let target = if output_path.is_dir() {
                let stem = input_path.file_stem().unwrap().to_string_lossy();
                output_path.join(format!("{}.type.json", stem))
            } else {
                output_path.to_path_buf()
            };
            process_file(input_path, &target);
        } else {
            eprintln!("Error: '{}' no es un archivo .clsx o .clsi", input);
            return 1;
        }
    }

    eprintln!("Completado.");
    0
}
