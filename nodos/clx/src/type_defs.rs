use std::collections::HashMap;
use std::path::Path;
use cls_core::error::Span;
use cls_core::frontend::{Lexer, Parser};
use cls_core::frontend::ast::*;

/// Representa una declaración de tipo desde un archivo .clsi
#[derive(Debug, Clone)]
pub struct TypeMember {
    pub name: String,
    pub kind: MemberKind,
    pub signature: String,
    pub return_type: Option<String>,
    pub params: Vec<(String, String)>,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberKind {
    Function,
    Variable,
    Constant,
    Class,
    Structure,
    Interface,
    Module,
    Namespace,
}

/// Representa un módulo de tipos completo
#[derive(Debug, Clone)]
pub struct TypeModule {
    pub name: String,
    pub description: String,
    pub version: String,
    pub members: Vec<TypeMember>,
}

impl TypeModule {
    pub fn member_names(&self) -> Vec<&str> {
        self.members.iter().map(|m| m.name.as_str()).collect()
    }
}

/// Built-in type definitions embebidos en el binario
/// Se distribuyen con el ejecutable via include_str!
fn builtin_type_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("core", include_str!("../../../cls-runtime/clsi/core.clsi")),
        ("math", include_str!("../../../cls-runtime/clsi/math.clsi")),
        ("json", include_str!("../../../cls-runtime/clsi/json.clsi")),
        ("fs", include_str!("../../../cls-runtime/clsi/fs.clsi")),
        ("http", include_str!("../../../cls-runtime/clsi/http.clsi")),
        ("Lib", include_str!("../../../cls-runtime/clsi/Lib.clsi")),
        ("async", include_str!("../../../cls-runtime/clsi/async.clsi")),
    ]
}

/// Parsea un archivo .clsi a un TypeModule
pub fn parse_clsi(source: &str, module_name: &str) -> Option<TypeModule> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().ok()?;
    let mut parser = Parser::new(tokens);
    let module = parser.parse().ok()?;
    Some(extract_module(&module, module_name, source))
}

fn extract_module(module: &Module, name: &str, source: &str) -> TypeModule {
    let mut type_mod = TypeModule {
        name: name.to_string(),
        description: extract_module_doc(module, source),
        version: String::new(),
        members: Vec::new(),
    };
    for stmt in &module.statements {
        let span = statement_span(stmt);
        match stmt {
            Statement::FunctionDecl(f) => {
                type_mod.members.push(extract_function_member(f, source));
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v) => {
                type_mod.members.push(extract_var_member(v, source));
            }
            Statement::ClassDecl(c) => {
                type_mod.members.push(extract_class_member(c, source));
            }
            Statement::StructureDecl(s) => {
                type_mod.members.push(extract_structure_member(s, source));
            }
            Statement::InterfaceDecl(i) => {
                type_mod.members.push(extract_interface_member(i, source));
            }
            Statement::ModuleDecl(md) => {
                type_mod.members.push(extract_container_member(&md.name, MemberKind::Module, &md.body, source));
            }
            Statement::NamespaceDecl(ns) => {
                type_mod.members.push(extract_container_member(&ns.name, MemberKind::Namespace, &ns.body, source));
            }
            _ => { let _ = span; }
        }
    }
    type_mod
}

fn extract_class_member(c: &ClassDecl, source: &str) -> TypeMember {
    let mut members: Vec<String> = Vec::new();
    for member in &c.body {
        match member {
            ClassMember::Property(v) => members.push(v.name.clone()),
            ClassMember::Method(f) | ClassMember::Constructor(f) => members.push(f.name.clone()),
        }
    }
    let extends = c.extends.as_deref().unwrap_or("");
    let sig = if extends.is_empty() {
        format!("class {}", c.name)
    } else {
        format!("class {} extends {}", c.name, extends)
    };
    TypeMember {
        name: c.name.clone(),
        kind: MemberKind::Class,
        signature: sig,
        return_type: None,
        params: vec![],
        doc: format!("{} ({} members)", extract_doc_before(source, c.span.start_line, c.span.start_col), members.len()),
    }
}

