//! Type checker configurable de CLS (Fase 1: extraido de middleware/typeck.rs).

mod binary;
mod calls;
mod classes;
mod containers;
mod decls;
mod expressions;
mod flow;
mod helpers;
mod magics;
mod member;
mod modules;
mod statements;
mod tests;
mod types;

pub(crate) use helpers::{builtin_type_name, module_arity};
pub use helpers::{expr_short_display, expr_span};

use crate::error::{ClsResult, Diagnostic, Span};
use crate::frontend::ast::*;
use crate::middleware::types::{Type, LitVal};
use crate::config::types::TypesConfig;
use std::collections::HashMap;

/// Definición compile-time de una interface (shapes con genéricos).
#[derive(Clone)]
pub(crate) struct InterfaceInfo {
    type_params: Vec<TypeParam>,
    fields: HashMap<String, TypeAnnotation>,
    /// Orden de declaración de los campos (para offsets deterministas del shape).
    field_order: Vec<String>,
    signatures: HashMap<String, SignatureDecl>,
    /// Orden de declaración de los métodos (para offsets deterministas del shape).
    signature_order: Vec<String>,
}

/// Type checker configurable de CLS
pub struct TypeChecker {
    config: TypesConfig,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, Type>>,
    current_return_type: Option<Type>,
    /// Span de la función actual (para errores de `return` sin span propio).
    current_fn_span: Span,
    interfaces: HashMap<String, InterfaceInfo>,
    enums: std::collections::HashSet<String>,
    /// Mapa Span -> Type de TODAS las expresiones visitadas (para backends).
    /// Se llena solo cuando `config.check` es true.
    types_by_span: HashMap<Span, Type>,
    /// Miembros de cada clase: nombre -> tipo del campo o del retorno del método.
    class_members: HashMap<String, HashMap<String, Type>>,
    /// Parámetros de los métodos de cada clase: `Clase` -> método -> tipos de
    /// params (incluye heredados, como `class_members`). Para validar los
    /// operandos del dispatch de magic methods (M1: tipos incompatibles -> basura).
    magic_params: HashMap<String, HashMap<String, Vec<Type>>>,
    /// Padre de cada clase (`Hijo` -> `Base`), para la asignabilidad por herencia
    /// en la validación de operandos de magics (M2).
    class_parents: HashMap<String, String>,
    /// Campos de cada structure: nombre -> tipo. Para tipar `p.campo` (member access).
    struct_members: HashMap<String, HashMap<String, Type>>,
    /// Módulos importados (prelude) - para resolver símbolos de `import`/`from`/`include`.
    /// Cada entrada: (path del import, módulo parseado).
    prelude: Vec<(String, Module)>,
    /// Alias de `import "path" as x` -> path (para `x::miembro`).
    import_aliases: HashMap<String, String>,
    /// Nombres de símbolos CONSTANTES (intrinsics core): no redefinibles por el
    /// usuario (sobrescribirlos da resultados inesperados / bugs fatales).
    const_symbols: std::collections::HashSet<String>,
    /// Nombres pre-registrados por la pre-pasada de firmas del prelude (para
    /// recursión). `define_decl` no los cuenta como colisión (son el mismo
    /// símbolo del mismo módulo, re-chequeado en la pasada principal).
    pre_registered: std::collections::HashSet<String>,
    /// Target del entorno actual. Usado por las directivas `when` en
    /// compile-time: solo la rama que matchea este target se procesa (mismo
    /// comportamiento que el emisor WASM y el resolver). Default: el host.
    target: Target,
}

impl TypeChecker {
    pub fn new(config: TypesConfig) -> Self {
        Self::with_target(config, Target::host())
    }

