//! TypeChecker â€” tests (Fase 1: extraido de middleware/typeck.rs).

#[cfg(test)]
mod tests {
    use super::super::*;
use crate::middleware::TypeChecker;
use crate::config::types::TypesConfig;
    use crate::frontend::{Lexer, Parser};
    use crate::error::Diagnostic;
    use crate::error::diagnostic::Severity;

    /// Parsea y chequea un source, devolviendo los diagnostics.
    fn check_source(src: &str, strict: bool) -> Vec<Diagnostic> {
        let toks = Lexer::new(src).tokenize().expect("tokenize");
        let module = Parser::new(toks).parse().expect("parse");
        let config = TypesConfig { check: true, strict, ..Default::default() };
        let mut tc = TypeChecker::new(config);
        tc.check(&module).expect("check no debe fallar");
        tc.diagnostics().to_vec()
    }

    fn count_errors(diags: &[Diagnostic]) -> usize {
        diags.iter().filter(|d| matches!(d.severity, Severity::Error)).count()
    }

    #[test]
    fn tuple_valid() {
        let d = check_source("function f() { var a: (Int, String) = (1, \"x\"); };", true);
        assert_eq!(count_errors(&d), 0, "tupla valida: {:?}", d);
    }

    #[test]
    fn tuple_invalid_slot() {
        let d = check_source("function f() { var a: (Int, String) = (1, 2); };", true);
        assert_eq!(count_errors(&d), 1, "slot 2 es Int no String: {:?}", d);
    }

    #[test]
    fn union_literal_valid() {
        let src = "alias Color = \"red\" | \"green\"; function f() { var c: Color = \"red\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "{:?}", d);
    }

    #[test]
    fn union_literal_invalid() {
        let src = "alias Color = \"red\" | \"green\"; function f() { var c: Color = \"purple\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "purple no esta en la union: {:?}", d);
    }

    #[test]
    fn alias_function_type() {
        let src = "alias Fn = (Int) -> Int; function f() { };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "alias de funcion: {:?}", d);
    }

    #[test]
    fn interface_extract_default() {
        let src = "interface H<T=Int> { num: T, }; function f() { var n: H[\"num\"] = 1; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "H[\"num\"] con default Int: {:?}", d);
    }

    #[test]
    fn interface_extract_with_arg() {
        let src = "interface H<T=Int> { num: T, }; function f() { var s: H<String>[\"num\"] = \"x\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "H<String>[\"num\"] es String: {:?}", d);
    }

    #[test]
    fn generic_function() {
        let src = "function id<T>(x: T) -> T { return x; }; function f() { var g: Int = id(5); var h: String = id(\"a\"); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "genericos: {:?}", d);
    }

    #[test]
    fn phantom_not_substituted() {
        let src = "interface M<T> { real: T, ghost: !T, }; function f() { var r: M<String>[\"real\"] = \"ok\"; var g: M<String>[\"ghost\"]; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "phantom: {:?}", d);
    }

    #[test]
    fn enum_typed_ok() {
        let src = "enum Color { Rojo, Verde, }; function f() { var c: Color = Color.Rojo; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "enum: {:?}", d);
    }

    #[test]
    fn enum_typed_wrong() {
        let src = "enum Color { Rojo, Verde, }; function f() { var c: Color = 5; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "Int no es Color: {:?}", d);
    }

    #[test]
    fn record_typed() {
        let src = "function f() { var d: Record<String, Int> = {a: 1}; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "record: {:?}", d);
    }

    #[test]
    fn tuple_dynamic_index_union() {
        // í­ndice diní¡mico sobre tupla â†’ unión; no debe dar error en estricto
        let src = "function f() { var a: (Int, String) = (1, \"x\"); var i = 0; var v = a[i]; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "indice dinamico: {:?}", d);
    }

    #[test]
    fn tuple_access_by_literal() {
        // t[1] con í­ndice literal â†’ slot exacto
        let src = "function f() { var a: (Int, String) = (1, \"x\"); var n: Int = a[0]; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "indice literal: {:?}", d);
    }

    #[test]
    fn call_arg_type_mismatch() {
        // Tarea 1: arg Int a param String â†’ error en estricto (con firma conocida)
        let src = "function f(x: String) -> String { return x; }; function g() { var y = f(123); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "Int a param String: {:?}", d);
    }

    #[test]
    fn call_arg_type_ok() {
        let src = "function f(x: String) -> String { return x; }; function g() { var y = f(\"ok\"); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "String a param String: {:?}", d);
    }

    #[test]
    fn call_arg_promotion_int_to_float() {
        // int â†’ float es asignable; no debe dar error
        let src = "function f(x: Float) -> Float { return x; }; function g() { var y = f(5); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "int a param Float: {:?}", d);
    }

    #[test]
    fn generic_array_param_no_false_positive() {
        // T[] sin binding (param anidado en contenedor) â†’ no validar (sin falso positivo)
        let src = "function first<T>(a: T[]) -> T { return a[0]; }; function g() { var y = first([1,2,3]); };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "T[] no debe false-positivar: {:?}", d);
    }

    #[test]
    fn implements_missing_member_errors() {
        let src = "interface I { num: Int, }; class A implements I { var num: String = \"x\"; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "campo con tipo incompatibble: {:?}", d);
    }

    #[test]
    fn implements_ok() {
        let src = "interface I { num: Int, }; class A implements I { var num: Int = 1; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 0, "conformidad ok: {:?}", d);
    }

    #[test]
    fn implements_unknown_interface_errors() {
        let src = "class A implements NoExiste { var num: Int = 1; };";
        let d = check_source(src, true);
        assert_eq!(count_errors(&d), 1, "interface no definida: {:?}", d);
}
}
