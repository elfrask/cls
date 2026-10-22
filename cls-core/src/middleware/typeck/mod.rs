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
    name: String,
    type_params: Vec<TypeParam>,
    extends: Vec<TypeAnnotation>,
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
    /// Esquema de enums (F2): nombre -> por variante, `(nombre, tipos de params)`.
    /// `params` vacío = variante unit. Habilita la validación estática de la
    /// construcción (`X::v(args)`), del patrón (`v(binds)`) y del destructuring-let.
    enum_defs: HashMap<String, Vec<(String, Vec<Type>)>>,
    /// Provabilidad del destructuring-let (F2): variable local -> (enum, variante)
    /// cuando su último valor fue una construcción directa. Simplificación F2:
    /// sin dataflow estricto; se limpia al reasignar a otra cosa.
    var_variant: HashMap<String, (String, String)>,
    /// Mapa Span -> Type de TODAS las expresiones visitadas (para backends).
    /// Se llena solo cuando `config.check` es true.
    types_by_span: HashMap<Span, Type>,
    /// Miembros de cada clase: nombre -> tipo del campo o del retorno del método.
    class_members: HashMap<String, HashMap<String, Type>>,
    /// Parámetros de los métodos de cada clase: `Clase` -> método -> tipos de
    /// params (incluye heredados, como `class_members`). Para validar los
    /// operandos del dispatch de magic methods (M1: tipos incompatibles -> basura).
    magic_params: HashMap<String, HashMap<String, Vec<Type>>>,
    /// Cantidad de parámetros obligatorios del constructor `main` de cada
    /// clase. `magic_params` guarda el total; esto permite validar llamadas
    /// como `Persona()` cuando el constructor exige argumentos (antes el
    /// emisor generaba WASM inválido).
    ctor_required: HashMap<String, usize>,
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
    /// Nombres exportados del módulo en proceso (detecta duplicados entre
    /// `export { ... }`, exports directos y colisiones). Se reinicia por módulo.
    exported_names: std::collections::HashSet<String>,
    /// Pila de tipos de resultado de los **ternary-targets** activos (F4):
    /// la rama de un `if`-expresión o el cuerpo de un arma de `match` bloque.
    /// El tipo lo fija el valor del `* <-` que los targets reciben.
    ternary_yield_stack: Vec<Option<Type>>,
    /// Pila de tipos de resultado de los **match-targets** activos (F4): cada
    /// `match` envolvente. Lo fija el valor de `match <-`/`match* <-`/...
    match_yield_stack: Vec<Option<Type>>,
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
            enum_defs: HashMap::new(),
            var_variant: HashMap::new(),
            types_by_span: HashMap::new(),
            class_members: HashMap::new(),
            magic_params: HashMap::new(),
            ctor_required: HashMap::new(),
            class_parents: HashMap::new(),
            struct_members: HashMap::new(),
            prelude: Vec::new(),
            import_aliases: HashMap::new(),
            const_symbols: std::collections::HashSet::new(),
            pre_registered: std::collections::HashSet::new(),
            target,
            exported_names: std::collections::HashSet::new(),
            ternary_yield_stack: Vec::new(),
            match_yield_stack: Vec::new(),
        };
        // Clase core `CMX` (respaldada por el builtin `Cmx`): firmas de sus
        // métodos pre-registradas para que los MÓDULOS del prelude (que se
        // chequean ANTES que el módulo principal) puedan usarlas. Los cuerpos
        // viven en CLS (`frontend/builtins.rs`).
        {
            let mut cmx_methods: HashMap<String, Type> = HashMap::new();
            cmx_methods.insert("isPrimitive".to_string(), Type::Bool);
            cmx_methods.insert("isEmpty".to_string(), Type::Bool);
            cmx_methods.insert("isElementOf".to_string(), Type::Bool);
            cmx_methods.insert("tryCallTag".to_string(), Type::Value);
            cmx_methods.insert(
                "flattenChildren".to_string(),
                Type::Array(Box::new(Type::Cmx)),
            );
            tc.class_members.insert("CMX".to_string(), cmx_methods);
        }
        // Registrar funciones built-in (core intrinsics) como CONSTANTES (no
        // redefinibles): print, input, args, toString, int, float, str, bool,
        // len, type, now, exit, sleep, throw.
        let core_names = [
            "print", "input", "args", "toString", "int", "float", "str", "bool",
            "len", "type", "now", "exit", "sleep", "throw", "config", "char", "ord",
        ];
        tc.define("print", Type::Fun(vec![Type::Value], Box::new(Type::Void)));
        tc.define("input", Type::Fun(vec![Type::String], Box::new(Type::String)));
        tc.define("args", Type::Array(Box::new(Type::String)));
        tc.define("toString", Type::Fun(vec![Type::Value], Box::new(Type::String)));
        tc.define("int", Type::Fun(vec![Type::Value], Box::new(Type::Int)));
        tc.define("float", Type::Fun(vec![Type::Value], Box::new(Type::Float)));
        tc.define("str", Type::Fun(vec![Type::Value], Box::new(Type::String)));
        tc.define("bool", Type::Fun(vec![Type::Value], Box::new(Type::Bool)));
        tc.define("len", Type::Fun(vec![Type::Value], Box::new(Type::Int)));
        tc.define("type", Type::Fun(vec![Type::Value], Box::new(Type::String)));
        tc.define("now", Type::Fun(vec![], Box::new(Type::Int)));
        tc.define("exit", Type::Fun(vec![Type::Int], Box::new(Type::Void)));
        tc.define("sleep", Type::Fun(vec![Type::Int], Box::new(Type::Void)));
        tc.define("throw", Type::Fun(vec![Type::Value], Box::new(Type::Unknown)));
        // `char(n: int) -> Char` / `ord(c: Char) -> int`: unidad de 1 byte para
        // buffers (modelo bytes/str). Intrínsecos primitivos (i32 trivial).
        tc.define("char", Type::Fun(vec![Type::Int], Box::new(Type::Char)));
        tc.define("ord", Type::Fun(vec![Type::Char], Box::new(Type::Int)));
        // Intrinsic de introspección: `config() -> Record<String, String>`.
        tc.define(
            "config",
            Type::Fun(
                vec![],
                Box::new(Type::Record(Box::new(Type::String), Box::new(Type::String))),
            ),
        );
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
            .map(|p| p.type_ann.as_ref().map(|t| self.resolve_type_annotation(t)).unwrap_or(Type::Value))
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
        // Checkear los statements del prelude. Esto es necesario para validar
        // tipos internos de los exports (e.g. `function saludar() { return "..."; }`
        // debe validar que la concatenacion es valida). Cada módulo se chequea en
        // su PROPIO scope: así sus declaraciones y sus imports (`from ... import *`)
        // no se mezclan con los de otros módulos. Antes compartían scope[0] y un
        // módulo reutilizado por varios (`a` y `b` importan `part`) disparaba
        // "ya está declarado" (bug M).
        for (_path, m) in prelude {
            self.exported_names.clear();
            self.push_scope();
            for stmt in &m.statements {
                self.check_statement(stmt);
            }
            self.pop_scope();
        }
        // Bug fix dev-2 (Fase 7): antes la pasada tambien checkeaba los
        // statements del prelude (linea 190-194 originales). Esto era
        // problematico porque:
        // 1. Los `check_statement` del prelude llamaban `define_decl` para los
        //    exports, que consumia el `pre_registered`.
        // 2. Ademas, los exports quedaban en scope[0] del modulo principal, lo
        //    que hacia fallar el `include "libmod"` posterior con
        //    "ya esta declarado" (especialmente para vars/consts que no se
        //    pre-registran).
        // Solucion: en `define_decl`, chequear presencia (no consumir) del
        // pre-registrado. Y en `check_include` del modulo principal, saltar
        // exports que ya estan en scope[0] (porque ya fueron definidos por la
        // pasada del prelude).
        self.exported_names.clear();
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
            // Bug fix dev-2 (Fase 7): antes era `self.pre_registered.remove(name)`.
            // El problema: la pre-pasada pre-registra las funciones del prelude,
            // y luego la pasada principal (sobre los statements del prelude)
            // llama `check_function_decl` que consume el pre-registro via
            // `define_decl`. Despues, cuando el modulo principal hace
            // `include "libmod"`, `check_include` llama `define_decl("saludar", ...)`
            // y el pre-registro ya no esta. Ademas, el scope[0] ya tiene
            // "saludar" (definido por check_function_decl). Resultado: error
            // "ya esta declarado".
            //
            // Solucion: chequear presencia (no consumir). Si esta pre-registrado,
            // es legitimo redefinir (es el mismo simbolo del mismo modulo re-checkeado).
            if self.pre_registered.contains(name) {
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
