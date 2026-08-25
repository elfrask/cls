//! classes.rs (Fase 1: extraido de cls-core/src/middleware/typeck/statements.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_class(&mut self, c: &ClassDecl) -> Type {
        let class_type = Type::Named(c.name.clone(), vec![]);
        self.define_decl(&c.name, class_type.clone(), &c.span);
        self.push_scope();
        self.define("me", class_type.clone());
        self.define("super", class_type.clone());
        // Type params de la clase como placeholders (para fields/methods genéricos)
        for tp in &c.type_params {
            self.define(&tp.name, Type::Named(tp.name.clone(), vec![]));
        }
        // 1. pasada: recolectar los tipos de los miembros ANTES de chequear los
        // bodies, para que `me.campo`/`me.metodo()` resuelvan dentro del check.
        let mut members: HashMap<String, Type> = HashMap::new();
        let mut params_map: HashMap<String, Vec<Type>> = HashMap::new();
        if let Some(parent) = &c.extends {
            if let Some(pm) = self.class_members.get(parent) {
                members.extend(pm.clone());
            }
            if let Some(pp) = self.magic_params.get(parent) {
                params_map.extend(pp.clone());
            }
        }
        for member in &c.body {
            match member {
                ClassMember::Method(f) | ClassMember::Constructor(f) => {
                    members.insert(
                        f.name.clone(),
                        f.return_type
                            .as_ref()
                            .map(|t| self.resolve_type_annotation(t))
                            .unwrap_or(Type::Void),
                    );
                    params_map.insert(
                        f.name.clone(),
                        f.params
                            .iter()
                            .map(|p| {
                                p.type_ann
                                    .as_ref()
                                    .map(|t| self.resolve_type_annotation(t))
                                    .unwrap_or(Type::Any)
                            })
                            .collect(),
                    );
                }
                ClassMember::Property(v) => {
                    members.insert(
                        v.name.clone(),
                        v.type_ann
                            .as_ref()
                            .map(|t| self.resolve_type_annotation(t))
                            .unwrap_or(Type::Any),
                    );
                }
            }
        }
        self.class_members.insert(c.name.clone(), members);
        self.magic_params.insert(c.name.clone(), params_map);
        if let Some(parent) = &c.extends {
            self.class_parents.insert(c.name.clone(), parent.clone());
        }
        // 2. pasada: chequear los bodies.
        for member in &c.body {
            match member {
                ClassMember::Method(f) | ClassMember::Constructor(f) => {
                    self.check_function_decl(f);
                }
                ClassMember::Property(v) => {
                    self.check_var_decl(v, false);
                }
            }
        }
        // 3. pasada: verificar conformidad con las interfaces `implements`.
        for iface in &c.implements {
            self.check_implements(&c.name, iface, c.span.clone());
        }
        self.pop_scope();
        class_type
    }



    /// Verifica que la clase provea los campos y métodos que exige la interface.
    pub(crate) fn check_implements(&mut self, class_name: &str, iface_name: &str, span: Span) {
        let info = match self.interfaces.get(iface_name) {
            Some(i) => i.clone(),
            None => {
                self.error(
                    &format!(
                        "La clase '{}' implementa la interface '{}', que no está definida",
                        class_name, iface_name
                    ),
                    span,
                );
                return;
            }
        };
        let bind = self.interface_bindings(&info, &[]);
        let member_types: HashMap<String, Type> = self.class_members
            .get(class_name)
            .cloned()
            .unwrap_or_default();
        for fname in &info.field_order {
            let Some(ta) = info.fields.get(fname) else { continue };
            let required = self.resolve_annotation_with(ta, &bind);
            let ok = member_types
                .get(fname)
                .map(|provided| provided.is_assignable_to(&required))
                .unwrap_or(false);
            if !ok {
                self.error(
                    &format!(
                        "La clase '{}' no implementa el campo '{}: {}' exigido por la interface '{}'",
                        class_name, fname, required, iface_name
                    ),
                    span.clone(),
                );
            }
        }
        for (sig_name, sig) in &info.signatures {
            let required_fun = self.signature_type(sig, &bind);
            match member_types.get(sig_name) {
                None => {
                    self.error(
                        &format!(
                            "La clase '{}' no implementa el método '{}' exigido por la interface '{}'",
                            class_name, sig_name, iface_name
                        ),
                        span.clone(),
                    );
                }
                Some(ret) => {
                    if let Type::Fun(_, req_ret) = &required_fun {
                        if !ret.is_assignable_to(req_ret) {
                            self.error(
                                &format!(
                                    "El método '{}' de '{}' devuelve {}, la interface '{}' exige {}",
                                    sig_name, class_name, ret, iface_name, req_ret
                                ),
                                span.clone(),
                            );
                        }
                    }
                }
            }
        }
    }

}