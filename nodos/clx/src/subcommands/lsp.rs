pub fn execute(args: &[String]) -> i32 {
    let silent = args.iter().any(|a| a == "--silent" || a == "-s");
    crate::lsp::run_server(silent);
    0
}
