//! Fusión de imports aplanados (`from "m" import ...` / `include "m"` / `import "m" as x`)
//! en el módulo principal, para que el backend los compile en el mismo WASM.
//!
//! Incluye los exports de TODOS los módulos del grafo (transitivo): las clases
//! importadas usan símbolos de sus propios imports (p.ej. `App` usa `Router()`,
//! que `mod` importó de `framework/router`), así que cada módulo del grafo
//! aporta sus exports al merged. El dedup por hash de source evita traer el
//! mismo módulo dos veces si llegó por rutas distintas (módulos duplicados en
//! el sistema de archivos con el mismo contenido).
//!
//! Los exports provienen de DOS fuentes (aditivas):
//! - `export <decl>` (prefijo clásico) — nombre exportado == nombre declarado.
//! - `export { a, b as c, ns::member as X, ns::* } [from "path"]` — puede
//!   renombrar, reexportar bindings importados y aplanar namespaces.

use cls_core::frontend::ast::{
    ExportRef, ExportSpec, Module as ClsModule, Statement, Visibility,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Fusiona los imports aplanados en el módulo principal: reemplaza los
/// statements de import por las declaraciones `export` del módulo importado
/// (con el alias aplicado para `from import`), y cierra la transitividad
/// aplanando los imports internos de cada módulo del grafo (los símbolos que
/// las clases/funciones importadas usan deben existir en el merged).
pub fn flatten_imports(module: &ClsModule, imports: &[(String, ClsModule)]) -> ClsModule {
    let mut statements = Vec::new();
    // Imports del módulo principal.
    for stmt in &module.statements {
        match stmt {
            Statement::FromImport(fi) => {
                if let Some(m) = imports.iter().find(|(p, _)| *p == fi.path) {
                    for im in &fi.names {
                        let local = im.alias.clone().unwrap_or_else(|| im.name.clone());
                        push_export(&mut statements, &m.1, &im.name, &local, imports);
                    }
                    // `from "m" import *` -> todos los exports al scope actual
                    // (reemplaza al antiguo `include`).
                    if fi.import_all {
                        push_all_exports(&mut statements, &m.1, imports);
                    }
                    // `from "m" import * as ns` -> namespace `ns::sym`.
                    if let Some(ns) = &fi.namespace {
                        push_prefixed_exports(&mut statements, &m.1, ns, imports);
                    }
                    // Una funcion/tipo importada puede referenciar los TIPOS
                    // exportados del modulo (p. ej. una `structure` como tipo de
                    // retorno): se transportan para que el cuerpo compilado los
                    // resuelva.
                    push_exported_types(&mut statements, &m.1, imports);
                }
            }
            Statement::Import(imp) => {
                // `import "m" as x` -> exports bajo el prefijo `x::` (namespaced).
                if let Some(m) = imports.iter().find(|(p, _)| *p == imp.path) {
                    let prefix = imp.alias.clone().unwrap_or_else(|| imp.path.clone());
                    push_prefixed_exports(&mut statements, &m.1, &prefix, imports);
                }
            }
            other => statements.push(other.clone()),
        }
    }
    // Cerrar transitividad: cada módulo del grafo aporta sus exports (los
    // símbolos que las clases/funciones importadas usan). Dedup por hash del
    // módulo (las declaraciones): el mismo módulo no se aplanará dos veces
    // aunque llegue por rutas distintas (módulos duplicados en FS con el
    // mismo contenido).
    let mut seen: HashSet<u64> = HashSet::new();
    for (_path, m) in imports {
        flatten_nested_imports(&mut statements, m, imports, &mut seen);
    }
    ClsModule {
        statements: dedupe_statements(statements),
        span: module.span.clone(),
    }
}

/// Procesa los imports internos de un módulo importado (y recursivamente los de
/// sus propios imports), trayendo sus exports al módulo merged. `seen` evita
/// reprocesar módulos con el mismo contenido (hash de las declaraciones).
fn flatten_nested_imports(
    out: &mut Vec<Statement>,
    m: &ClsModule,
    imports: &[(String, ClsModule)],
    seen: &mut HashSet<u64>,
) {
    let mut h = DefaultHasher::new();
    format!("{:?}", m.statements).hash(&mut h);
    if !seen.insert(h.finish()) {
        return;
    }
    // Renombres de declaraciones nativas definidos por el `export { ... }` del
    // propio módulo (`export { strlen as len }`): al transportar los nativos se
    // aplica el nombre exportado, para que el consumidor use `len`.
    let renames = native_renames(m);
    for stmt in &m.statements {
        match stmt {
            Statement::FromImport(fi) => {
                if let Some(sub) = imports.iter().find(|(p, _)| *p == fi.path) {
                    for im in &fi.names {
                        let local = im.alias.clone().unwrap_or_else(|| im.name.clone());
                        push_export(out, &sub.1, &im.name, &local, imports);
                    }
                    if fi.import_all {
                        push_all_exports(out, &sub.1, imports);
                    }
                    if let Some(ns) = &fi.namespace {
                        push_prefixed_exports(out, &sub.1, ns, imports);
                    }
                    // Tipos exportados del modulo (deps de la firma/cuerpo).
                    push_exported_types(out, &sub.1, imports);
                    flatten_nested_imports(out, &sub.1, imports, seen);
                }
            }
            Statement::Import(imp) => {
                if let Some(sub) = imports.iter().find(|(p, _)| *p == imp.path) {
                    let prefix = imp.alias.clone().unwrap_or_else(|| imp.path.clone());
                    push_prefixed_exports(out, &sub.1, &prefix, imports);
                    flatten_nested_imports(out, &sub.1, imports, seen);
                }
            }
            // FFI (ya desugarizado a declaraciones nativas): las declaraciones
            // nativas del módulo importado (y su `when` cross-OS) deben llegar
            // al módulo fusionado para que el emisor registre los imports
            // nativos. Es el equivalente de exponer una `function` nativa.
            Statement::FunctionDecl(f) if f.native.is_some() => {
                let mut s = stmt.clone();
                if let Some(new) = renames.get(&f.name) {
                    rename_stmt(&mut s, new);
                }
                out.push(s);
            }
            Statement::StructureDecl(sd) if sd.native.is_some() => {
                let mut s = stmt.clone();
                if let Some(new) = renames.get(&sd.name) {
                    rename_stmt(&mut s, new);
                }
                out.push(s);
            }
            Statement::VarDecl(v) if v.native.is_some() => {
                let mut s = stmt.clone();
                if let Some(new) = renames.get(&v.name) {
                    rename_stmt(&mut s, new);
                }
                out.push(s);
            }
            Statement::When(w) => {
                if w.branches.iter().any(|b| block_has_native(&b.block)) {
                    let mut s = stmt.clone();
                    apply_native_renames(&mut s, &renames);
                    out.push(s);
                }
            }
            _ => {}
        }
    }
}

/// Renombres `local -> exportado` de declaraciones nativas declarados por el
/// `export { local as exportado }` del módulo.
fn native_renames(m: &ClsModule) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for stmt in &m.statements {
        if let Statement::ExportList(el) = stmt {
            if el.from.is_some() {
                continue;
            }
            for spec in &el.specs {
                if let (ExportRef::Name(local), Some(exported)) = (&spec.reference, &spec.exported) {
                    map.insert(local.clone(), exported.clone());
                }
            }
        }
    }
    map
}

