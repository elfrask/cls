//! TypeChecker - check_statement y chequeos de statements (Fase 1: extraido de middleware/typeck.rs).

use super::*;

impl TypeChecker {



    pub(crate) fn check_statement(&mut self, stmt: &Statement) -> Type {
        match stmt {
            Statement::VarDecl(v) => self.check_var_decl(v, false),
            Statement::ConstDecl(v) => self.check_var_decl(v, true),
            Statement::FunctionDecl(f) => self.check_function_decl(f),
            Statement::If(i) => self.check_if(i),
            Statement::While(w) => self.check_while(w),
            Statement::Loop(b) => {
                self.check_block(b);
                Type::Void
            }
            Statement::For(f) => self.check_for(f),
            Statement::ForEach(fe) => self.check_foreach(fe),
            Statement::Switch(s) => self.check_switch(s),
            Statement::Try(t) => self.check_try(t),
            Statement::With(w) => self.check_with(w),
            Statement::Return(expr) => {
                // Literal de record en return con función declarada como
                // Record<K,V> (o Shape): registrar el tipo esperado en el span
                // del literal ANTES de chequearlo, para que el backend lo emita
                // como dict (con keys) y no como shape contiguo sin claves.
                if let (Some(expected), Some(Expression::Record(rec))) =
                    (&self.current_return_type, expr.as_ref())
                {
                    if matches!(expected, Type::Record(_, _)) || matches!(expected, Type::Shape(_)) {
                        self.types_by_span.insert(rec.span.clone(), expected.clone());
                    }
                }
                let ret_type = expr.as_ref()
                    .map(|e| self.check_expression(e))
                    .unwrap_or(Type::Void);
                // Verificar que el tipo de retorno concuerde
                if let Some(expected) = &self.current_return_type {
                    if !ret_type.is_assignable_to(expected) {
                        let msg = format!(
                            "Tipo de retorno {} no coincide con el declarado {}",
                            ret_type, expected
                        );
                        let span = expr.as_ref()
                            .map(|e| expr_span(e))
                            .unwrap_or_else(|| self.current_fn_span.clone());
                        // null como centinela (p.ej. __next -> int con `return null`)
                        // se permite con null_safety: warn, no bloquea (paridad walker).
                        if self.config.null_safety && matches!(ret_type, Type::Null) {
                            self.warn(&msg, span);
                        } else if self.config.strict {
                            self.error(&msg, span);
                        } else {
                            self.warn(&msg, span);
                        }
                    }
                }
                ret_type
            }
            Statement::Break(_) => Type::Void,
            Statement::Continue(_) => Type::Void,
            Statement::Expression(e) => self.check_expression(e),
            Statement::ClassDecl(c) => self.check_class(c),
            Statement::StructureDecl(s) => {
                self.define(&s.name, Type::Named(s.name.clone(), vec![]));
                let members: HashMap<String, Type> = s.fields.iter()
                    .map(|f| {
                        let t = self.resolve_type_annotation(&f.type_ann);
                        (f.name.clone(), t)
                    })
                    .collect();
                self.struct_members.insert(s.name.clone(), members);
                Type::Void
            }
            Statement::InterfaceDecl(i) => {
                self.define(&i.name, Type::Named(i.name.clone(), vec![]));
                let fields: HashMap<String, TypeAnnotation> = i.fields.iter()
                    .map(|f| (f.name.clone(), f.type_ann.clone()))
                    .collect();
                let signatures: HashMap<String, SignatureDecl> = i.signatures.iter()
                    .map(|s| (s.name.clone(), s.clone()))
                    .collect();
                self.interfaces.insert(i.name.clone(), InterfaceInfo {
                    type_params: i.type_params.clone(),
                    fields,
                    field_order: i.fields.iter().map(|f| f.name.clone()).collect(),
                    signatures,
                    signature_order: i.signatures.iter().map(|s| s.name.clone()).collect(),
                });
                if !self.config.strict {
                    self.warn(&format!("interface '{}' solo tiene efecto en type-checker", i.name), i.span);
                }
                Type::Void
            }            Statement::TypeAlias(t) => {
                self.check_type_alias(t);
                Type::Void
            }
            Statement::EnumDecl(e) => {
                self.define(&e.name, Type::Named(e.name.clone(), vec![]));
                self.enums.insert(e.name.clone());
                Type::Void
            }
            Statement::ModuleDecl(m) => {
                self.define(&m.name, Type::Named(m.name.clone(), vec![]));
                self.push_scope();
                for stmt in &m.body {
                    self.check_statement(stmt);
                }
                self.pop_scope();
                Type::Void
            }
            Statement::NamespaceDecl(n) => {
                self.define(&n.name, Type::Named(n.name.clone(), vec![]));
                Type::Void
            }
            Statement::Import(imp) => self.check_import(imp),
            Statement::FromImport(fi) => self.check_from_import(fi),
            Statement::Include(inc) => self.check_include(inc),
            Statement::When(w) => {
                // Cada rama se chequea en su propio scope (símbolos condicionales).
                for branch in &w.branches {
                    self.push_scope();
                    self.check_block(&branch.block);
                    self.pop_scope();
                }
                Type::Void
            }
            Statement::Extension(e) => {
                // Funciones/structs/variables nativas se registran como símbolos.
                for decl in &e.declarations {
                    match decl {
                        NativeDecl::Function(f) => {
                            let mut param_tys = Vec::new();
                            for p in &f.params {
                                let t = p.type_ann.as_ref()
                                    .map(|ta| self.resolve_type_annotation(ta))
                                    .unwrap_or(Type::Any);
                                param_tys.push(t);
                            }
                            let ret = f.return_type.as_ref()
                                .map(|ta| self.resolve_type_annotation(ta))
                                .unwrap_or(Type::Void);
                            // Retorno de FFI estructurado: CArray/CRecord son
                            // punteros al layout, pero CLS los usa como
                            // Array/Record indexable (len(), [i], has()).
                            // CStruct queda opaco (Named); `Struct(Nombre)`
                            // referencia un `structure` declarado -> se tipa
                            // como el struct (acceso a campos por offsets).
                            let ret = match ret {
                                Type::Named(n, _) if n == "CArray" => {
                                    Type::Array(Box::new(Type::Any))
                                }
                                Type::Named(n, _) if n == "CRecord" => {
                                    Type::Record(Box::new(Type::String), Box::new(Type::Any))
                                }
                                Type::Named(n, ref args) if n == "Struct" => {
                                    if let Some(Type::Named(sn, _)) = args.first() {
                                        Type::Named(sn.clone(), vec![])
                                    } else {
                                        Type::Named("Struct".to_string(), args.clone())
                                    }
                                }
                                r => r,
                            };
                            self.define(&f.name, Type::Fun(param_tys, Box::new(ret)));
                        }
                        NativeDecl::Structure(s) => {
                            self.define(&s.name, Type::Named(s.name.clone(), vec![]));
                        }
                        NativeDecl::Var(v) => {
                            let t = v.type_ann.as_ref()
                                .map(|ta| self.resolve_type_annotation(ta))
                                .unwrap_or(Type::Any);
                            self.define(&v.name, t);
                        }
                    }
                }
                Type::Void
            }
            Statement::Config(_) | Statement::Meta(_) => Type::Void,
            Statement::Cmx(c) => {
                self.check_expression(&Expression::Cmx(c.clone()));
                Type::Cmx
            }
        }
    }



    pub(crate) fn check_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.check_statement(stmt);
        }
    }

}
