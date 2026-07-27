use cls_runtime::{Environment, Value};
use cls_runtime::value::FunValue;
use cls_core::error::ClsResult;
use cls_core::frontend::ast::{Statement, Visibility, Expression, LiteralKind};
use std::collections::{HashSet, HashMap};

/// Carga un .clsx como módulo, devolviendo solo lo exportado
pub fn load_module(source: &str) -> ClsResult<Value> {
    let mut lexer = cls_core::frontend::Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = parser.parse()?;

    let mut sub = SubInterpreter::new();

    // Registrar math y json para que el módulo los use
    sub.env.define("math", cls_runtime::stdlib::math::module());
    sub.env.define("json", cls_runtime::stdlib::json::module());

    for stmt in &module.statements {
        sub.execute_stmt(stmt)?;
    }

    let mut entries = HashMap::new();
    for name in &sub.exports {
        if let Some(val) = sub.env.get(name) {
            entries.insert(name.clone(), val.clone());
        }
    }
    Ok(Value::Record(entries))
}

struct SubInterpreter {
    env: Environment,
    exports: HashSet<String>,
}

impl SubInterpreter {
    fn new() -> Self {
        Self { env: Environment::new(), exports: HashSet::new() }
    }

    fn execute_stmt(&mut self, stmt: &Statement) -> ClsResult<()> {
        match stmt {
            Statement::FunctionDecl(f) => {
                let fun = Value::Fun(FunValue::new_user(&f.name, f.params.clone(), f.body.clone()));
                self.env.define(&f.name, fun);
                if let Visibility::Export = f.visibility {
                    self.exports.insert(f.name.clone());
                }
            }
            Statement::VarDecl(v) => {
                let val = eval_literal(&v.value);
                self.env.define(&v.name, val);
                if let Visibility::Export = v.visibility {
                    self.exports.insert(v.name.clone());
                }
            }
            Statement::ConstDecl(v) => {
                let val = eval_literal(&v.value);
                self.env.define(&v.name, val);
                if let Visibility::Export = v.visibility {
                    self.exports.insert(v.name.clone());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn eval_literal(expr: &Option<Expression>) -> Value {
    match expr {
        Some(Expression::Literal(l)) => match &l.kind {
            LiteralKind::Int(v) => Value::Int(*v),
            LiteralKind::Float(v) => Value::Float(*v),
            LiteralKind::String(s) => Value::String(s.clone()),
            LiteralKind::Bool(b) => Value::Bool(*b),
            _ => Value::Null,
        },
        Some(Expression::Parenthesized(inner, _)) => eval_literal(&Some(*inner.clone())),
        Some(Expression::Array(arr)) => Value::Array(arr.elements.iter().map(|e| eval_literal(&Some(e.clone()))).collect()),
        Some(Expression::Record(rec)) => Value::Record(rec.entries.iter().map(|(k, e)| (k.clone(), eval_literal(&Some(e.clone())))).collect()),
        _ => Value::Null,
    }
}
