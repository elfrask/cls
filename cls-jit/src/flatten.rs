//! Fusión de imports aplanados (`from "m" import ...` / `include "m"`) en el
//! módulo principal, para que el backend los compile en el mismo WASM.

use cls_core::frontend::ast::{Module as ClsModule, Statement, Visibility};

/// Fusiona los imports aplanados (`from "m" import ...` / `include "m"`) en el
/// módulo principal: reemplaza el statement por las declaraciones `export` del
/// módulo importado (con el alias aplicado para `from import`). Los `import`
/// namespaced se dejan sin tocar (se resuelven en una fase posterior).
pub fn flatten_imports(module: &ClsModule, imports: &[(String, ClsModule)]) -> ClsModule {
    let mut statements = Vec::new();
    for stmt in &module.statements {
        match stmt {
            Statement::FromImport(fi) => {
                let m = imports.iter().find(|(p, _)| *p == fi.path).map(|(_, m)| m);
                if let Some(m) = m {
                    for im in &fi.names {
                        let local = im.alias.clone().unwrap_or_else(|| im.name.clone());
                        push_export(&mut statements, m, &im.name, &local);
                    }
                }
            }
            Statement::Include(inc) => {
                let m = imports.iter().find(|(p, _)| *p == inc.path).map(|(_, m)| m);
                if let Some(m) = m {
                    push_all_exports(&mut statements, m);
                }
            }
            Statement::Import(imp) => {
                // `import "m" as x` → exports bajo el prefijo `x::` (namespaced).
                let m = imports.iter().find(|(p, _)| *p == imp.path).map(|(_, m)| m);
                if let Some(m) = m {
                    let prefix = imp.alias.clone().unwrap_or_else(|| imp.path.clone());
                    push_prefixed_exports(&mut statements, m, &prefix);
                    // Módulo→módulo: el módulo importado puede importar otros a su
                    // vez (nested_b → nested_a). Sus referencias internas (na::base)
                    // necesitan que esos exports existan en el WASM merged.
                    flatten_nested_imports(&mut statements, m, imports);
                }
            }
            other => statements.push(other.clone()),
        }
    }
    ClsModule {
        statements: dedupe_statements(statements),
        span: module.span.clone(),
    }
}

/// Procesa los imports internos de un módulo importado (y recursivamente los de
/// sus propios imports), trayendo sus exports namespaced al módulo merged.
fn flatten_nested_imports(out: &mut Vec<Statement>, m: &ClsModule, imports: &[(String, ClsModule)]) {
    for stmt in &m.statements {
        if let Statement::Import(imp) = stmt {
            let sub = imports
                .iter()
                .find(|(p, _)| *p == imp.path)
                .map(|(_, s)| s);
            if let Some(sub) = sub {
                let prefix = imp.alias.clone().unwrap_or_else(|| imp.path.clone());
                push_prefixed_exports(out, sub, &prefix);
                flatten_nested_imports(out, sub, imports);
            }
        }
    }
}

/// Elimina declaraciones duplicadas por nombre (función/var/const/enum). Al
/// flattenear imports anidados, un mismo módulo podría llegar por dos rutas.
fn dedupe_statements(stmts: Vec<Statement>) -> Vec<Statement> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for stmt in stmts {
        let key = match &stmt {
            Statement::FunctionDecl(f) => Some(format!("fn:{}", f.name)),
            Statement::VarDecl(v) => Some(format!("var:{}", v.name)),
            Statement::ConstDecl(v) => Some(format!("const:{}", v.name)),
            Statement::EnumDecl(e) => Some(format!("enum:{}", e.name)),
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
            _ => {}
        }
    }
}
