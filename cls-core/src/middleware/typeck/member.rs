//! member.rs (Fase 1: extraido de cls-core/src/middleware/typeck/expressions.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_member_access(&mut self, member: &MemberAccessExpr) -> Type {
        // Módulos internos del nodo (resueltos por nombre en el JIT): se manejan
        // ANTES de evaluar el object (que no está definido como variable).
        if let Expression::Identifier(name, _) = &*member.object {
            if self.enums.contains(name) {
                return Type::Named(name.clone(), vec![]);
            }
            if name == "http" {
                return match member.member.as_str() {
                    "get" | "post" => Type::String,
                    _ => Type::Any,
                };
            }
            if name == "fs" {
                return match member.member.as_str() {
                    "exists" => Type::Bool,
                    "cwd" | "readFile" => Type::String,
                    "listDir" => Type::Array(Box::new(Type::String)),
                    _ => Type::Any,
                };
            }
            if name == "json" {
                return match member.member.as_str() {
                    // parse devuelve un JSON (objeto/array/valor dinámico tipado).
                    // El tag runtime viaja con el valor: acceso por índice, print
                    // y stringify despachan por tag sin perder el tipo.
                    "parse" => Type::Json,
                    "stringify" => Type::String,
                    _ => Type::Any,
                };
            }
            if name == "math" {
                return match member.member.as_str() {
                    "range" => Type::Array(Box::new(Type::Int)),
                    "random" => Type::Float,
                    "sqrt" | "floor" | "ceil" | "round" | "sin" | "cos" | "tan"
                    | "log" | "pow" | "min" | "max" => Type::Float,
                    "abs" => Type::Int,
                    _ => Type::Any,
                };
            }
            if name == "os" {
                return match member.member.as_str() {
                    "platform" | "arch" | "version" | "hostname" | "home"
                    | "tempdir" | "env" | "sep" => Type::String,
                    "cpus" | "pid" | "uptime" => Type::Int,
                    "isWindows" | "isUnix" => Type::Bool,
                    _ => Type::Any,
                };
            }
            if name == "path" {
                return match member.member.as_str() {
                    "join" | "basename" | "dirname" | "extname" | "resolve"
                    | "normalize" | "sep" => Type::String,
                    "isAbsolute" => Type::Bool,
                    _ => Type::Any,
                };
            }
            if name == "process" {
                return match member.member.as_str() {
                    "args" => Type::Array(Box::new(Type::String)),
                    "cwd" | "env" | "platform" | "title" => Type::String,
                    "pid" => Type::Int,
                    "exit" => Type::Void,
                    _ => Type::Any,
                };
            }
            if name == "time" {
                return match member.member.as_str() {
                    "iso" | "date" | "clock" => Type::String,
                    "now" | "seconds" | "year" | "month" | "day" | "hour"
                    | "minute" | "second" => Type::Int,
                    "sleep" => Type::Void,
                    _ => Type::Any,
                };
            }
            if name == "random" {
                return match member.member.as_str() {
                    "random" | "float" => Type::Float,
                    "int" => Type::Int,
                    "uuid" => Type::String,
                    _ => Type::Any,
                };
            }
            // net eliminado (dev-2): no hay miembros conocidos. Si el usuario
            // escribe `net.X` el typeck cae al default (Any/Unknown) y el
            // emisor lo rechaza con "miembro no soportado".
            // Para sockets: `extension` con `when` por SO en el .clsx.
            if name == "strings" {
                return match member.member.as_str() {
                    "indexOf" => Type::Int,
                    "slice" => Type::String,
                    "split" => Type::Array(Box::new(Type::String)),
                    _ => Type::Any,
                };
            }
        }
        let obj_type = self.check_expression(&member.object);
        // Color.Rojo -> el tipo del enum (si member.object es un nombre de enum)
        // Métodos/getters de primitivos (sin boxing): tipo conocido por miembro.
        match obj_type {
            Type::String => match member.member.as_str() {
                "length" => Type::Int,
                "upper" | "lower" | "trim" | "toString" => Type::String,
                "contains" | "startsWith" | "endsWith" | "isEmpty" => Type::Bool,
                _ => Type::Any,
            },
            Type::Array(elem) => match member.member.as_str() {
                "length" => Type::Int,
                "join" | "toString" => Type::String,
                "includes" | "isEmpty" => Type::Bool,
                "indexOf" => Type::Int,
                "push" | "pop" | "shift" | "unshift" | "reverse" => Type::Array(elem.clone()),
                _ => Type::Any,
            },
            Type::Tuple(_) => match member.member.as_str() {
                "length" => Type::Int,
                "join" | "toString" => Type::String,
                _ => Type::Any,
            },
            Type::Record(k, _) => match member.member.as_str() {
                "length" | "size" => Type::Int,
                "has" => Type::Bool,
                "keys" => Type::Array(k.clone()),
                "values" => Type::Array(Box::new(Type::Value)),
                "toString" => Type::String,
                _ => Type::Any,
            },
            // JSON (objeto/array dinámico) y Value: el acceso a campo devuelve
            // un `Value` (el tag runtime viaja con el valor; las operaciones
            // posteriores — str, ==, print, stringify, index — despachan por tag).
            Type::Json => match member.member.as_str() {
                "toString" => Type::String,
                "length" | "size" => Type::Int,
                "has" => Type::Bool,
                "keys" => Type::Array(Box::new(Type::String)),
                _ => Type::Value,
            },
            Type::Value => match member.member.as_str() {
                "toString" => Type::String,
                "length" => Type::Int,
                _ => Type::Value,
            },
            Type::Shape(fields) => {
                match member.member.as_str() {
                    "length" | "size" => Type::Int,
                    "keys" => Type::Array(Box::new(Type::String)),
                    "values" => Type::Array(Box::new(Type::Any)),
                    "has" => Type::Bool,
                    "toString" => Type::String,
                    name => fields.iter()
                        .find(|(n, _)| *n == name)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_else(|| self.error(
                            &format!("El record no tiene el campo '{}'", name),
                            member.span.clone(),
                        )),
                }
            }
            // Cmx: tipo de primera clase (réplica del patrón JSON, ver plan
            // completar-tipo-cmx.md). `.tag` es Value (string para tag
            // minúscula, handle para mayúscula — el tag-bit decide en
            // runtime); `.props` es Record<String, Value>; `.kind` distingue
            // texto (1) de elemento (0), necesario para un renderer CLS puro.
            Type::Cmx => match member.member.as_str() {
                "tag" => Type::Value,
                "props" => Type::Record(Box::new(Type::String), Box::new(Type::Value)),
                "children" => Type::Array(Box::new(Type::Cmx)),
                "kind" => Type::Int,
                "toString" => Type::String,
                _ => Type::Any,
            },
            Type::Int | Type::Float => match member.member.as_str() {
                "toString" => Type::String,
                "abs" => obj_type,
                _ => Type::Any,
            },
            Type::Bool | Type::Char => match member.member.as_str() {
                "toString" => Type::String,
                _ => Type::Any,
            },
            Type::Named(name, _) => {
                if let Some(members) = self.class_members.get(name.as_str()) {
                    if let Some(t) = members.get(&member.member) {
                        return t.clone();
                    }
                }
                // Campo de structure: `p.campo` -> tipo anotado del campo.
                if let Some(members) = self.struct_members.get(name.as_str()) {
                    if let Some(t) = members.get(&member.member) {
                        return t.clone();
                    }
                }
                // `Color.Rojo` / `lib::Color.Rojo` -> la variante de enum es
                // del mismo tipo (identidad con nombre del enum).
                if self.enums.contains(name.as_str()) {
                    return Type::Named(name.clone(), vec![]);
                }
                // Módulo/namespace importado: `x::miembro`.
                if let Some(t) = self.module_member_type(name.as_str(), &member.member) {
                    return t;
                }
                Type::Any
            }
            _ => Type::Any,
        }
    }

}