fn extract_structure_member(s: &StructureDecl, source: &str) -> TypeMember {
    let fields: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
    TypeMember {
        name: s.name.clone(),
        kind: MemberKind::Structure,
        signature: format!("structure {}", s.name),
        return_type: None,
        params: vec![],
        doc: format!("{} (fields: {})", extract_doc_before(source, s.span.start_line, s.span.start_col), fields.join(", ")),
    }
}

fn extract_interface_member(i: &InterfaceDecl, source: &str) -> TypeMember {
    let sigs: Vec<String> = i.signatures.iter().map(|s| s.name.clone()).collect();
    TypeMember {
        name: i.name.clone(),
        kind: MemberKind::Interface,
        signature: format!("interface {}", i.name),
        return_type: None,
        params: vec![],
        doc: format!("{} (methods: {})", extract_doc_before(source, i.span.start_line, i.span.start_col), sigs.join(", ")),
    }
}

fn extract_container_member(name: &str, kind: MemberKind, body: &[Statement], source: &str) -> TypeMember {
    let members: Vec<String> = body.iter().filter_map(|s| match s {
        Statement::FunctionDecl(f) => Some(f.name.clone()),
        Statement::VarDecl(v) | Statement::ConstDecl(v) => Some(v.name.clone()),
        _ => None,
    }).collect();
    let kind_str = kind_name(&kind);
    TypeMember {
        name: name.to_string(),
        kind: kind.clone(),
        signature: format!("{} {}", kind_str, name),
        return_type: None,
        params: vec![],
        doc: format!("{} members: {}", members.len(), members.join(", ")),
    }
}

fn kind_name(k: &MemberKind) -> &str {
    match k { MemberKind::Module => "module", MemberKind::Namespace => "namespace", _ => "container" }
}

fn statement_span(stmt: &Statement) -> Span {
    use cls_core::error::Span;
    match stmt {
        Statement::VarDecl(s) | Statement::ConstDecl(s) => s.span,
        Statement::FunctionDecl(s) => s.span,
        Statement::If(s) => s.span,
        Statement::While(s) => s.span,
        Statement::Loop(s) => s.span,
        Statement::For(s) => s.span,
        Statement::ForEach(s) => s.span,
        Statement::Switch(s) => s.span,
        Statement::Try(s) => s.span,
        Statement::With(s) => s.span,
        Statement::Return(_) => Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
        Statement::Expression(e) => match e {
            Expression::Binary(b) => b.span,
            Expression::Unary(u) => u.span,
            Expression::Call(c) => c.span,
            Expression::MemberAccess(m) => m.span,
            Expression::Index(i) => i.span,
            Expression::Array(a) => a.span,
            Expression::Record(r) => r.span,
            Expression::ArrowFunction(a) => a.span,
            Expression::Conditional(c) => c.span,
            Expression::Assignment(a) => a.span,
            Expression::Identifier(_, span) | Expression::NamespaceAccess(_, _, span) => *span,
            Expression::Parenthesized(_, span) => *span,
            Expression::StringInterpolation(s) => s.span,
            Expression::Cmx(c) => c.span,
            Expression::Await(_, span) => *span,
            Expression::Literal(_) => Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
        },
        Statement::Import(s) => s.span,
        Statement::FromImport(s) => s.span,
        Statement::Include(s) => s.span,
        Statement::Break | Statement::Continue => Span { start_line: 0, start_col: 0, end_line: 0, end_col: 0 },
        Statement::ClassDecl(s) => s.span,
        Statement::StructureDecl(s) => s.span,
        Statement::InterfaceDecl(s) => s.span,
        Statement::ModuleDecl(s) => s.span,
        Statement::NamespaceDecl(s) => s.span,
        Statement::Cmx(s) => s.span,
        Statement::Config(s) => s.span,
        Statement::Meta(s) => s.span,
    }
}

