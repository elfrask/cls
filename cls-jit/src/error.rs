//! Presentación de errores del JIT: error CLS con trace numerado y
//! diagnósticos del typeck con línea + caret.

use cls_core::error::{ClsError, Span};
use cls_core::frontend::ast::Module as ClsModule;
use cls_runtime::error_report::{format_error, ErrorFormat, ErrorReport};

/// Muestra un error CLS con el formato estricto (trace numerado + línea + caret),
/// igual que el walker (error_report). El nodo decide el formato (Console).
pub fn show_cls_error(error: &ClsError, entry: &str, source: Option<&str>) {
    let span = match error {
        ClsError::SyntaxErrorAt(_, s) | ClsError::CompileErrorAt(_, s) => Some(s.clone()),
        _ => ClsError::extract_line_col(&error.to_string())
            .map(|(l, c)| Span::new(l as u32, c as u32, l as u32, c as u32)),
    };
    let report = ErrorReport {
        error: error.clone(),
        span,
        stack: vec![],
        import_trace: vec![],
        source_file: entry.to_string(),
        source: source.map(|s| s.to_string()),
    };
    eprintln!("{}", format_error(&report, &ErrorFormat::Console));
}

/// Muestra un diagnóstico de tipo (del typeck) con línea + caret, como `clx check`.
/// Si el span pertenece a un módulo importado (desplazado con offset 100000*n),
/// des-desplaza la línea y usa el source del módulo real.
pub fn show_type_diag(
    diag: &cls_core::error::diagnostic::Diagnostic,
    entry_source: &str,
    entry: &str,
    imports: &[(String, String, ClsModule)],
) {
    use cls_core::ansi;
    let sev = ansi::bold(true, ansi::fg(true, ansi::codes::BRIGHT_RED, "ERROR"));
    let msg = ansi::fg(true, ansi::codes::BRIGHT_RED, &diag.message);

    // Determinar el archivo real del span (desplazado -> módulo importado).
    let raw_line = diag.span.start_line;
    let col = diag.span.start_col;
    let (file_label, source, real_line) = if raw_line >= 100000 {
        // Módulo importado: offset = (raw_line / 100000) * 100000.
        let idx = (raw_line / 100000) as usize;
        let offset = idx * 100000;
        let real_line = raw_line - offset as u32;
        if let Some((path, src, _)) = imports.get(idx.saturating_sub(1)) {
            (path.clone(), src.clone(), real_line)
        } else {
            (entry.to_string(), entry_source.to_string(), real_line)
        }
    } else {
        (entry.to_string(), entry_source.to_string(), raw_line)
    };

    eprintln!(
        "[{}] {} ({}:{}:{})",
        sev, msg, file_label, real_line, col
    );
    // Línea fuente + caret (del archivo real).
    let line = real_line as usize;
    if let Some(src_line) = source.lines().nth(line.saturating_sub(1)) {
        let pad = " ".repeat(line.to_string().len());
        eprintln!("{} | {}", pad, src_line);
        eprintln!(
            "{} | {}^",
            pad,
            " ".repeat(col.saturating_sub(1) as usize)
        );
    }
}
