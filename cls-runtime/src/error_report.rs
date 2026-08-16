use cls_core::error::{ClsError, Span, StackFrame};
use cls_core::ansi;
use crate::ImportFrame;

/// Reporte de error con contexto completo para formateo centralizado.
pub struct ErrorReport {
    pub error: ClsError,
    pub span: Option<Span>,
    pub stack: Vec<StackFrame>,
    pub import_trace: Vec<ImportFrame>,
    pub source_file: String,
    /// Contenido fuente (si se tiene en memoria); si es None se lee por `source_file`.
    pub source: Option<String>,
}

impl ErrorReport {
    pub fn from_runtime(
        error: ClsError,
        stack: Vec<StackFrame>,
        import_trace: &[ImportFrame],
        source_file: &str,
    ) -> Self {
        let span = error_span(&error);
        Self { error, span, stack, import_trace: import_trace.to_vec(), source_file: source_file.to_string(), source: None }
    }

    pub fn from_syntax(error: ClsError, source: &str, source_file: &str) -> Self {
        let span = error_span(&error);
        Self { error, span, stack: vec![], import_trace: vec![], source_file: source_file.to_string(), source: Some(source.to_string()) }
    }

    pub fn from_config(error: ClsError) -> Self {
        Self { error, span: None, stack: vec![], import_trace: vec![], source_file: String::new(), source: None }
    }
}

// ─── Span del error (estructurado o fallback legacy) ─────────────────────────

fn error_span(error: &ClsError) -> Option<Span> {
    match error {
        ClsError::SyntaxErrorAt(_, span) | ClsError::CompileErrorAt(_, span) => Some(span.clone()),
        _ => ClsError::extract_line_col(&error.to_string())
            .map(|(l, c)| Span::new(l as u32, c as u32, l as u32, c as u32)),
    }
}

fn error_label(error: &ClsError) -> &'static str {
    match error {
        ClsError::SyntaxError(_) | ClsError::SyntaxErrorAt(_, _) => "[Error de Sintaxis]",
        ClsError::CompileErrorAt(_, _) => "[Error de Compilación]",
        ClsError::RuntimeError(_) => "[Runtime Error]",
        ClsError::TypeError(_) => "[Error de Tipo]",
        ClsError::CompileError(_) => "[Error de Compilación]",
        _ => "",
    }
}

/// ¿Es un error de sintaxis (vs runtime/compilación)?
fn is_syntax(report: &ErrorReport) -> bool {
    matches!(
        report.error,
        ClsError::SyntaxError(_) | ClsError::SyntaxErrorAt(_, _)
    )
}

/// ¿Es un error de compilación (no sintaxis ni runtime)?
fn is_compile(report: &ErrorReport) -> bool {
    matches!(report.error, ClsError::CompileError(_) | ClsError::CompileErrorAt(_, _))
}

/// Encabezado del reporte según el tipo de error.
fn report_header(report: &ErrorReport) -> String {
    if is_syntax(report) {
        format!("Error en '{}':", report.source_file)
    } else if is_compile(report) {
        "Error de Compilación:".to_string()
    } else {
        "Error de ejecución:".to_string()
    }
}

// ─── Limpieza de mensajes ────────────────────────────────────────────────────

/// Quita el prefijo "Error de X: " y el "Call stack:" embebido
pub fn clean_error_msg(msg: &str) -> String {
    let msg = if let Some(pos) = msg.find("\n  Call stack:") {
        &msg[..pos]
    } else {
        msg
    };
    if let Some(pos) = msg.find(": ") {
        let rest = &msg[pos + 2..];
        if rest.starts_with("Error") || rest.starts_with("error") {
            clean_error_msg(rest)
        } else {
            rest.to_string()
        }
    } else {
        msg.to_string()
    }
}

// ─── Traza estructurada ──────────────────────────────────────────────────────

enum EntryKind {
    Import,
    Call,
    Error,
}

/// Una entrada de la traza (import, llamada o el error mismo).
struct TraceEntry {
    num: usize,
    kind: EntryKind,
    function: Option<String>,
    file: String,
    line: u32,
    col: u32,
    label: Option<&'static str>,
    code: Option<String>,
}

impl TraceEntry {
    fn location(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.col)
    }
}

