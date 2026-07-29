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

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with_values() -> Environment {
        let mut e = Environment::new();
        e.define("x", Value::Int(10));
        e.define("y", Value::String("hello".into()));
        e
    }

    #[test]
    fn test_define_and_get() {
        let e = env_with_values();
        assert_eq!(e.get("x"), Some(&Value::Int(10)));
        assert_eq!(e.get("y"), Some(&Value::String("hello".into())));
        assert_eq!(e.get("z"), None);
    }

    #[test]
    fn test_set_updates_existing() {
        let mut e = env_with_values();
        e.set("x", Value::Int(99));
        assert_eq!(e.get("x"), Some(&Value::Int(99)));
    }

    #[test]
    fn test_set_creates_new() {
        let mut e = env_with_values();
        e.set("z", Value::Bool(true));
        assert_eq!(e.get("z"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_scope_isolation() {
        let mut e = env_with_values();
        e.push_scope();
        e.define("x", Value::Int(42));
        // inner scope shadow
        assert_eq!(e.get("x"), Some(&Value::Int(42)));
        e.pop_scope();
        // original restored
        assert_eq!(e.get("x"), Some(&Value::Int(10)));
    }

    #[test]
    fn test_scope_depth() {
        let mut e = Environment::new();
        assert_eq!(e.scope_depth(), 1);
        e.push_scope();
        assert_eq!(e.scope_depth(), 2);
        e.push_scope();
        assert_eq!(e.scope_depth(), 3);
        e.pop_scope();
        assert_eq!(e.scope_depth(), 2);
    }

    #[test]
    fn test_contains() {
        let e = env_with_values();
        assert!(e.contains("x"));
        assert!(!e.contains("z"));
    }

    #[test]
    fn test_all_globals() {
        let e = env_with_values();
        let all = e.all();
        assert_eq!(all.len(), 2);
        assert!(all.contains_key("x"));
        assert!(all.contains_key("y"));
    }
}
