use crate::value::Value;
use std::collections::HashMap;

/// Entorno de ejecución: scopes anidados con variables
#[derive(Debug, Clone)]
pub struct Environment {
    scopes: Vec<Scope>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, value);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    pub fn set(&mut self, name: &str, value: Value) -> bool {
        // Buscar en scopes anidados
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains(name) {
                scope.set(name, value);
                return true;
            }
        }
        // Si no existe, crear en scope actual
        self.define(name, value);
        true
    }

    pub fn contains(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|s| s.contains(name))
    }

    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Devuelve todas las variables del scope global (útil para exports)
    pub fn all(&self) -> std::collections::HashMap<String, Value> {
        self.scopes.first().map(|s| s.all()).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
struct Scope {
    variables: HashMap<String, Value>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn define(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    fn set(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    fn all(&self) -> std::collections::HashMap<String, Value> {
        self.variables.clone()
    }
}