    /// Construye un typeck para un target especifico (usado por
    /// `clx check --target <tripla>` para simular el entorno).
    pub fn with_target(config: TypesConfig, target: Target) -> Self {
        let mut tc = Self {
            config,
            diagnostics: Vec::new(),
            scopes: vec![HashMap::new()],
            current_return_type: None,
    current_fn_span: Span::new(1, 1, 1, 1),
            interfaces: HashMap::new(),
            enums: std::collections::HashSet::new(),
            types_by_span: HashMap::new(),
            class_members: HashMap::new(),
            magic_params: HashMap::new(),
            class_parents: HashMap::new(),
            struct_members: HashMap::new(),
            prelude: Vec::new(),
            import_aliases: HashMap::new(),
            const_symbols: std::collections::HashSet::new(),
            pre_registered: std::collections::HashSet::new(),
            target,
        };
        // Registrar funciones built-in (core intrinsics) como CONSTANTES (no
        // redefinibles): print, input, args, toString, int, float, str, bool,
        // len, type, now, exit, sleep, throw.
        let core_names = [
            "print", "input", "args", "toString", "int", "float", "str", "bool",
            "len", "type", "now", "exit", "sleep", "throw",
        ];
        tc.define("print", Type::Fun(vec![Type::Any], Box::new(Type::Void)));
        tc.define("input", Type::Fun(vec![Type::String], Box::new(Type::String)));
        tc.define("args", Type::Array(Box::new(Type::String)));
        tc.define("toString", Type::Fun(vec![Type::Any], Box::new(Type::String)));
        tc.define("int", Type::Fun(vec![Type::Any], Box::new(Type::Int)));
        tc.define("float", Type::Fun(vec![Type::Any], Box::new(Type::Float)));
        tc.define("str", Type::Fun(vec![Type::Any], Box::new(Type::String)));
        tc.define("bool", Type::Fun(vec![Type::Any], Box::new(Type::Bool)));
        tc.define("len", Type::Fun(vec![Type::Any], Box::new(Type::Int)));
        tc.define("type", Type::Fun(vec![Type::Any], Box::new(Type::String)));
        tc.define("now", Type::Fun(vec![], Box::new(Type::Int)));
        tc.define("exit", Type::Fun(vec![Type::Int], Box::new(Type::Void)));
        tc.define("sleep", Type::Fun(vec![Type::Int], Box::new(Type::Void)));
        tc.define("throw", Type::Fun(vec![Type::Any], Box::new(Type::Unknown)));
        for n in &core_names {
            tc.const_symbols.insert(n.to_string());
        }
        tc
    }


    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }


    pub fn check(&mut self, module: &Module) -> ClsResult<()> {
        if !self.config.check {
            return Ok(());
        }
        self.pre_registered.clear();
        // Pre-registrar firmas de funciones top-level (uso antes de definición).
        for stmt in &module.statements {
            if let Statement::FunctionDecl(f) = stmt {
                self.pre_registered.insert(f.name.clone());
                self.define_function_signature(f);
            }
        }
        for stmt in &module.statements {
            self.check_statement(stmt);
        }
        // No fallar si hay errores; reportar como diagnóstico
        Ok(())
    }


    pub(crate) fn define_function_signature(&mut self, f: &FunctionDecl) {
        let param_tys: Vec<Type> = f.params.iter()
            .map(|p| p.type_ann.as_ref().map(|t| self.resolve_type_annotation(t)).unwrap_or(Type::Any))
            .collect();
        let ret = f.return_type.as_ref()
            .map(|t| self.resolve_type_annotation(t))
            .unwrap_or(Type::Void);
        self.define(&f.name, Type::Fun(param_tys, Box::new(ret)));
    }


    /// Chequea un módulo con un prelude de módulos importados.
    /// Los tipos (enum/class/alias/interface) del prelude se registran primero,
    /// para que el módulo principal pueda usarlos en anotaciones.
    pub fn check_with_prelude(&mut self, module: &Module, prelude: &[(String, Module)]) -> ClsResult<()> {        if !self.config.check {
            return Ok(());
        }
        self.prelude = prelude.to_vec();
        self.pre_registered.clear();
        // Pre-registrar firmas de funciones top-level de cada módulo del prelude
        // (para soportar recursión y uso antes de definición dentro del módulo).
        for (_path, m) in prelude {
            for stmt in &m.statements {
                if let Statement::FunctionDecl(f) = stmt {
                    self.pre_registered.insert(f.name.clone());
                    self.define_function_signature(f);
                }
            }
        }
        for (_path, m) in prelude {
            for stmt in &m.statements {
                self.check_statement(stmt);
            }
        }
        for stmt in &module.statements {
            self.check_statement(stmt);
        }
        Ok(())
    }


    /// Registra las firmas de las funciones host del NODO (intrinsics) en el
    /// scope global: las llamadas a esos nombres se tipan contra la firma y el
    /// emisor las compila vía el canal `env.host_call`.
    pub fn register_host_intrinsics(&mut self, intrinsics: &[crate::middleware::types::HostIntrinsic]) {
        for i in intrinsics {
            self.define(&i.name, Type::Fun(i.params.clone(), Box::new(i.ret.clone())));
        }
    }


    pub(crate) fn error(&mut self, msg: &str, span: Span) -> Type {
        self.diagnostics.push(Diagnostic::error(msg, span));
        Type::Unknown
    }


    pub(crate) fn warn(&mut self, msg: &str, span: Span) {
        self.diagnostics.push(Diagnostic::warning(msg, span));
    }


    pub(crate) fn define(&mut self, name: &str, typ: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), typ);
        }
    }

    /// Define un símbolo de DECLARACIÓN top-level (función, clase, enum, struct,
    /// interface, alias, var/const global, import). En el scope global detecta:
    /// - nombre ya declarado -> error "declaración múltiple" (colisión entre
    ///   módulos importados o redefinición en el script).
    /// - nombre de intrinsic const -> error (no redefinible).
    /// En scopes locales no hace nada especial (el shadowing es normal).
    pub(crate) fn define_decl(&mut self, name: &str, typ: Type, span: &Span) -> Type {
        if self.scopes.len() == 1 {
            let is_const = self.const_symbols.contains(name);
            if is_const && self.scopes[0].contains_key(name) {
                return self.error(
                    &format!(
                        "El nombre '{}' es un intrinsic del lenguaje y no puede redefinirse",
                        name
                    ),
                    span.clone(),
                );
            }
            // Un símbolo pre-registrado (firma de la pre-pasada del prelude) no
            // cuenta como colisión: es el mismo símbolo del mismo módulo que se
            // re-chequea en la pasada principal.
            if self.pre_registered.remove(name) {
                self.define(name, typ);
                return Type::Void;
            }
            if self.scopes[0].contains_key(name) {
                return self.error(
                    &format!(
                        "El nombre '{}' ya está declarado (declaración múltiple). Los módulos importados no pueden exportar el mismo nombre en el mismo scope.",
                        name
                    ),
                    span.clone(),
                );
            }
        }
        self.define(name, typ);
        Type::Void
    }


    pub(crate) fn lookup(&self, name: &str) -> Option<&Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }


    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }


    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }


    /// Mapa de tipos por span de todas las expresiones visitadas.
    pub fn type_map(&self) -> &HashMap<Span, Type> {
        &self.types_by_span
    }

}
