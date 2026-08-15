//! AST â€” display (Fase 1: extraido de frontend/ast.rs).

use crate::frontend::ast::*;
use std::fmt;


/// Formatea una expresiÃ³n como cÃ³digo CLS legible (para mensajes de error).
/// NO usa Debug del AST â€” el usuario debe poder leer quÃ© fallÃ³. Es la Ãºnica
/// implementaciÃ³n compartida por typeck y el backend WASM.
pub fn expr_display(expr: &Expression) -> String {
    use crate::frontend::token::Operator;
    match expr {
        Expression::Literal(l) => match &l.kind {
            LiteralKind::Int(v) => v.to_string(),
            LiteralKind::Float(v) => v.to_string(),
            LiteralKind::String(s) => format!("\"{}\"", s),
            LiteralKind::Bool(b) => b.to_string(),
            LiteralKind::Char(c) => format!("'{}'", c),
            LiteralKind::Null => "null".to_string(),
            LiteralKind::Unknown => "?".to_string(),
        },
        Expression::Identifier(name, _) => name.clone(),
        Expression::Binary(b) => {
            let op = match b.op {
                Operator::Plus => "+",
                Operator::Minus => "-",
                Operator::Star => "*",
                Operator::Slash => "/",
                Operator::Percent => "%",
                Operator::StarStar => "**",
                Operator::StrictEqual => "==",
                Operator::NotEqual => "!=",
                Operator::LessThan => "<",
                Operator::LessEqual => "<=",
                Operator::GreaterThan => ">",
                Operator::GreaterEqual => ">=",
                Operator::And => "&&",
                Operator::Or => "||",
                Operator::In => "in",
                Operator::Is => "is",
                Operator::Caret => "^",
                Operator::ShiftLeft => "<<",
                Operator::ShiftRight => ">>",
                Operator::PlusEqual => "+=",
                Operator::MinusEqual => "-=",
                Operator::StarEqual => "*=",
                Operator::SlashEqual => "/=",
                Operator::PercentEqual => "%=",
                _ => "?",
            };
            format!(
                "({} {} {})",
                expr_display(&b.left),
                op,
                expr_display(&b.right)
            )
        }
        Expression::Unary(u) => {
            let op = match u.op {
                UnaryOp::Negate => "-",
                UnaryOp::Not => "!",
                UnaryOp::BitwiseNot => "~",
                UnaryOp::TypeOf => "typeof ",
                UnaryOp::PostInc => "++",
                UnaryOp::PostDec => "--",
                UnaryOp::PreInc => "++",
                UnaryOp::PreDec => "--",
            };
            format!("{}{}", op, expr_display(&u.operand))
        }
        Expression::Call(c) => format!(
            "{}({})",
            expr_display(&c.callee),
            c.args
                .iter()
                .map(expr_display)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expression::MemberAccess(m) => format!("{}.{}", expr_display(&m.object), m.member),
        Expression::Index(i) => format!("{}[{}]", expr_display(&i.object), expr_display(&i.index)),
        Expression::Array(a) => format!(
            "[{}]",
            a.elements
                .iter()
                .map(expr_display)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expression::Tuple(t) => format!(
            "({})",
            t.elements
                .iter()
                .map(expr_display)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expression::Record(r) => format!(
            "{{{}}}",
            r.entries
                .iter()
                .map(|(k, v)| format!("{}: {}", k, expr_display(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expression::ArrowFunction(_) => "fn(...)".to_string(),
        Expression::Conditional(c) => format!(
            "({} ? {} : {})",
            expr_display(&c.condition),
            expr_display(&c.then_expr),
            expr_display(&c.else_expr)
        ),
        Expression::Assignment(a) => {
            format!("{} = {}", expr_display(&a.target), expr_display(&a.value))
        }
        Expression::Parenthesized(inner, _) => format!("({})", expr_display(inner)),
        Expression::StringInterpolation(s) => {
            let mut out = String::from("\"");
            for part in &s.parts {
                match part {
                    InterpolationPart::Text(t) => out.push_str(t),
                    InterpolationPart::Expr(e) => {
                        out.push_str("${");
                        out.push_str(&expr_display(e));
                        out.push('}');
                    }
                }
            }
            out.push('"');
            out
        }
        Expression::Cmx(c) => format!("<{} />", c.tag),
        Expression::NamespaceAccess(ns, name, _) => format!("{}::{}", ns, name),
        Expression::Await(inner, _) => format!("await {}", expr_display(inner)),
    }
}


impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::VarDecl(v) => f.write_fmt(format_args!("var {} = ...", v.name)),
            Statement::ConstDecl(v) => f.write_fmt(format_args!("const {} = ...", v.name)),
            Statement::FunctionDecl(fd) => f.write_fmt(format_args!("function {}(...) -> ...", fd.name)),
            Statement::If(_) => f.write_str("if (...)"),
            Statement::While(_) => f.write_str("while (...)"),
            Statement::Loop(_) => f.write_str("loop"),
            Statement::For(_) => f.write_str("for (...)"),
            Statement::ForEach(_) => f.write_str("for each ..."),
            Statement::Switch(_) => f.write_str("switch (...)"),
            Statement::Try(_) => f.write_str("try"),
            Statement::With(_) => f.write_str("with ..."),
            Statement::Return(_) => f.write_str("return"),
            Statement::Break(_) => f.write_str("break"),
            Statement::Continue(_) => f.write_str("continue"),
            Statement::ClassDecl(c) => f.write_fmt(format_args!("class {}", c.name)),
            Statement::StructureDecl(s) => f.write_fmt(format_args!("structure {}", s.name)),
            Statement::InterfaceDecl(i) => f.write_fmt(format_args!("interface {}", i.name)),
            Statement::ModuleDecl(m) => f.write_fmt(format_args!("module {}", m.name)),
            Statement::NamespaceDecl(n) => f.write_fmt(format_args!("namespace {}", n.name)),
            Statement::TypeAlias(t) => f.write_fmt(format_args!("alias {} = ...", t.name)),
            Statement::EnumDecl(e) => f.write_fmt(format_args!("enum {}", e.name)),
            Statement::Import(i) => f.write_fmt(format_args!("import \"{}\"", i.path)),
            Statement::FromImport(fi) => f.write_fmt(format_args!("from \"{}\" import ...", fi.path)),
            Statement::Include(i) => f.write_fmt(format_args!("include \"{}\"", i.path)),
            Statement::Extension(e) => f.write_fmt(format_args!("extension \"{}\" as {}", e.library, e.kind.name())),
            Statement::When(w) => f.write_fmt(format_args!("when {{ {} rama(s) }}", w.branches.len())),
            Statement::Expression(e) => f.write_fmt(format_args!("expr: {:?}", e)),
            Statement::Config(c) => f.write_fmt(format_args!("#config({} = {})", c.key, c.value)),
            Statement::Cmx(c) => f.write_fmt(format_args!("<{}>", c.tag)),
            Statement::Meta(m) => f.write_fmt(format_args!("#{}", m.name)),
        }
    }
}