/// Aplica los renombres a las declaraciones nativas dentro de un `when`.
fn apply_native_renames(stmt: &mut Statement, renames: &HashMap<String, String>) {
    if let Statement::When(w) = stmt {
        for b in &mut w.branches {
            for s in &mut b.block.statements {
                match s {
                    Statement::FunctionDecl(f) if f.native.is_some() => {
                        if let Some(new) = renames.get(&f.name) {
                            f.name = new.clone();
                        }
                    }
                    Statement::StructureDecl(sd) if sd.native.is_some() => {
                        if let Some(new) = renames.get(&sd.name) {
                            sd.name = new.clone();
                        }
                    }
                    Statement::VarDecl(v) if v.native.is_some() => {
                        if let Some(new) = renames.get(&v.name) {
                            v.name = new.clone();
                        }
                    }
                    Statement::When(_) => apply_native_renames(s, renames),
                    _ => {}
                }
            }
        }
    }
}

/// ¿El bloque (o un `when` anidado) declara algo nativo (FFI)?
fn block_has_native(b: &cls_core::frontend::ast::Block) -> bool {
    b.statements.iter().any(|s| match s {
        Statement::FunctionDecl(f) => f.native.is_some(),
        Statement::StructureDecl(sd) => sd.native.is_some(),
        Statement::VarDecl(v) => v.native.is_some(),
        Statement::When(w) => w.branches.iter().any(|br| block_has_native(&br.block)),
        _ => false,
    })
}

