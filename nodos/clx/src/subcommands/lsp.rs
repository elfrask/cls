pub fn execute(_args: &[String]) -> i32 {
    eprintln!("[clx lsp] Iniciando servidor LSP...");
    crate::lsp::run_server();
    0
}