/// Recolecta la traza completa (import_trace + call stack + error) como entradas.
fn collect_trace(report: &ErrorReport) -> Vec<TraceEntry> {
    let mut entries = Vec::new();
    let mut num = 1;

    for frame in &report.import_trace {
        let code = read_line(&frame.source_file, None, frame.line as usize);
        entries.push(TraceEntry {
            num,
            kind: EntryKind::Import,
            function: None,
            file: frame.source_file.clone(),
            line: frame.line,
            col: frame.col,
            label: None,
            code,
        });
        num += 1;
    }

    for frame in &report.stack {
        match &frame.span {
            Some(s) => {
                let code = if s.start_line == 0 {
                    None
                } else {
                    read_line(&frame.source_file, None, s.start_line as usize)
                };
                entries.push(TraceEntry {
                    num,
                    kind: EntryKind::Call,
                    function: Some(frame.function.clone()),
                    file: frame.source_file.clone(),
                    line: s.start_line,
                    col: s.start_col,
                    label: None,
                    code,
                });
            }
            None => {
                entries.push(TraceEntry {
                    num,
                    kind: EntryKind::Call,
                    function: Some(frame.function.clone()),
                    file: frame.source_file.clone(),
                    line: 0,
                    col: 0,
                    label: None,
                    code: None,
                });
            }
        }
        num += 1;
    }

    if let Some(span) = &report.span {
        let code = read_line(&report.source_file, report.source.as_deref(), span.start_line as usize);
        entries.push(TraceEntry {
            num,
            kind: EntryKind::Error,
            function: None,
            file: report.source_file.clone(),
            line: span.start_line,
            col: span.start_col,
            label: Some(error_label(&report.error)),
            code,
        });
    }

    entries
}

/// Lee la línea `line` del source (del contenido en memoria o del archivo).
fn read_line(source_file: &str, source: Option<&str>, line: usize) -> Option<String> {
    if line == 0 {
        return None;
    }
    let text = match source {
        Some(s) => Some(s.to_string()),
        None => std::fs::read_to_string(source_file).ok(),
    }?;
    text.lines().nth(line.saturating_sub(1)).map(|l| l.to_string())
}

fn line_pad(line: u32) -> String {
    " ".repeat(line.to_string().len())
}

/// Construye el caret visual para una línea (tabulaciones -> 4 espacios).
fn caret_for(line_text: &str, col: u32) -> String {
    let col = col as usize;
    if col > 0 && col <= line_text.chars().count() {
        let prefix: String = line_text.chars().take(col.saturating_sub(1)).collect();
        let visual = prefix.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum::<usize>();
        " ".repeat(visual) + "^"
    } else {
        "^".to_string()
    }
}

// ─── Formatos ────────────────────────────────────────────────────────────────

/// Formato de salida de un reporte de error. Lo elige el NODO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFormat {
    /// Texto plano sin decoradores.
    Plain,
    /// Texto con códigos ANSI para consola (colores).
    Console,
    /// Texto HTML (string).
    Html,
    /// JSON estructurado (fallback / máquinas).
    Json,
}

/// Trait de formateo: cada formato produce un String.
pub trait ErrorFormatter {
    fn format(&self, report: &ErrorReport) -> String;
}

pub fn format_error(report: &ErrorReport, format: &ErrorFormat) -> String {
    match format {
        ErrorFormat::Plain => PlainFormatter.format(report),
        ErrorFormat::Console => ConsoleFormatter.format(report),
        ErrorFormat::Html => HtmlFormatter.format(report),
        ErrorFormat::Json => JsonFormatter.format(report),
    }
}

// ── Plain ────────────────────────────────────────────────────────────────────

pub struct PlainFormatter;

impl ErrorFormatter for PlainFormatter {
    fn format(&self, report: &ErrorReport) -> String {
        let mut out = String::new();
        out.push_str(&report_header(report));
        out.push('\n');
        if !is_syntax(report) {
            out.push('\n');
        }
        for e in collect_trace(report) {
            out.push_str(&entry_plain(&e));
        }
        out.push_str(&format!("  Error: {}\n", clean_error_msg(&report.error.to_string())));
        out
    }
}

fn entry_plain(e: &TraceEntry) -> String {
    let mut s = String::new();
    match e.kind {
        EntryKind::Import => s.push_str(&format!("{}. En {}\n", e.num, e.location())),
        EntryKind::Call => match &e.function {
            Some(f) if e.line == 0 => s.push_str(&format!("{}. -> {} ({})\n", e.num, f, e.file)),
            Some(f) => s.push_str(&format!("{}. En {} -> {}\n", e.num, e.location(), f)),
            None => s.push_str(&format!("{}. En {}\n", e.num, e.location())),
        },
        EntryKind::Error => {
            let label = e.label.unwrap_or("");
            s.push_str(&format!("{}. En {} {}\n", e.num, e.location(), label));
        }
    }
    if let Some(code) = &e.code {
        s.push_str(&format!("  {} | {}\n", e.line, code));
        s.push_str(&format!("  {} | {}\n", line_pad(e.line), caret_for(code, e.col)));
    }
    s
}