/// Elimina declaraciones duplicadas por nombre (función/var/const/enum/clase/
/// struct/interface/alias). Al flattenear imports anidados, un mismo módulo
/// podría llegar por dos rutas.
fn dedupe_statements(stmts: Vec<Statement>) -> Vec<Statement> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for stmt in stmts {
        let key = match &stmt {
            Statement::FunctionDecl(f) => Some(format!("fn:{}", f.name)),
            Statement::VarDecl(v) => Some(format!("var:{}", v.name)),
            Statement::ConstDecl(v) => Some(format!("const:{}", v.name)),
            Statement::EnumDecl(e) => Some(format!("enum:{}", e.name)),
            Statement::ClassDecl(c) => Some(format!("cls:{}", c.name)),
            Statement::StructureDecl(s) => Some(format!("struct:{}", s.name)),
            Statement::InterfaceDecl(i) => Some(format!("iface:{}", i.name)),
            Statement::TypeAlias(t) => Some(format!("alias:{}", t.name)),
            // FFI: dedup de bloques `when` (cross-OS) idénticos cargados por
            // rutas distintas del grafo.
            Statement::When(_) => {
                let mut h = DefaultHasher::new();
                format!("{:?}", stmt).hash(&mut h);
                Some(format!("when:{:x}", h.finish()))
            }
            _ => None,
        };
        match key {
            Some(k) => {
                if seen.insert(k) {
                    out.push(stmt);
                }
            }
            None => out.push(stmt),
        }
    }
    out
}

// ── Resolución de exports (directo + `export { ... }`) ──────────────────────

/// Nombre exportado de un `ExportSpec` (`as` o el nombre del ref). `None` para
/// `ns::*` (que expande a varios nombres).
fn spec_exported_name(spec: &ExportSpec) -> Option<String> {
    match &spec.reference {
        ExportRef::Name(n) => Some(spec.exported.clone().unwrap_or_else(|| n.clone())),
        ExportRef::NamespaceMember(_, _) => spec.exported.clone(),
        ExportRef::NamespaceAll(_) => None,
    }
}

/// Renombra la declaración a `name`.
fn rename_stmt(stmt: &mut Statement, name: &str) {
    match stmt {
        Statement::FunctionDecl(f) => f.name = name.to_string(),
        Statement::VarDecl(v) | Statement::ConstDecl(v) => v.name = name.to_string(),
        Statement::EnumDecl(e) => e.name = name.to_string(),
        Statement::ClassDecl(c) => c.name = name.to_string(),
        Statement::StructureDecl(s) => s.name = name.to_string(),
        Statement::InterfaceDecl(i) => i.name = name.to_string(),
        Statement::TypeAlias(t) => t.name = name.to_string(),
        _ => {}
    }
}

/// Declaración local de `m` con ese nombre (cualquier visibilidad).
/// Las declaraciones nativas se excluyen: se transportan aparte (con su
/// `when`/renombres), no se materializan por `push_*`.
fn local_decl(m: &ClsModule, name: &str) -> Option<Statement> {
    for stmt in &m.statements {
        let hit = match stmt {
            Statement::FunctionDecl(f) => f.name == name && f.native.is_none(),
            Statement::VarDecl(v) | Statement::ConstDecl(v) => v.name == name && v.native.is_none(),
            Statement::EnumDecl(e) => e.name == name,
            Statement::ClassDecl(c) => c.name == name,
            Statement::StructureDecl(s) => s.name == name && s.native.is_none(),
            Statement::InterfaceDecl(i) => i.name == name,
            Statement::TypeAlias(t) => t.name == name,
            _ => false,
        };
        if hit {
            return Some(stmt.clone());
        }
    }
    None
}

/// Declaración con `export` directo y el nombre dado. Busca también dentro de
/// ramas `when` (tipos/funciones nativas exportadas cross-OS).
fn direct_export_decl(m: &ClsModule, name: &str) -> Option<Statement> {
    fn scan(stmts: &[Statement], name: &str) -> Option<Statement> {
        for stmt in stmts {
            match stmt {
                Statement::When(w) => {
                    for b in &w.branches {
                        if let Some(s) = scan(&b.block.statements, name) {
                            return Some(s);
                        }
                    }
                }
                Statement::FunctionDecl(f)
                    if f.visibility == Visibility::Export && f.name == name && f.native.is_none() =>
                {
                    return Some(stmt.clone())
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v)
                    if v.visibility == Visibility::Export && v.name == name && v.native.is_none() =>
                {
                    return Some(stmt.clone())
                }
                Statement::EnumDecl(e) if e.visibility == Visibility::Export && e.name == name => {
                    return Some(stmt.clone())
                }
                Statement::ClassDecl(c) if c.visibility == Visibility::Export && c.name == name => {
                    return Some(stmt.clone())
                }
                Statement::StructureDecl(s)
                    if s.visibility == Visibility::Export && s.name == name && s.native.is_none() =>
                {
                    return Some(stmt.clone())
                }
                Statement::InterfaceDecl(i) if i.visibility == Visibility::Export && i.name == name => {
                    return Some(stmt.clone())
                }
                Statement::TypeAlias(t) if t.visibility == Visibility::Export && t.name == name => {
                    return Some(stmt.clone())
                }
                _ => {}
            }
        }
        None
    }
    scan(&m.statements, name)
}

