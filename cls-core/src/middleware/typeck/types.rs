//! TypeChecker - resolución de anotaciones y genéricos (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {


    /// ¿El tipo aún contiene un type param genérico sin binding (Named sin args
    /// que no está en bindings)? Si sí, la firma no está completamente resuelta
    /// y el argumento no se puede validar de forma fiable (p.ej. `T[]`).
    pub(crate) fn has_unbound_generic(&self, ty: &Type, bindings: &HashMap<String, Type>) -> bool {
        match ty {
            Type::Named(n, ps) => {
                if ps.is_empty() {
                    !bindings.contains_key(n)
                } else {
                    ps.iter().any(|p| self.has_unbound_generic(p, bindings))
                }
            }
            Type::Array(inner) => self.has_unbound_generic(inner, bindings),
            Type::Tuple(ts) => ts.iter().any(|t| self.has_unbound_generic(t, bindings)),
            Type::Record(k, v) => {
                self.has_unbound_generic(k, bindings) || self.has_unbound_generic(v, bindings)
            }
            Type::Shape(fields) => fields.iter().any(|(_, t)| self.has_unbound_generic(t, bindings)),
            Type::Fun(ps, r) => {
                ps.iter().any(|p| self.has_unbound_generic(p, bindings))
                    || self.has_unbound_generic(r, bindings)
            }
            Type::Union(ts) => ts.iter().any(|t| self.has_unbound_generic(t, bindings)),
            _ => false,
        }
    }


    /// Sustituye type params (Named sin args) por sus bindings en un tipo.
    pub(crate) fn substitute(&self, ty: &Type, bindings: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Named(n, params) => {
                if params.is_empty() {
                    if let Some(b) = bindings.get(n) {
                        b.clone()
                    } else {
                        ty.clone()
                    }
                } else {
                    Type::Named(
                        n.clone(),
                        params.iter().map(|p| self.substitute(p, bindings)).collect(),
                    )
                }
            }
            Type::Array(inner) => Type::Array(Box::new(self.substitute(inner, bindings))),
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.substitute(t, bindings)).collect()),
            Type::Union(ts) => Type::Union(ts.iter().map(|t| self.substitute(t, bindings)).collect()),
            Type::Record(k, v) => Type::Record(
                Box::new(self.substitute(k, bindings)),
                Box::new(self.substitute(v, bindings)),
            ),
            Type::Fun(ps, r) => Type::Fun(
                ps.iter().map(|p| self.substitute(p, bindings)).collect(),
                Box::new(self.substitute(r, bindings)),
            ),
            _ => ty.clone(),
        }
    }


    /// Tipo de un valor simple (literal/identificador) para exports sin anotación.
    pub(crate) fn infer_literal_type(&mut self, val: &Expression) -> Type {
        match val {
            Expression::Literal(l) => match &l.kind {
                LiteralKind::Int(_) => Type::Int,
                LiteralKind::Float(_) => Type::Float,
                LiteralKind::String(_) => Type::String,
                LiteralKind::Bool(_) => Type::Bool,
                LiteralKind::Char(_) => Type::Char,
                LiteralKind::Null => Type::Null,
                _ => Type::Any,
            },
            Expression::Array(_) => Type::Array(Box::new(Type::Any)),
            Expression::Identifier(_, _) => Type::Any,
            _ => Type::Any,
        }
    }


    /// Tipo de una función a partir de su declaración.
    pub(crate) fn function_decl_type(&mut self, f: &FunctionDecl) -> Type {        let params: Vec<Type> = f.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_type_annotation(ta))
                .unwrap_or(Type::Any))
            .collect();
        let ret = f.return_type.as_ref()
            .map(|ta| self.resolve_type_annotation(ta))
            .unwrap_or(Type::Void);
        Type::Fun(params, Box::new(ret))
    }


    // ===========================================
    // Type resolution
    // ===========================================

    pub fn resolve_type_annotation(&mut self, ann: &TypeAnnotation) -> Type {
        self.resolve_annotation_with(ann, &HashMap::new())
    }


    /// Resuelve una anotación bajo un contexto de type params (bindings T->tipo).
    pub(crate) fn resolve_annotation_with(
        &mut self,
        ann: &TypeAnnotation,
        bindings: &HashMap<String, Type>,
    ) -> Type {
        match &ann.kind {
            TypeKind::Int => Type::Int,
            TypeKind::Float => Type::Float,
            TypeKind::String => Type::String,
            TypeKind::Bool => Type::Bool,
            TypeKind::Char => Type::Char,
            TypeKind::Any => Type::Any,
            TypeKind::Unknown => Type::Unknown,
            TypeKind::Null => Type::Null,
            TypeKind::Void => Type::Void,
            TypeKind::Empty => Type::Empty,
            TypeKind::Json => Type::Json,
            TypeKind::Value => Type::Value,
            TypeKind::Array(inner) => {
                Type::Array(Box::new(self.resolve_annotation_with(inner, bindings)))
            }
            TypeKind::Tuple(types) => Type::Tuple(
                types.iter()
                    .map(|t| self.resolve_annotation_with(t, bindings))
                    .collect(),
            ),
            TypeKind::Union(types) => Type::Union(
                types.iter()
                    .map(|t| self.resolve_annotation_with(t, bindings))
                    .collect(),
            ),
            TypeKind::Literal(lit) => self.literal_type(lit),
            TypeKind::Access(base, access) => {
                self.resolve_type_access(base, access, bindings)
            }
            // Phantom: !T se resuelve SIN sustituir type params (no unifica)
            TypeKind::Phantom(inner) => self.resolve_annotation_with(inner, &HashMap::new()),
            TypeKind::Record(k, v) => {
                Type::Record(
                    Box::new(self.resolve_annotation_with(k, bindings)),
                    Box::new(self.resolve_annotation_with(v, bindings)),
                )
            }
            TypeKind::Shape(fields) => {
                Type::Shape(
                    fields.iter()
                        .map(|(n, ta)| (n.clone(), self.resolve_annotation_with(ta, bindings)))
                        .collect(),
                )
            }
            TypeKind::Intersection(members) => {
                // Merge de shapes: campos de todos los miembros (los no-shape se
                // ignoran o resuelven a Any). Conflicto de tipo = error.
                let mut out: Vec<(String, Type)> = Vec::new();
                for m in members {
                    let t = self.resolve_annotation_with(m, bindings);
                    if let Type::Shape(fields) = t {
                        for (n, ty) in fields {
                            if let Some((_, existing)) = out.iter_mut().find(|(en, _)| *en == n) {
                                if *existing != ty {
                                    return self.error(
                                        &format!("Campo '{}' con tipos incompatibles en la conjunción de shapes", n),
                                        ann.span.clone(),
                                    );
                                }
                            } else {
                                out.push((n, ty));
                            }
                        }
                    }
                }
                Type::Shape(out)
            }
            TypeKind::Fun(params, ret) => {
                let param_types: Vec<Type> = params.iter()
                    .map(|p| self.resolve_annotation_with(p, bindings))
                    .collect();
                Type::Fun(param_types, Box::new(self.resolve_annotation_with(ret, bindings)))
            }
            TypeKind::I32 => Type::I32,
            TypeKind::I64 => Type::I64,
            TypeKind::I16 => Type::I16,
            TypeKind::I8 => Type::I8,
            TypeKind::F32 => Type::F32,
            TypeKind::F64 => Type::F64,
            TypeKind::Cmx => Type::Cmx,
            TypeKind::Named(name, params) => {
                // Type param (T, U) del contexto genérico
                if let Some(t) = bindings.get(name) {
                    return t.clone();
                }
                let param_types: Vec<Type> = params.iter()
                    .map(|p| self.resolve_annotation_with(p, bindings))
                    .collect();
                // Si es un nombre conocido, mapearlo
                match name.as_str() {
                    "Integer" => Type::Int,
                    "Float" => Type::Float,
                    "Character" => Type::Char,
                    "Boolean" => Type::Bool,
                    // Record<K, V> -> diccionario tipado
                    "Record" if param_types.len() == 2 => Type::Record(
                        Box::new(param_types[0].clone()),
                        Box::new(param_types[1].clone()),
                    ),
                    name if self.interfaces.contains_key(name) => {
                        let info = self.interfaces[name].clone();
                        let bind = self.interface_bindings(&info, &param_types);
                        let mut fields: Vec<(String, Type)> = info
                            .field_order
                            .iter()
                            .filter_map(|fn_| info.fields.get(fn_).map(|ta| (fn_.clone(), self.resolve_annotation_with(ta, &bind))))
                            .collect();
                        for name_sig in &info.signature_order {
                            if let Some(sig) = info.signatures.get(name_sig) {
                                fields.push((name_sig.clone(), self.signature_type(sig, &bind)));
                            }
                        }
                        Type::Shape(fields)
                    }
                    _ => {
                        self.lookup(name)
                            .cloned()
                            .unwrap_or(Type::Named(name.clone(), param_types))
                    }
                }
            }
        }
    }


    /// Convierte un literal AST a un literal type (o su tipo base).
    pub(crate) fn literal_type(&self, lit: &LiteralKind) -> Type {
        match lit {
            LiteralKind::String(s) => Type::Literal(LitVal::Str(s.clone())),
            LiteralKind::Int(i) => Type::Literal(LitVal::Int(*i)),
            LiteralKind::Float(f) => Type::Literal(LitVal::Float(f.to_bits())),
            LiteralKind::Bool(b) => Type::Literal(LitVal::Bool(*b)),
            _ => Type::Any,
        }
    }


    /// Resuelve un acceso a tipo: `T["field"]` o `T[0]`.
    pub(crate) fn resolve_type_access(
        &mut self,
        base: &TypeAnnotation,
        access: &TypeAccess,
        bindings: &HashMap<String, Type>,
    ) -> Type {
        // Caso interface nombrada (con args opcionales): resolver miembros con genéricos
        if let TypeKind::Named(name, arg_anns) = &base.kind {
            if let Some(info) = self.interfaces.get(name).cloned() {
                let arg_types: Vec<Type> = arg_anns.iter()
                    .map(|a| self.resolve_annotation_with(a, bindings))
                    .collect();
                let b = self.interface_bindings(&info, &arg_types);
                match access {
                    TypeAccess::Key(key) => {
                        if let Some(ta) = info.fields.get(key) {
                            return self.resolve_annotation_with(ta, &b);
                        }
                        if let Some(sig) = info.signatures.get(key) {
                            return self.signature_type(sig, &b);
                        }
                        return self.error(
                            &format!("Interface '{}' no tiene miembro '{}'", name, key),
                            base.span.clone(),
                        );
                    }
                    TypeAccess::Index(i) => {
                        let order = self.interface_member_types(&info, &b);
                        return order.get(*i).cloned().unwrap_or_else(|| self.error(
                            &format!("Index '{}' fuera de rango en interface '{}'", i, name),
                            base.span.clone(),
                        ));
                    }
                }
            }
        }

        // Fallback: resolver el tipo base y aplicar sobre tipos compuestos
        let base_type = self.resolve_annotation_with(base, bindings);
        match access {
            TypeAccess::Key(key) => match &base_type {
                Type::Record(_, v) => (**v).clone(),
                Type::Shape(fields) => fields.iter()
                    .find(|(n, _)| n == key)
                    .map(|(_, t)| t.clone())
                    .unwrap_or(Type::Any),
                _ => Type::Any,
            },
            TypeAccess::Index(i) => match base_type {
                Type::Tuple(ts) => ts.get(*i).cloned().unwrap_or(Type::Any),
                Type::Array(inner) => *inner,
                Type::Union(ts) => ts.get(*i).cloned().unwrap_or(Type::Any),
                _ => Type::Any,
            },
        }
    }


    /// Construye bindings T->tipo para los type params de una interface.
    pub(crate) fn interface_bindings(&mut self, info: &InterfaceInfo, args: &[Type]) -> HashMap<String, Type> {
        let mut bindings = HashMap::new();
        for (i, tp) in info.type_params.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                bindings.insert(tp.name.clone(), arg.clone());
            } else if let Some(default) = &tp.default {
                let resolved = self.resolve_annotation_with(default, &bindings);
                bindings.insert(tp.name.clone(), resolved);
            } else {
                bindings.insert(tp.name.clone(), Type::Any);
            }
        }
        bindings
    }


    /// Tipos de los campos de una interface en orden de declaración (para acceso
    /// por índice y offsets deterministas del shape). NO itera el HashMap: usa
    /// `field_order` (el orden en que se declararon los campos).
    pub(crate) fn interface_member_types(&mut self, info: &InterfaceInfo, bindings: &HashMap<String, Type>) -> Vec<Type> {
        info.field_order.iter()
            .filter_map(|name| info.fields.get(name).map(|ta| self.resolve_annotation_with(ta, bindings)))
            .collect()
    }


    /// Tipo `fun(params) -> ret` de una signature, con genéricos aplicados.
    pub(crate) fn signature_type(&mut self, sig: &SignatureDecl, bindings: &HashMap<String, Type>) -> Type {
        let params: Vec<Type> = sig.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_annotation_with(ta, bindings))
                .unwrap_or(Type::Any))
            .collect();
        let ret = sig.return_type.as_ref()
            .map(|ta| self.resolve_annotation_with(ta, bindings))
            .unwrap_or(Type::Void);
        Type::Fun(params, Box::new(ret))
    }


    /// Registra y define un alias de tipo (compile-time).
    pub(crate) fn check_type_alias(&mut self, alias: &TypeAliasDecl) {
        let type_ann = alias.type_ann.clone();
        let resolved = self.resolve_annotation_with(&type_ann, &HashMap::new());
        self.define_decl(&alias.name, resolved, &alias.span);
    }

}