// ── Console (ANSI) ───────────────────────────────────────────────────────────

pub struct ConsoleFormatter;

impl ErrorFormatter for ConsoleFormatter {
    fn format(&self, report: &ErrorReport) -> String {
        let mut out = String::new();
        if is_syntax(report) {
            out.push_str(&ansi::fg(true, ansi::codes::BRIGHT_RED, &report_header(report)));
        } else {
            out.push_str(&ansi::fg(true, ansi::codes::BRIGHT_RED, &report_header(report)));
            out.push('\n');
        }
        out.push('\n');
        for e in collect_trace(report) {
            out.push_str(&entry_console(&e));
        }
        let desc = clean_error_msg(&report.error.to_string());
        out.push_str(&format!(
            "  {} {}\n",
            ansi::bold(true, ansi::fg(true, ansi::codes::BRIGHT_RED, "Error:")),
            desc
        ));
        out
    }
}

fn entry_console(e: &TraceEntry) -> String {
    let mut s = String::new();
    let num = ansi::fg(true, ansi::codes::CYAN, &e.num.to_string());
    match e.kind {
        EntryKind::Import => s.push_str(&format!("{}. En {}\n", num, e.location())),
        EntryKind::Call => match &e.function {
            Some(f) if e.line == 0 => {
                s.push_str(&format!("{}. {} {}\n", num, ansi::fg(true, ansi::codes::YELLOW, "->"), f));
            }
            Some(f) => {
                s.push_str(&format!("{}. En {} {} {}\n", num, e.location(), ansi::fg(true, ansi::codes::YELLOW, "->"), f));
            }
            None => s.push_str(&format!("{}. En {}\n", num, e.location())),
        },
        EntryKind::Error => {
            let label = e.label.unwrap_or("");
            s.push_str(&format!("{}. En {} {}\n", num, e.location(), ansi::fg(true, ansi::codes::BRIGHT_MAGENTA, label)));
        }
    }
    if let Some(code) = &e.code {
        s.push_str(&format!("  {} | {}\n", e.line, code));
        let caret = caret_for(code, e.col);
        let pad = line_pad(e.line);
        if matches!(e.kind, EntryKind::Error) {
            s.push_str(&format!("  {} | {}\n", pad, ansi::fg(true, ansi::codes::RED, &caret)));
        } else {
            s.push_str(&format!("  {} | {}\n", pad, ansi::fg(true, ansi::codes::GRAY, &caret)));
        }
    }
    s
}

// ── Html ─────────────────────────────────────────────────────────────────────

pub struct HtmlFormatter;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

impl ErrorFormatter for HtmlFormatter {
    fn format(&self, report: &ErrorReport) -> String {
        let mut out = String::new();
        out.push_str("<pre class=\"cls-error\">");
        out.push_str(&format!("<strong style=\"color:#e06c75\">{}</strong>\n", html_escape(&report_header(report))));
        for e in collect_trace(report) {
            out.push_str(&entry_html(&e));
        }
        let desc = html_escape(&clean_error_msg(&report.error.to_string()));
        out.push_str(&format!("  <strong style=\"color:#e06c75\">Error:</strong> {}\n", desc));
        out.push_str("</pre>");
        out
    }
}

fn entry_html(e: &TraceEntry) -> String {
    let mut s = String::new();
    match e.kind {
        EntryKind::Import => s.push_str(&format!("<span style=\"color:#56b6c2\">{}</span>. En {}<br/>", e.num, html_escape(&e.location()))),
        EntryKind::Call => match &e.function {
            Some(f) if e.line == 0 => s.push_str(&format!("<span style=\"color:#56b6c2\">{}</span>. -> {}<br/>", e.num, html_escape(f))),
            Some(f) => s.push_str(&format!("<span style=\"color:#56b6c2\">{}</span>. En {} -> {}<br/>", e.num, html_escape(&e.location()), html_escape(f))),
            None => s.push_str(&format!("<span style=\"color:#56b6c2\">{}</span>. En {}<br/>", e.num, html_escape(&e.location()))),
        },
        EntryKind::Error => s.push_str(&format!("<span style=\"color:#56b6c2\">{}</span>. En {} {}<br/>", e.num, html_escape(&e.location()), e.label.unwrap_or(""))),
    }
    if let Some(code) = &e.code {
        let caret = caret_for(code, e.col);
        s.push_str(&format!("&nbsp;{} | {}<br/>", e.line, html_escape(code)));
        let color = if matches!(e.kind, EntryKind::Error) { "#e06c75" } else { "#6c7086" };
        s.push_str(&format!("&nbsp;{} | <span style=\"color:{}\">{}</span><br/>", line_pad(e.line), color, caret));
    }
    s
}