/// Path del módulo importado con el alias `ns` dentro de `m`.
fn resolve_ns_path_in(m: &ClsModule, ns: &str) -> Option<String> {
    for stmt in &m.statements {
        if let Statement::Import(imp) = stmt {
            let alias = imp.alias.as_deref().unwrap_or(&imp.path);
            if alias == ns {
                return Some(imp.path.clone());
            }
        }
    }
    None
}

/// Declaración fuente del export `exported` del módulo `m` (sin renombrar).
fn resolve_export_decl(
    m: &ClsModule,
    exported: &str,
    imports: &[(String, ClsModule)],
) -> Option<Statement> {
    if let Some(s) = direct_export_decl(m, exported) {
        return Some(s);
    }
    for stmt in &m.statements {
        if let Statement::ExportList(el) = stmt {
            for spec in &el.specs {
                if let ExportRef::NamespaceAll(ns) = &spec.reference {
                    if let Some(path) = resolve_ns_path_in(m, ns) {
                        if let Some(sub) = imports.iter().find(|(p, _)| *p == path) {
                            if let Some(s) = resolve_export_decl(&sub.1, exported, imports) {
                                return Some(s);
                            }
                        }
                    }
                    continue;
                }
                if spec_exported_name(spec).as_deref() == Some(exported) {
                    return resolve_spec_decl(m, &el.from, spec, imports);
                }
            }
        }
    }
    None
}

/// Declaración fuente de un `ExportSpec`.
fn resolve_spec_decl(
    m: &ClsModule,
    from: &Option<String>,
    spec: &ExportSpec,
    imports: &[(String, ClsModule)],
) -> Option<Statement> {
    if let Some(from) = from {
        if let ExportRef::Name(local) = &spec.reference {
            let sub = imports.iter().find(|(p, _)| p == from)?;
            return resolve_export_decl(&sub.1, local, imports);
        }
        return None;
    }
    match &spec.reference {
        ExportRef::Name(local) => resolve_local_decl(m, local, imports),
        ExportRef::NamespaceMember(ns, member) => {
            let path = resolve_ns_path_in(m, ns)?;
            let sub = imports.iter().find(|(p, _)| *p == path)?;
            resolve_export_decl(&sub.1, member, imports)
        }
        ExportRef::NamespaceAll(_) => None,
    }
}

/// Declaración fuente de un ref simple: local o binding de `from-import`.
fn resolve_local_decl(
    m: &ClsModule,
    name: &str,
    imports: &[(String, ClsModule)],
) -> Option<Statement> {
    if let Some(s) = local_decl(m, name) {
        return Some(s);
    }
    for stmt in &m.statements {
        if let Statement::FromImport(fi) = stmt {
            for im in &fi.names {
                let local = im.alias.as_deref().unwrap_or(&im.name);
                if local == name {
                    let sub = imports.iter().find(|(p, _)| *p == fi.path)?;
                    return resolve_export_decl(&sub.1, &im.name, imports);
                }
            }
        }
    }
    None
}

