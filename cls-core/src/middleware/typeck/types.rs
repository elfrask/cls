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
                _ => Type::Value,
            },
            Expression::Array(_) => Type::Array(Box::new(Type::Value)),
            Expression::Identifier(_, _) => Type::Value,
            _ => Type::Value,
        }
    }


    /// Tipo de una función a partir de su declaración.
    pub(crate) fn function_decl_type(&mut self, f: &FunctionDecl) -> Type {        let params: Vec<Type> = f.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_type_annotation(ta))
                .unwrap_or(Type::Value))
            .collect();
        let ret = f.return_type.as_ref()
            .map(|ta| self.resolve_type_annotation(ta))
            .unwrap_or(Type::Void);
        // Función nativa (FFI): el retorno usa el mapeo C (`CArray`/`CRecord`/
        // `Struct(Nombre)`), igual que el emisor.
        let ret = if f.native.is_some() { self.map_native_ret(ret) } else { ret };
        Type::Fun(params, Box::new(ret))
    }


    /// Mapeo del retorno de una función nativa: `CArray`/`CRecord` son punteros
    /// al layout indexables como Array/Record; `Struct(Nombre)` referencia un
    /// `structure` declarado (acceso a campos por offsets). Los escalares C se
    /// mapean a su tipo CLS equivalente **independientemente de los params**
    /// (si no, `CString` quedaba como `Named("CString")` y no era asignable a
    /// `String`; además la inferencia genérica de `check_call` lo trataba como
    /// variable de tipo y lo sustituía según los args).
    pub(crate) fn map_native_ret(&self, ret: Type) -> Type {
        match ret {
            Type::Named(n, _) if n == "CString" => Type::String,
            Type::Named(n, _)
                if matches!(
                    n.as_str(),
                    "CInt" | "CUInt" | "CShort" | "CUShort" | "CChar" | "CUChar"
                        | "CLong" | "CULong"
                ) =>
            {
                Type::Int
            }
            Type::Named(n, _) if n == "CDouble" || n == "Double" => Type::Float,
            Type::Named(n, _) if n == "CBool" => Type::Bool,
            Type::Named(n, _) if n == "CArray" => Type::Array(Box::new(Type::Value)),
            Type::Named(n, _) if n == "CRecord" => {
                Type::Record(Box::new(Type::String), Box::new(Type::Value))
            }
            Type::Named(n, ref args) if n == "Struct" => {
                if let Some(Type::Named(sn, _)) = args.first() {
                    Type::Named(sn.clone(), vec![])
                } else {
                    Type::Named("Struct".to_string(), args.clone())
                }
            }
            r => r,
        }
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
            TypeKind::Any => {
                return self.error(
                    "`Any` ya no es un tipo válido: CLS es estático. Usá un tipo concreto \
                     (Int, Float, String, Bool, Char, un enum/clase/structure, una tupla/unión…) \
                     o `Value`/`JSON`/`Cmx` para valores dinámicos tipados.",
                    ann.span.clone(),
                );
            }
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
                // ignoran o resuelven a Value). Conflicto de tipo = error.
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
                // Tipo calificado de módulo importado: `alias::Clase` /
                // `alias::Enum` / `alias::Alias` (el alias está en
                // import_aliases). Se resuelve contra los exports del módulo.
                if let Some((alias, member)) = name.split_once("::") {
                    if self.import_aliases.contains_key(alias) {
                        if let Some(t) = self.module_member_type(alias, member) {
                            // Aplicar params genéricos si el export los tiene
                            // (p. ej. `alias::Caja<Int>`): por ahora se devuelve
                            // el tipo del export sin instanciar (igual que las
                            // clases locales con params).
                            return t;
                        }
                        return self.error(
                            &format!(
                                "El módulo '{}' no exporta el tipo '{}'",
                                alias, member
                            ),
                            ann.span.clone(),
                        );
                    }
                    // alias desconocido: dejar Named calificado (error claro en
                    // el uso, no acá)
                    return Type::Named(name.clone(), vec![]);
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
                    "Callable" => Type::Callable,
                    // Record<K, V> -> diccionario tipado
                    "Record" if param_types.len() == 2 => Type::Record(
                        Box::new(param_types[0].clone()),
                        Box::new(param_types[1].clone()),
                    ),
                    name if self.interfaces.contains_key(name) => {
                        let info = self.interfaces[name].clone();
                        let bind = self.interface_bindings(&info, &param_types);
                        let (mut fields, signatures) = self.interface_all_members(&info, &bind);
                        fields.extend(signatures);
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
            _ => Type::Value,
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
                let (fields, signatures) = self.interface_all_members(&info, &b);
                match access {
                    TypeAccess::Key(key) => {
                        if let Some((_, ty)) = fields.iter().find(|(n, _)| n == key) {
                            return ty.clone();
                        }
                        if let Some((_, ty)) = signatures.iter().find(|(n, _)| n == key) {
                            return ty.clone();
                        }
                        return self.error(
                            &format!("Interface '{}' no tiene miembro '{}'", name, key),
                            base.span.clone(),
                        );
                    }
                    TypeAccess::Index(i) => {
                        let mut order: Vec<Type> = fields.into_iter().map(|(_, t)| t).collect();
                        order.extend(signatures.into_iter().map(|(_, t)| t));
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
                    .unwrap_or(Type::Value),
                _ => Type::Value,
            },
            TypeAccess::Index(i) => match base_type {
                Type::Tuple(ts) => ts.get(*i).cloned().unwrap_or(Type::Value),
                Type::Array(inner) => *inner,
                Type::Union(ts) => ts.get(*i).cloned().unwrap_or(Type::Value),
                _ => Type::Value,
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
                bindings.insert(tp.name.clone(), Type::Value);
            }
        }
        bindings
    }


    /// Campos y métodos de una interface incluyendo los heredados por `extends`.
    /// Los padres se resuelven primero; un campo redefinido por el hijo conserva
    /// su posición y reemplaza el tipo. `seen` evita ciclos de herencia.
    pub(crate) fn interface_all_members(
        &mut self,
        info: &InterfaceInfo,
        bindings: &HashMap<String, Type>,
    ) -> (Vec<(String, Type)>, Vec<(String, Type)>) {
        let mut seen = std::collections::HashSet::new();
        self.interface_all_members_rec(info, bindings, &mut seen)
    }

    fn interface_all_members_rec(
        &mut self,
        info: &InterfaceInfo,
        bindings: &HashMap<String, Type>,
        seen: &mut std::collections::HashSet<String>,
    ) -> (Vec<(String, Type)>, Vec<(String, Type)>) {
        if !seen.insert(info.name.clone()) {
            return (Vec::new(), Vec::new());
        }

        let mut fields: Vec<(String, Type)> = Vec::new();
        let mut signatures: Vec<(String, Type)> = Vec::new();
        let mut field_pos: HashMap<String, usize> = HashMap::new();
        let mut signature_pos: HashMap<String, usize> = HashMap::new();

        for parent in &info.extends {
            if let TypeKind::Named(parent_name, args) = &parent.kind {
                if let Some(parent_info) = self.interfaces.get(parent_name).cloned() {
                    let arg_types: Vec<Type> = args
                        .iter()
                        .map(|a| self.resolve_annotation_with(a, bindings))
                        .collect();
                    let parent_bind = self.interface_bindings(&parent_info, &arg_types);
                    let (parent_fields, parent_signatures) =
                        self.interface_all_members_rec(&parent_info, &parent_bind, seen);
                    for (name, ty) in parent_fields {
                        if let Some(&idx) = field_pos.get(&name) {
                            if fields[idx].1 != ty {
                                self.error(
                                    &format!(
                                        "El campo '{}' tiene tipos incompatibles en las interfaces base de '{}'",
                                        name, info.name
                                    ),
                                    parent.span.clone(),
                                );
                            }
                            fields[idx].1 = ty;
                        } else {
                            field_pos.insert(name.clone(), fields.len());
                            fields.push((name, ty));
                        }
                    }
                    for (name, ty) in parent_signatures {
                        if let Some(&idx) = signature_pos.get(&name) {
                            signatures[idx].1 = ty;
                        } else {
                            signature_pos.insert(name.clone(), signatures.len());
                            signatures.push((name, ty));
                        }
                    }
                } else {
                    self.error(
                        &format!(
                            "La interface '{}' extiende '{}', que no está definida",
                            info.name, parent_name
                        ),
                        parent.span.clone(),
                    );
                }
            } else {
                self.error(
                    &format!("La interface '{}' solo puede extender interfaces", info.name),
                    parent.span.clone(),
                );
            }
        }

        for name in &info.field_order {
            if let Some(ta) = info.fields.get(name) {
                let ty = self.resolve_annotation_with(ta, bindings);
                if let Some(&idx) = field_pos.get(name) {
                    fields[idx].1 = ty;
                } else {
                    field_pos.insert(name.clone(), fields.len());
                    fields.push((name.clone(), ty));
                }
            }
        }
        for name in &info.signature_order {
            if let Some(sig) = info.signatures.get(name) {
                let ty = self.signature_type(sig, bindings);
                if let Some(&idx) = signature_pos.get(name) {
                    signatures[idx].1 = ty;
                } else {
                    signature_pos.insert(name.clone(), signatures.len());
                    signatures.push((name.clone(), ty));
                }
            }
        }

        (fields, signatures)
    }




    /// Tipo `fun(params) -> ret` de una signature, con genéricos aplicados.
    pub(crate) fn signature_type(&mut self, sig: &SignatureDecl, bindings: &HashMap<String, Type>) -> Type {
        let params: Vec<Type> = sig.params.iter()
            .map(|p| p.type_ann.as_ref()
                .map(|ta| self.resolve_annotation_with(ta, bindings))
                .unwrap_or(Type::Value))
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