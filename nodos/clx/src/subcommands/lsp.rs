pub fn execute(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("clx lsp — Language Server");
        println!();
        println!("Uso: clx lsp [--silent]");
        println!();
        println!("  --silent, -s  No imprimir el banner de inicio");
        return 0;
    }
    let silent = args.iter().any(|a| a == "--silent" || a == "-s");
    crate::lsp::run_server(silent);
    0
}