/// Todos los nombres exportados de `m` (directos + `export { ... }`, con `ns::*`
/// expandido a los exports del submódulo).
fn all_exported_names(m: &ClsModule, imports: &[(String, ClsModule)]) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in &m.statements {
        match stmt {
            Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                names.push(f.name.clone())
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v)
                if v.visibility == Visibility::Export =>
            {
                names.push(v.name.clone())
            }
            Statement::EnumDecl(e) if e.visibility == Visibility::Export => names.push(e.name.clone()),
            Statement::ClassDecl(c) if c.visibility == Visibility::Export => names.push(c.name.clone()),
            Statement::StructureDecl(s) if s.visibility == Visibility::Export => {
                names.push(s.name.clone())
            }
            Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => {
                names.push(i.name.clone())
            }
            Statement::TypeAlias(t) if t.visibility == Visibility::Export => names.push(t.name.clone()),
            Statement::ExportList(el) => {
                for spec in &el.specs {
                    if let ExportRef::NamespaceAll(ns) = &spec.reference {
                        if let Some(path) = resolve_ns_path_in(m, ns) {
                            if let Some(sub) = imports.iter().find(|(p, _)| *p == path) {
                                names.extend(all_exported_names(&sub.1, imports));
                            }
                        }
                    } else if let Some(n) = spec_exported_name(spec) {
                        names.push(n);
                    }
                }
            }
            // Declaraciones exportadas dentro de `when` (cross-OS): también son
            // parte de la API del módulo.
            Statement::When(w) => {
                for b in &w.branches {
                    collect_when_export_names(&b.block.statements, &mut names);
                }
            }
            _ => {}
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Nombres exportados simples dentro de un bloque `when` (recursivo; sin
/// `export { ... }`, que no se usa dentro de `when`).
fn collect_when_export_names(stmts: &[Statement], names: &mut Vec<String>) {
    for stmt in stmts {
        match stmt {
            Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                names.push(f.name.clone())
            }
            Statement::VarDecl(v) | Statement::ConstDecl(v)
                if v.visibility == Visibility::Export =>
            {
                names.push(v.name.clone())
            }
            Statement::EnumDecl(e) if e.visibility == Visibility::Export => names.push(e.name.clone()),
            Statement::ClassDecl(c) if c.visibility == Visibility::Export => names.push(c.name.clone()),
            Statement::StructureDecl(s) if s.visibility == Visibility::Export => {
                names.push(s.name.clone())
            }
            Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => {
                names.push(i.name.clone())
            }
            Statement::TypeAlias(t) if t.visibility == Visibility::Export => {
                names.push(t.name.clone())
            }
            Statement::When(w) => {
                for b in &w.branches {
                    collect_when_export_names(&b.block.statements, names);
                }
            }
            _ => {}
        }
    }
}

/// Inserta el export `export_name` del módulo `m` renombrado a `local`.
fn push_export(
    out: &mut Vec<Statement>,
    m: &ClsModule,
    export_name: &str,
    local: &str,
    imports: &[(String, ClsModule)],
) {
    if let Some(mut s) = resolve_export_decl(m, export_name, imports) {
        rename_stmt(&mut s, local);
        out.push(s);
    }
}

/// Inserta los exports del módulo `m` renombrados con prefijo `{prefix}::`.
fn push_prefixed_exports(
    out: &mut Vec<Statement>,
    m: &ClsModule,
    prefix: &str,
    imports: &[(String, ClsModule)],
) {
    for name in all_exported_names(m, imports) {
        if let Some(mut s) = resolve_export_decl(m, &name, imports) {
            rename_stmt(&mut s, &format!("{}::{}", prefix, name));
            out.push(s);
        }
    }
}

/// Inserta todos los exports del módulo `m` (sin renombrar).
fn push_all_exports(out: &mut Vec<Statement>, m: &ClsModule, imports: &[(String, ClsModule)]) {
    for name in all_exported_names(m, imports) {
        if let Some(s) = resolve_export_decl(m, &name, imports) {
            out.push(s);
        }
    }
}

/// ¿Es una declaración de tipo (no función/var)?
fn is_type_decl(s: &Statement) -> bool {
    matches!(
        s,
        Statement::StructureDecl(_)
            | Statement::ClassDecl(_)
            | Statement::EnumDecl(_)
            | Statement::InterfaceDecl(_)
            | Statement::TypeAlias(_)
    )
}

/// Nombre de la declaración (función/var/const/enum/clase/struct/interface/alias).
fn decl_name(s: &Statement) -> Option<&str> {
    match s {
        Statement::FunctionDecl(f) => Some(&f.name),
        Statement::VarDecl(v) | Statement::ConstDecl(v) => Some(&v.name),
        Statement::EnumDecl(e) => Some(&e.name),
        Statement::ClassDecl(c) => Some(&c.name),
        Statement::StructureDecl(s) => Some(&s.name),
        Statement::InterfaceDecl(i) => Some(&i.name),
        Statement::TypeAlias(t) => Some(&t.name),
        _ => None,
    }
}

/// Transporta los TIPOS exportados de `m` (structures/classes/enums/interfaces/
/// aliases). Necesario para `from "m" import f` cuando `f` referencia un tipo
/// del módulo (p. ej. lo retorna): sin esto, el tipo queda fuera del módulo
/// fusionado y el emisor falla al resolverlo. No pisa declaraciones ya presentes.
fn push_exported_types(out: &mut Vec<Statement>, m: &ClsModule, imports: &[(String, ClsModule)]) {
    for name in all_exported_names(m, imports) {
        if out.iter().any(|e| decl_name(e) == Some(name.as_str())) {
            continue;
        }
        if let Some(s) = resolve_export_decl(m, &name, imports) {
            if is_type_decl(&s) {
                out.push(s);
            }
        }
    }
}
