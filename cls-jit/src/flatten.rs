//! Fusión de imports aplanados (`from "m" import ...` / `include "m"` / `import "m" as x`)
//! en el módulo principal, para que el backend los compile en el mismo WASM.
//!
//! Incluye los exports de TODOS los módulos del grafo (transitivo): las clases
//! importadas usan símbolos de sus propios imports (p.ej. `App` usa `Router()`,
//! que `mod` importó de `framework/router`), así que cada módulo del grafo
//! aporta sus exports al merged. El dedup por hash de source evita traer el
//! mismo módulo dos veces si llegó por rutas distintas (módulos duplicados en
//! el sistema de archivos con el mismo contenido).

use cls_core::frontend::ast::{Module as ClsModule, Statement, Visibility};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
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
                        push_export(&mut statements, &m.1, &im.name, &local);
                    }
                }
            }
            Statement::Include(inc) => {
                if let Some(m) = imports.iter().find(|(p, _)| *p == inc.path) {
                    push_all_exports(&mut statements, &m.1);
                }
            }
            Statement::Import(imp) => {
                // `import "m" as x` -> exports bajo el prefijo `x::` (namespaced).
                if let Some(m) = imports.iter().find(|(p, _)| *p == imp.path) {
                    let prefix = imp.alias.clone().unwrap_or_else(|| imp.path.clone());
                    push_prefixed_exports(&mut statements, &m.1, &prefix);
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
    for stmt in &m.statements {
        match stmt {
            Statement::FromImport(fi) => {
                if let Some(sub) = imports.iter().find(|(p, _)| *p == fi.path) {
                    for im in &fi.names {
                        let local = im.alias.clone().unwrap_or_else(|| im.name.clone());
                        push_export(out, &sub.1, &im.name, &local);
                    }
                    flatten_nested_imports(out, &sub.1, imports, seen);
                }
            }
            Statement::Include(inc) => {
                if let Some(sub) = imports.iter().find(|(p, _)| *p == inc.path) {
                    push_all_exports(out, &sub.1);
                    flatten_nested_imports(out, &sub.1, imports, seen);
                }
            }
            Statement::Import(imp) => {
                if let Some(sub) = imports.iter().find(|(p, _)| *p == imp.path) {
                    let prefix = imp.alias.clone().unwrap_or_else(|| imp.path.clone());
                    push_prefixed_exports(out, &sub.1, &prefix);
                    flatten_nested_imports(out, &sub.1, imports, seen);
                }
            }
            _ => {}
        }
    }
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

/// Inserta un export del módulo `m` con nombre `export_name`, renombrado a `local`.
fn push_export(out: &mut Vec<Statement>, m: &ClsModule, export_name: &str, local: &str) {
    for stmt in &m.statements {
        match stmt {
            Statement::FunctionDecl(f)
                if f.visibility == Visibility::Export && f.name == export_name =>
            {
                let mut f2 = f.clone();
                f2.name = local.to_string();
                out.push(Statement::FunctionDecl(f2));
                return;
            }
            Statement::VarDecl(v)
                if v.visibility == Visibility::Export && v.name == export_name =>
            {
                let mut v2 = v.clone();
                v2.name = local.to_string();
                out.push(Statement::VarDecl(v2));
                return;
            }
            Statement::ConstDecl(v)
                if v.visibility == Visibility::Export && v.name == export_name =>
            {
                let mut v2 = v.clone();
                v2.name = local.to_string();
                out.push(Statement::ConstDecl(v2));
                return;
            }
            Statement::EnumDecl(e)
                if e.visibility == Visibility::Export && e.name == export_name =>
            {
                let mut e2 = e.clone();
                e2.name = local.to_string();
                out.push(Statement::EnumDecl(e2));
                return;
            }
            Statement::ClassDecl(c)
                if c.visibility == Visibility::Export && c.name == export_name =>
            {
                let mut c2 = c.clone();
                c2.name = local.to_string();
                out.push(Statement::ClassDecl(c2));
                return;
            }
            Statement::StructureDecl(s)
                if s.visibility == Visibility::Export && s.name == export_name =>
            {
                let mut s2 = s.clone();
                s2.name = local.to_string();
                out.push(Statement::StructureDecl(s2));
                return;
            }
            Statement::InterfaceDecl(i)
                if i.visibility == Visibility::Export && i.name == export_name =>
            {
                let mut i2 = i.clone();
                i2.name = local.to_string();
                out.push(Statement::InterfaceDecl(i2));
                return;
            }
            Statement::TypeAlias(t)
                if t.visibility == Visibility::Export && t.name == export_name =>
            {
                let mut t2 = t.clone();
                t2.name = local.to_string();
                out.push(Statement::TypeAlias(t2));
                return;
            }
            _ => {}
        }
    }
}

/// Inserta los exports del módulo `m` renombrados con prefijo `{prefix}::`.
fn push_prefixed_exports(out: &mut Vec<Statement>, m: &ClsModule, prefix: &str) {
    for stmt in &m.statements {
        match stmt {
            Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                let mut f2 = f.clone();
                f2.name = format!("{}::{}", prefix, f.name);
                out.push(Statement::FunctionDecl(f2));
            }
            Statement::VarDecl(v) if v.visibility == Visibility::Export => {
                let mut v2 = v.clone();
                v2.name = format!("{}::{}", prefix, v.name);
                out.push(Statement::VarDecl(v2));
            }
            Statement::ConstDecl(v) if v.visibility == Visibility::Export => {
                let mut v2 = v.clone();
                v2.name = format!("{}::{}", prefix, v.name);
                out.push(Statement::ConstDecl(v2));
            }
            Statement::EnumDecl(e) if e.visibility == Visibility::Export => {
                let mut e2 = e.clone();
                e2.name = format!("{}::{}", prefix, e.name);
                out.push(Statement::EnumDecl(e2));
            }
            Statement::ClassDecl(c) if c.visibility == Visibility::Export => {
                let mut c2 = c.clone();
                c2.name = format!("{}::{}", prefix, c.name);
                out.push(Statement::ClassDecl(c2));
            }
            Statement::StructureDecl(s) if s.visibility == Visibility::Export => {
                let mut s2 = s.clone();
                s2.name = format!("{}::{}", prefix, s.name);
                out.push(Statement::StructureDecl(s2));
            }
            Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => {
                let mut i2 = i.clone();
                i2.name = format!("{}::{}", prefix, i.name);
                out.push(Statement::InterfaceDecl(i2));
            }
            Statement::TypeAlias(t) if t.visibility == Visibility::Export => {
                let mut t2 = t.clone();
                t2.name = format!("{}::{}", prefix, t.name);
                out.push(Statement::TypeAlias(t2));
            }
            _ => {}
        }
    }
}

/// Inserta todos los exports del módulo `m` (sin renombrar).
fn push_all_exports(out: &mut Vec<Statement>, m: &ClsModule) {
    for stmt in &m.statements {
        match stmt {
            Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                out.push(Statement::FunctionDecl(f.clone()));
            }
            Statement::VarDecl(v) if v.visibility == Visibility::Export => {
                out.push(Statement::VarDecl(v.clone()));
            }
            Statement::ConstDecl(v) if v.visibility == Visibility::Export => {
                out.push(Statement::ConstDecl(v.clone()));
            }
            Statement::EnumDecl(e) if e.visibility == Visibility::Export => {
                out.push(Statement::EnumDecl(e.clone()));
            }
            Statement::ClassDecl(c) if c.visibility == Visibility::Export => {
                out.push(Statement::ClassDecl(c.clone()));
            }
            Statement::StructureDecl(s) if s.visibility == Visibility::Export => {
                out.push(Statement::StructureDecl(s.clone()));
            }
            Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => {
                out.push(Statement::InterfaceDecl(i.clone()));
            }
            Statement::TypeAlias(t) if t.visibility == Visibility::Export => {
                out.push(Statement::TypeAlias(t.clone()));
            }
            _ => {}
        }
    }
}
