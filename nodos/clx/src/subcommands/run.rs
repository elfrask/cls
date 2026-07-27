use cls_runtime::{Intrinsics, Interpreter, ImportFrame};
use crate::module_loader;
use std::fs;

pub fn execute(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Uso: clx run <archivo> [--] [args...]");
        return 1;
    }
    let path = &args[0];
    let app_args: Vec<String> = args[1..].to_vec();

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error al leer '{}': {}", path, e); return 1; }
    };

    let mut lexer = cls_core::frontend::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => { super::util::show_error(&source, &e.to_string(), path); return 1; }
    };

    let mut parser = cls_core::frontend::Parser::new(tokens);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => { super::util::show_error(&source, &e.to_string(), path); return 1; }
    };

    let resolver = make_desktop_resolver();
    let mut interpreter = Interpreter::new(Intrinsics::desktop_defaults(app_args), resolver);
    interpreter.set_source_file(path.to_string());

    if let Err(e) = interpreter.execute(&module) {
        super::util::show_runtime_error(&e.to_string(), interpreter.get_import_trace(), path);
        return 1;
    }
    match interpreter.call_main() {
        Ok(code) => code,
        Err(e) => {
            super::util::show_runtime_error(&e.to_string(), interpreter.get_import_trace(), path);
            1
        }
    }
}

fn make_desktop_resolver() -> cls_runtime::ModuleResolver {
    let mut resolver = cls_runtime::ModuleResolver::new().with_core_stdlib();
    resolver.add_internal("fs", crate::modules::fs::module());
    resolver.add_internal("http", crate::modules::http::module());
    resolver.set_external(|path: String, _env: &mut cls_runtime::Environment| -> cls_core::error::ClsResult<Option<cls_runtime::Value>> {
        let candidate = format!("{}.clsx", path);
        match fs::read_to_string(&candidate) {
            Ok(source) => Ok(Some(module_loader::load_module(&source)?)),
            Err(_) => Ok(None),
        }
    });
    resolver
}
