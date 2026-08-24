//! TypeChecker - imports y módulos (prelude) (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {


    /// `import "path" as x` -> define el alias como módulo (acceso `x::f`).
    pub(crate) fn check_import(&mut self, imp: &ImportStatement) -> Type {
        // `import "math"`/`import "json"` (internals del nodo) -> namespace.
        let alias = imp.alias.as_deref().unwrap_or(&imp.path);
        self.import_aliases.insert(alias.to_string(), imp.path.clone());
        self.define(alias, Type::Named(alias.to_string(), vec![]));
        Type::Void
    }


    /// `from "path" import a as fa, b` -> define cada nombre en el scope actual.
    pub(crate) fn check_from_import(&mut self, fi: &FromImportStatement) -> Type {
        for im in &fi.names {
            if let Some(t) = self.find_export_type(&fi.path, &im.name) {
                let local = im.alias.as_deref().unwrap_or(&im.name);
                self.define(local, t);
            } else {
                let available = self.module_export_names(&fi.path);
                let hint = if available.is_empty() {
                    format!(
                        "El módulo '{}' no exporta ningún símbolo (usa `export` en cada declaración).",
                        fi.path
                    )
                } else {
                    format!(
                        "El módulo '{}' exporta: {}",
                        fi.path,
                        available.join(", ")
                    )
                };
                self.error(
                    &format!(
                        "'{}' no se exporta en el módulo '{}'. {}",
                        im.name, fi.path, hint
                    ),
                    fi.span.clone(),
                );
            }
        }
        Type::Void
    }


    /// Nombres de los símbolos exportados de un módulo del prelude.
    pub(crate) fn module_export_names(&self, path: &str) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(m) = self.find_prelude_module(path) {
            for stmt in &m.statements {
                match stmt {
                    Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                        names.push(f.name.clone());
                    }
                    Statement::VarDecl(v) | Statement::ConstDecl(v)
                        if v.visibility == Visibility::Export =>
                    {
                        names.push(v.name.clone());
                    }
                    Statement::EnumDecl(e) if e.visibility == Visibility::Export => {
                        names.push(e.name.clone());
                    }
                    Statement::ClassDecl(c) if c.visibility == Visibility::Export => {
                        names.push(c.name.clone());
                    }
                    Statement::StructureDecl(s) if s.visibility == Visibility::Export => {
                        names.push(s.name.clone());
                    }
                    Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => {
                        names.push(i.name.clone());
                    }
                    Statement::TypeAlias(t) if t.visibility == Visibility::Export => {
                        names.push(t.name.clone());
                    }
                    _ => {}
                }
            }
        }
        names
    }


    /// `include "path"` -> define TODOS los exports en el scope actual.
    pub(crate) fn check_include(&mut self, inc: &IncludeStatement) -> Type {
        let m = match self.find_prelude_module(&inc.path) {
            Some(m) => m.clone(),
            None => return Type::Void,
        };
        for stmt in &m.statements {
            match stmt {
                Statement::FunctionDecl(f) if f.visibility == Visibility::Export => {
                    let t = self.function_decl_type(f);
                    self.define(&f.name, t);
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v)
                    if v.visibility == Visibility::Export =>
                {
                    let t = v.type_ann.as_ref()
                        .map(|ta| self.resolve_type_annotation(ta))
                        .or_else(|| v.value.as_ref().map(|val| self.infer_literal_type(val)))
                        .unwrap_or(Type::Any);
                    self.define(&v.name, t);
                }
                Statement::EnumDecl(e) if e.visibility == Visibility::Export => {
                    self.define(&e.name, Type::Named(e.name.clone(), vec![]));
                }
                Statement::ClassDecl(c) if c.visibility == Visibility::Export => {
                    self.define(&c.name, Type::Named(c.name.clone(), vec![]));
                }
                Statement::StructureDecl(s) if s.visibility == Visibility::Export => {
                    self.define(&s.name, Type::Named(s.name.clone(), vec![]));
                }
                Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => {
                    self.define(&i.name, Type::Named(i.name.clone(), vec![]));
                }
                Statement::TypeAlias(t) if t.visibility == Visibility::Export => {
                    let ty = self.resolve_type_annotation(&t.type_ann);
                    self.define(&t.name, ty);
                }
                _ => {}
            }
        }
        Type::Void
    }


    /// Tipo de un export por nombre en el módulo del prelude (path).
    pub(crate) fn find_export_type(&mut self, path: &str, name: &str) -> Option<Type> {
        let m = self.find_prelude_module(path)?.clone();
        for stmt in &m.statements {
            match stmt {
                Statement::FunctionDecl(f)
                    if f.visibility == Visibility::Export && f.name == name =>
                {
                    return Some(self.function_decl_type(f));
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v)
                    if v.visibility == Visibility::Export && v.name == name =>
                {
                    let t = v.type_ann.as_ref()
                        .map(|ta| self.resolve_type_annotation(ta))
                        .or_else(|| v.value.as_ref().map(|val| self.infer_literal_type(val)))
                        .unwrap_or(Type::Any);
                    return Some(t);
                }
                Statement::EnumDecl(e) if e.visibility == Visibility::Export && e.name == name => {
                    return Some(Type::Named(e.name.clone(), vec![]));
                }
                Statement::ClassDecl(c) if c.visibility == Visibility::Export && c.name == name => {
                    return Some(Type::Named(c.name.clone(), vec![]));
                }
                Statement::StructureDecl(s) if s.visibility == Visibility::Export && s.name == name => {
                    return Some(Type::Named(s.name.clone(), vec![]));
                }
                Statement::InterfaceDecl(i) if i.visibility == Visibility::Export && i.name == name => {
                    return Some(Type::Named(i.name.clone(), vec![]));
                }
                Statement::TypeAlias(t) if t.visibility == Visibility::Export && t.name == name => {
                    return Some(self.resolve_type_annotation(&t.type_ann));
                }
                _ => {}
            }
        }
        None
    }


    /// Busca un módulo del prelude cuyo path coincida con el import.
    pub(crate) fn find_prelude_module(&self, path: &str) -> Option<&Module> {
        self.prelude.iter().find(|(p, _)| p == path).map(|(_, m)| m)
    }


    /// Tipo de `x::miembro` cuando `x` es un módulo importado.
    pub(crate) fn module_member_type(&mut self, module_alias: &str, member: &str) -> Option<Type> {
        let path = self
            .import_aliases
            .get(module_alias)
            .cloned()
            .unwrap_or_else(|| module_alias.to_string());
        self.find_export_type(&path, member)
    }

}