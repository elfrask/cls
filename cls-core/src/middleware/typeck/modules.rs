//! TypeChecker - imports y módulos (prelude) (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {


    /// `import "path" as x` -> define el alias como módulo (acceso `x::f`).
    pub(crate) fn check_import(&mut self, imp: &ImportStatement) -> Type {
        // `import "math"`/`import "json"` (internals del nodo) -> namespace.
        let alias = imp.alias.as_deref().unwrap_or(&imp.path);
        // Import aliases pueden aparecer en varios módulos del prelude (p.ej.
        // `import "strings" as strings` en router y en request); redefinir el
        // mismo alias para el mismo path no es colisión.
        if let Some(existing_path) = self.import_aliases.get(alias) {
            if existing_path == &imp.path {
                return Type::Void;
            }
        }
        if self.scopes.first().is_some_and(|s| s.contains_key(alias)) {
            self.define_decl(alias, Type::Named(alias.to_string(), vec![]), &imp.span);
            return Type::Void;
        }
        self.import_aliases.insert(alias.to_string(), imp.path.clone());
        self.define_decl(alias, Type::Named(alias.to_string(), vec![]), &imp.span);
        Type::Void
    }


    /// `from "path" import a as fa, b` -> define cada nombre en el scope actual.
    /// `from "path" import *` -> todos los exports (reemplaza `include`).
    /// `from "path" import * as ns` -> namespace (`ns::sym`).
    pub(crate) fn check_from_import(&mut self, fi: &FromImportStatement) -> Type {
        for im in &fi.names {
            if let Some(t) = self.find_export_type(&fi.path, &im.name) {
                let local = im.alias.as_deref().unwrap_or(&im.name);
                // Los exports ya se definieron al procesar el módulo del prelude
                // (check_with_prelude). Aquí solo se asegura el binding local; la
                // colisión entre módulos DISTINTOS ya se detectó en el prelude.
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
        // `import *` -> todos los exports al scope actual.
        if fi.import_all {
            for name in self.module_export_names(&fi.path) {
                if self.lookup(&name).is_some() {
                    continue;
                }
                if let Some(t) = self.find_export_type(&fi.path, &name) {
                    self.define(&name, t);
                }
            }
        }
        // `* as ns` -> namespace (equivalente a `import "m" as ns`).
        if let Some(ns) = &fi.namespace {
            let ns = ns.clone();
            self.import_aliases.insert(ns.clone(), fi.path.clone());
            self.define_decl(&ns, Type::Named(ns.clone(), vec![]), &fi.span);
        }
        Type::Void
    }


    /// Nombre con el que se exporta una declaración directa (`export function foo`),
    /// o `None` si el statement no es un export directo.
    pub(crate) fn direct_export_name(&self, stmt: &Statement) -> Option<String> {
        match stmt {
            Statement::FunctionDecl(f) if f.visibility == Visibility::Export => Some(f.name.clone()),
            Statement::VarDecl(v) | Statement::ConstDecl(v)
                if v.visibility == Visibility::Export =>
            {
                Some(v.name.clone())
            }
            Statement::EnumDecl(e) if e.visibility == Visibility::Export => Some(e.name.clone()),
            Statement::ClassDecl(c) if c.visibility == Visibility::Export => Some(c.name.clone()),
            Statement::StructureDecl(s) if s.visibility == Visibility::Export => Some(s.name.clone()),
            Statement::InterfaceDecl(i) if i.visibility == Visibility::Export => Some(i.name.clone()),
            Statement::TypeAlias(t) if t.visibility == Visibility::Export => Some(t.name.clone()),
            _ => None,
        }
    }


    /// Nombre exportado de un `ExportSpec` (`as` o el nombre del ref). `None`
    /// para `ns::*` (que expande a varios nombres).
    fn spec_exported_name(spec: &ExportSpec) -> Option<String> {
        match &spec.reference {
            ExportRef::Name(n) => Some(spec.exported.clone().unwrap_or_else(|| n.clone())),
            ExportRef::NamespaceMember(_, _) => spec.exported.clone(),
            ExportRef::NamespaceAll(_) => None,
        }
    }


    /// `export { ... }` — valida y registra los nombres exportados.
    pub(crate) fn check_export_list(&mut self, el: &ExportListDecl) -> Type {
        if el.specs.is_empty() {
            self.error("`export { }` no puede estar vacío", el.span.clone());
            return Type::Void;
        }

        // Forma reexport: `export { a, b as c } from "path"` — solo nombres simples.
        if let Some(from) = &el.from {
            for spec in &el.specs {
                let local = match &spec.reference {
                    ExportRef::Name(n) => n.clone(),
                    _ => {
                        self.error(
                            "En `export { ... } from \"...\"` solo se permiten nombres simples",
                            el.span.clone(),
                        );
                        continue;
                    }
                };
                if self.find_export_type(from, &local).is_none() {
                    let available = self.module_export_names(from);
                    let hint = if available.is_empty() {
                        format!("El módulo '{}' no exporta ningún símbolo.", from)
                    } else {
                        format!("El módulo '{}' exporta: {}", from, available.join(", "))
                    };
                    self.error(
                        &format!("'{}' no se exporta en el módulo '{}'. {}", local, from, hint),
                        el.span.clone(),
                    );
                    continue;
                }
                let exported = spec.exported.clone().unwrap_or_else(|| local.clone());
                self.register_exported_name(&exported, &el.span);
            }
            return Type::Void;
        }

        for spec in &el.specs {
            match &spec.reference {
                ExportRef::Name(local) => {
                    let local = local.clone();
                    if self.lookup(&local).is_none() {
                        self.error(
                            &format!("'{}' no existe en el módulo (no se puede exportar)", local),
                            el.span.clone(),
                        );
                        continue;
                    }
                    let exported = spec.exported.clone().unwrap_or_else(|| local.clone());
                    self.register_exported_name(&exported, &el.span);
                }
                ExportRef::NamespaceMember(ns, member) => {
                    let (ns, member) = (ns.clone(), member.clone());
                    if spec.exported.is_none() {
                        self.error(
                            &format!(
                                "`{}::{}` requiere un nombre con `as` (ej. `{}::{} as ...`)",
                                ns, member, ns, member
                            ),
                            el.span.clone(),
                        );
                    }
                    if self.resolve_ns_path_by_alias(&ns).is_none() {
                        self.error(
                            &format!(
                                "Namespace '{}' no está definido (usa `import \"...\" as {}`)",
                                ns, ns
                            ),
                            el.span.clone(),
                        );
                        continue;
                    }
                    if self.module_member_type(&ns, &member).is_none() {
                        self.error(
                            &format!("'{}::{}' no existe", ns, member),
                            el.span.clone(),
                        );
                    }
                    if let Some(exported) = &spec.exported {
                        self.register_exported_name(exported, &el.span);
                    }
                }
                ExportRef::NamespaceAll(ns) => {
                    let ns = ns.clone();
                    let Some(path) = self.resolve_ns_path_by_alias(&ns) else {
                        self.error(
                            &format!(
                                "Namespace '{}' no está definido (usa `import \"...\" as {}`)",
                                ns, ns
                            ),
                            el.span.clone(),
                        );
                        continue;
                    };
                    for name in self.module_export_names(&path) {
                        self.register_exported_name(&name, &el.span);
                    }
                }
            }
        }
        Type::Void
    }


    /// Inserta un nombre exportado; error si ya estaba exportado en el módulo.
    fn register_exported_name(&mut self, name: &str, span: &Span) {
        if !self.exported_names.insert(name.to_string()) {
            self.error(
                &format!(
                    "El nombre '{}' ya está exportado (no se puede exportar dos veces)",
                    name
                ),
                span.clone(),
            );
        }
    }


    /// Nombres de los símbolos exportados de un módulo del prelude.
    pub(crate) fn module_export_names(&self, path: &str) -> Vec<String> {
        let m = match self.find_prelude_module(path) {
            Some(m) => m,
            None => return Vec::new(),
        };
        let mut names = Vec::new();
        for stmt in self.effective_stmt_refs(m) {
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
                Statement::ExportList(el) => {
                    for spec in &el.specs {
                        match &spec.reference {
                            ExportRef::NamespaceAll(ns) => {
                                if let Some(sub_path) = self.resolve_ns_path_in(m, ns) {
                                    names.extend(self.module_export_names(&sub_path));
                                }
                            }
                            _ => {
                                if let Some(n) = Self::spec_exported_name(spec) {
                                    names.push(n);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        names
    }


    /// Tipo de un export por nombre en el módulo del prelude (path).
    /// Considera tanto `export <decl>` como `export { ... }`.
    pub(crate) fn find_export_type(&mut self, path: &str, name: &str) -> Option<Type> {
        let m = self.find_prelude_module(path)?.clone();
        self.find_export_type_in(&m, name)
    }


    /// Igual que `find_export_type` pero sobre un módulo ya resuelto.
    fn find_export_type_in(&mut self, m: &Module, name: &str) -> Option<Type> {
        // 1. Export directo (`export function foo`).
        if let Some(t) = self.direct_export_type(m, name) {
            return Some(t);
        }
        // 2. `export { ... }`.
        for stmt in &m.statements {
            if let Statement::ExportList(el) = stmt {
                for spec in &el.specs {
                    if let ExportRef::NamespaceAll(ns) = &spec.reference {
                        if let Some(sub_path) = self.resolve_ns_path_in(m, ns) {
                            if let Some(t) = self.find_export_type(&sub_path, name) {
                                return Some(t);
                            }
                        }
                        continue;
                    }
                    if Self::spec_exported_name(spec).as_deref() == Some(name) {
                        return self.resolve_spec_type(m, &el.from, spec);
                    }
                }
            }
        }
        None
    }


    /// Tipo de una declaración con `export` directo y el nombre dado.
    fn direct_export_type(&mut self, m: &Module, name: &str) -> Option<Type> {
        for stmt in self.effective_stmt_refs(m) {
            match stmt {
                Statement::FunctionDecl(f)
                    if f.visibility == Visibility::Export && f.name == name =>
                {
                    return Some(self.function_decl_type(f));
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v)
                    if v.visibility == Visibility::Export && v.name == name =>
                {
                    return Some(self.var_decl_type(v));
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


    /// Tipo de una declaración local (cualquier visibilidad: `export { }` la promueve).
    fn local_decl_type(&mut self, m: &Module, name: &str) -> Option<Type> {
        for stmt in self.effective_stmt_refs(m) {
            match stmt {
                Statement::FunctionDecl(f) if f.name == name => {
                    return Some(self.function_decl_type(f));
                }
                Statement::VarDecl(v) | Statement::ConstDecl(v) if v.name == name => {
                    return Some(self.var_decl_type(v));
                }
                Statement::EnumDecl(e) if e.name == name => {
                    return Some(Type::Named(e.name.clone(), vec![]));
                }
                Statement::ClassDecl(c) if c.name == name => {
                    return Some(Type::Named(c.name.clone(), vec![]));
                }
                Statement::StructureDecl(s) if s.name == name => {
                    return Some(Type::Named(s.name.clone(), vec![]));
                }
                Statement::InterfaceDecl(i) if i.name == name => {
                    return Some(Type::Named(i.name.clone(), vec![]));
                }
                Statement::TypeAlias(t) if t.name == name => {
                    return Some(self.resolve_type_annotation(&t.type_ann));
                }
                _ => {}
            }
        }
        None
    }


    fn var_decl_type(&mut self, v: &VarDecl) -> Type {
        v.type_ann.as_ref()
            .map(|ta| self.resolve_type_annotation(ta))
            .or_else(|| v.value.as_ref().map(|val| self.infer_literal_type(val)))
            .unwrap_or(Type::Any)
    }


    /// Statements "efectivos" del módulo para el sistema de módulos: los de
    /// nivel superior + los de las ramas de `when` que matchean el target
    /// (donde vive el FFI cross-OS ya desugarizado a declaraciones nativas).
    fn effective_stmt_refs<'a>(&self, m: &'a Module) -> Vec<&'a Statement> {
        fn walk<'a>(target: &Target, stmts: &'a [Statement], out: &mut Vec<&'a Statement>) {
            for s in stmts {
                match s {
                    Statement::When(w) => {
                        for b in &w.branches {
                            if target.matches(&b.cond) {
                                walk(target, &b.block.statements, out);
                            }
                        }
                    }
                    _ => out.push(s),
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.target, &m.statements, &mut out);
        out
    }


    /// Tipo de un ref simple dentro del módulo: declaración local o binding de
    /// `from-import`.
    fn resolve_local_export_type(&mut self, m: &Module, name: &str) -> Option<Type> {
        if let Some(t) = self.local_decl_type(m, name) {
            return Some(t);
        }
        for stmt in &m.statements {
            if let Statement::FromImport(fi) = stmt {
                for im in &fi.names {
                    let local = im.alias.as_deref().unwrap_or(&im.name);
                    if local == name {
                        return self.find_export_type(&fi.path, &im.name);
                    }
                }
                // `from "m" import *`: el símbolo puede venir de cualquier export.
                if fi.import_all {
                    if let Some(t) = self.find_export_type(&fi.path, name) {
                        return Some(t);
                    }
                }
            }
        }
        None
    }


    /// Resuelve el tipo de un `ExportSpec` dentro del módulo `m`.
    fn resolve_spec_type(
        &mut self,
        m: &Module,
        from: &Option<String>,
        spec: &ExportSpec,
    ) -> Option<Type> {
        if let Some(from) = from {
            if let ExportRef::Name(local) = &spec.reference {
                return self.find_export_type(from, local);
            }
            return None;
        }
        match &spec.reference {
            ExportRef::Name(local) => self.resolve_local_export_type(m, local),
            ExportRef::NamespaceMember(ns, member) => {
                let path = self.resolve_ns_path_in(m, ns)?;
                self.find_export_type(&path, member)
            }
            ExportRef::NamespaceAll(_) => None,
        }
    }


    /// Path del módulo importado con el alias `ns` dentro de `m`.
    fn resolve_ns_path_in(&self, m: &Module, ns: &str) -> Option<String> {
        for stmt in &m.statements {
            if let Statement::Import(imp) = stmt {
                let alias = imp.alias.as_deref().unwrap_or(&imp.path);
                if alias == ns {
                    return Some(imp.path.clone());
                }
            }
        }
        self.import_aliases.get(ns).cloned()
    }


    /// Path del módulo importado con el alias `ns` (mapa global de aliases).
    fn resolve_ns_path_by_alias(&self, ns: &str) -> Option<String> {
        self.import_aliases.get(ns).cloned()
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