fn extract_function_member(f: &FunctionDecl, source: &str) -> TypeMember {
    let params: Vec<(String, String)> = f.params.iter()
        .map(|p| {
            let t = p.type_ann.as_ref().map(type_ann_to_string).unwrap_or_else(|| "Any".to_string());
            (p.name.clone(), t)
        })
        .collect();
    let return_type = f.return_type.as_ref().map(type_ann_to_string);
    let signature = format!("{}({})", f.name, params.iter()
        .map(|(n, t)| format!("{}: {}", n, t))
        .collect::<Vec<_>>()
        .join(", "));
    TypeMember {
        name: f.name.clone(),
        kind: MemberKind::Function,
        signature: if let Some(ref rt) = return_type {
            format!("{} -> {}", signature, rt)
        } else {
            signature
        },
        return_type,
        params,
        doc: extract_doc_before(source, f.span.start_line, f.span.start_col),
    }
}
fn extract_var_member(v: &VarDecl, source: &str) -> TypeMember {
    let type_str = v.type_ann.as_ref().map(type_ann_to_string);
    TypeMember {
        name: v.name.clone(),
        kind: MemberKind::Variable,
        signature: if let Some(ref t) = type_str {
            format!("var {}: {}", v.name, t)
        } else {
            format!("var {}", v.name)
        },
        return_type: type_str,
        params: vec![],
        doc: extract_doc_before(source, v.span.start_line, v.span.start_col),
    }
}

fn type_ann_to_string(ann: &TypeAnnotation) -> String {
    use cls_core::frontend::ast::TypeKind;
    match &ann.kind {
        TypeKind::Named(name, _) => name.clone(),
        TypeKind::Int => "int".to_string(),
        TypeKind::Float => "float".to_string(),
        TypeKind::String => "String".to_string(),
        TypeKind::Bool => "bool".to_string(),
        TypeKind::Any => "Any".to_string(),
        TypeKind::Void => "void".to_string(),
        TypeKind::Array(inner) => format!("Array<{}>", type_ann_to_string(inner)),
        TypeKind::Fun(params, ret) => format!("({}) -> {}", params.iter().map(type_ann_to_string).collect::<Vec<_>>().join(", "), type_ann_to_string(ret)),
        TypeKind::Record(k, v) => format!("Record<{}, {}>", type_ann_to_string(k), type_ann_to_string(v)),
        _ => "Any".to_string(),
    }
}

/// Extrae comentarios `# @description ...` antes de una declaración
fn extract_doc_before(source: &str, decl_line: u32, decl_col: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if decl_line == 0 { return String::new(); }
    let idx = (decl_line as usize).saturating_sub(1);
    if idx == 0 || idx > lines.len() { return String::new(); }
    let mut doc_lines = Vec::new();
    for i in (0..idx).rev() {
        let line = lines[i].trim();
        if line.starts_with("# @") || line.starts_with("#@") {
            doc_lines.insert(0, line.to_string());
        } else if line.starts_with("#") || line.is_empty() {
            if !line.is_empty() && !line.starts_with("# @") {
                break;
            }
        } else {
            break;
        }
    }
    doc_lines.join("\n")
}

fn extract_module_doc(module: &Module, source: &str) -> String {
    if let Some(first_stmt) = module.statements.first() {
        let span = statement_span(first_stmt);
        extract_doc_before(source, span.start_line, span.start_col)
    } else {
        String::new()
    }
}

/// Carga todas las definiciones de tipos disponibles:
/// 1. Builtin embebidas (siempre)
/// 2. Workspace .clsi (override por usuario, tiene prioridad)
pub fn load_all_type_definitions(workspace_root: Option<&str>) -> HashMap<String, TypeModule> {
    let mut result = HashMap::new();

    // 1. Builtin embebidos
    for (name, source) in builtin_type_definitions() {
        if let Some(tm) = parse_clsi(source, name) {
            result.insert(name.to_string(), tm);
        }
    }

    // 2. Workspace override
    if let Some(root) = workspace_root {
        let clsi_dir = Path::new(root).join("clsi");
        if clsi_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&clsi_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "clsi").unwrap_or(false) {
                        let name = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        if !name.is_empty() {
                            if let Ok(source) = std::fs::read_to_string(&path) {
                                if let Some(tm) = parse_clsi(&source, &name) {
                                    result.insert(name, tm);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

/// Busca un módulo de tipos por nombre
pub fn get_type_module<'a>(defs: &'a HashMap<String, TypeModule>, name: &str) -> Option<&'a TypeModule> {
    defs.get(name)
}