// ── Json ─────────────────────────────────────────────────────────────────────

pub struct JsonFormatter;

impl ErrorFormatter for JsonFormatter {
    fn format(&self, report: &ErrorReport) -> String {
        let span = report.span.as_ref().map(|s| serde_json::json!({
            "line": s.start_line,
            "col": s.start_col,
            "end_line": s.end_line,
            "end_col": s.end_col,
        }));
        let stack: Vec<serde_json::Value> = report.stack.iter().map(|f| serde_json::json!({
            "function": f.function,
            "file": f.source_file,
            "span": f.span.as_ref().map(|s| serde_json::json!({
                "line": s.start_line,
                "col": s.start_col,
            })),
        })).collect();
        let imports: Vec<serde_json::Value> = report.import_trace.iter().map(|f| serde_json::json!({
            "module": f.module_name,
            "file": f.source_file,
            "line": f.line,
            "col": f.col,
        })).collect();
        let obj = serde_json::json!({
            "error": report.error.to_string(),
            "message": clean_error_msg(&report.error.to_string()),
            "file": report.source_file,
            "span": span,
            "stack": stack,
            "imports": imports,
        });
        serde_json::to_string_pretty(&obj).unwrap_or_else(|_| "{}".to_string())
    }
}

// ── Puntos de entrada (el NODO decide cómo imprimir) ─────────────────────────

/// Formatea un reporte de error de runtime en el formato pedido.
/// El nodo decide el formato y dónde imprimirlo.
pub fn format_runtime_error(report: &ErrorReport, format: &ErrorFormat) -> String {
    format_error(report, format)
}

/// Formatea un error de sintaxis. El nodo decide el formato.
pub fn format_syntax_error(error: ClsError, source: &str, source_file: &str, format: &ErrorFormat) -> String {
    let report = ErrorReport::from_syntax(error, source, source_file);
    format_error(&report, format)
}

// ── Compatibilidad: show_* imprimen por stderr en formato Console ────────────

pub fn show_runtime_error(report: &ErrorReport) {
    eprintln!("{}", format_error(report, &ErrorFormat::Console));
}

pub fn show_syntax_error(error: ClsError, source: &str, source_file: &str) {
    eprintln!("{}", format_error(&ErrorReport::from_syntax(error, source, source_file), &ErrorFormat::Console));
}

pub fn show_config_error(error: &ClsError) {
    eprintln!("Error de configuración: {}", error);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cls_core::error::{Span, StackFrame};

    fn syntax_report() -> ErrorReport {
        let span = Span::new(2, 5, 2, 10);
        let err = ClsError::syntax_at("esperaba X", &span);
        ErrorReport::from_syntax(err, "line1\nline2 foo\nline3", "test.clsx")
    }

    #[test]
    fn plain_has_line_and_caret() {
        let s = format_error(&syntax_report(), &ErrorFormat::Plain);
        assert!(s.contains("Error en 'test.clsx'"), "header: {}", s);
        assert!(s.contains("line2 foo"), "linea: {}", s);
        assert!(s.contains("^"), "caret: {}", s);
        assert!(s.contains("Error: esperaba X"));
    }

    #[test]
    fn plain_has_no_ansi() {
        let s = format_error(&syntax_report(), &ErrorFormat::Plain);
        assert!(!s.contains('\x1b'), "no debe tener ANSI");
    }

    #[test]
    fn console_has_ansi() {
        let s = format_error(&syntax_report(), &ErrorFormat::Console);
        assert!(s.contains('\x1b'), "debe tener ANSI");
    }

    #[test]
    fn html_wraps_pre() {
        let s = format_error(&syntax_report(), &ErrorFormat::Html);
        assert!(s.contains("<pre class=\"cls-error\">"));
        assert!(s.contains("</pre>"));
        assert!(s.contains("line2 foo"));
    }

    #[test]
    fn json_is_parseable() {
        let s = format_error(&syntax_report(), &ErrorFormat::Json);
        let v: serde_json::Value = serde_json::from_str(&s).expect("json valido");
        assert_eq!(v["message"], "esperaba X");
        assert_eq!(v["file"], "test.clsx");
    }

    #[test]
    fn runtime_report_header() {
        let err = ClsError::RuntimeError("boom".into());
        let rep = ErrorReport::from_runtime(err, vec![StackFrame::new("main", None, "t.clsx")], &[], "t.clsx");
        let s = format_error(&rep, &ErrorFormat::Plain);
        assert!(s.contains("Error de ejecución:"));
    }
}


