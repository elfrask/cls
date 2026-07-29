use cls_core::error::{ClsError, Span, StackFrame};
use crate::ImportFrame;

/// Reporte de error con contexto completo para formateo centralizado.
pub struct ErrorReport {
    pub error: ClsError,
    pub span: Option<Span>,
    pub stack: Vec<StackFrame>,
    pub import_trace: Vec<ImportFrame>,
    pub source_file: String,
}

impl ErrorReport {
    pub fn from_runtime(
        error: ClsError,
        stack: Vec<StackFrame>,
        import_trace: &[ImportFrame],
        source_file: &str,
    ) -> Self {
        let span = ClsError::extract_line_col(&error.to_string()).map(|(l, c)| {
            Span::new(l as u32, c as u32, l as u32, c as u32)
        });
        Self { error, span, stack, import_trace: import_trace.to_vec(), source_file: source_file.to_string() }
    }

    pub fn from_syntax(error: ClsError, source_file: &str) -> Self {
        let span = ClsError::extract_line_col(&error.to_string()).map(|(l, c)| {
            Span::new(l as u32, c as u32, l as u32, c as u32)
        });
        Self { error, span, stack: vec![], import_trace: vec![], source_file: source_file.to_string() }
    }

    pub fn from_config(error: ClsError) -> Self {
        Self { error, span: None, stack: vec![], import_trace: vec![], source_file: String::new() }
    }
}

// ─── Helpers de línea/cursor ─────────────────────────────────────────────────

fn show_source_line(source: &str, line: usize, col: usize, pad: &str) {
    let line_text = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
    eprintln!("  {} | {}", line, line_text);
    let cursor = if col > 0 && col <= line_text.len() {
        let prefix = &line_text[..col.saturating_sub(1)];
        let visual_col = prefix.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum::<usize>();
        " ".repeat(visual_col) + "^"
    } else {
        "^".to_string()
    };
    eprintln!("  {} | {}", pad, cursor);
}

fn source_pad(_source: &str, line: usize) -> String {
    " ".repeat(line.to_string().len())
}

// ─── Formateo centralizado ───────────────────────────────────────────────────

pub fn show_runtime_error(report: &ErrorReport) {
    eprintln!("Error de ejecución:\n");

    // 1. Import trace (módulos cargados con su contexto)
    for (i, frame) in report.import_trace.iter().enumerate() {
        let num = i + 1;
        eprintln!("{}. En {}:{}:{}", num, frame.source_file, frame.line, frame.col);
        if let Ok(source) = std::fs::read_to_string(&frame.source_file) {
            let pad = source_pad(&source, frame.line as usize);
            show_source_line(&source, frame.line as usize, frame.col as usize, &pad);
        } else {
            eprintln!("  import '{}' desde {}:{}:{}", frame.module_name, frame.source_file, frame.line, frame.col);
        }
    }

    // 2. El error mismo (ubicación + contexto)
    let step = report.import_trace.len() + 1;
    if let Some(span) = &report.span {
        let label = match &report.error {
            ClsError::SyntaxError(_) => "[Error de Sintaxis]",
            ClsError::RuntimeError(_) => "[Runtime Error]",
            ClsError::TypeError(_) => "[Error de Tipo]",
            ClsError::CompileError(_) => "[Error de Compilación]",
            _ => "",
        };
        eprintln!("{}. En {}:{}:{} {}", step, report.source_file, span.start_line, span.start_col, label);
        if let Ok(source) = std::fs::read_to_string(&report.source_file) {
            let pad = source_pad(&source, span.start_line as usize);
            show_source_line(&source, span.start_line as usize, span.start_col as usize, &pad);
        }
    }

    // 3. Mensaje de error (limpio, sin prefijo "Error de runtime: ")
    let error_str = report.error.to_string();
    let desc = clean_error_msg(&error_str);
    eprintln!("  Error: {}", desc);
    eprintln!();
}

pub fn show_syntax_error(error: &ClsError, source: &str, source_file: &str) {
    eprintln!("Error en '{}':", source_file);
    let msg = error.to_string();
    if let Some((line, col)) = ClsError::extract_line_col(&msg) {
        let clean = strip_label_and_span(&msg);
        eprintln!("  {}", clean);
        if !source.is_empty() {
            let pad = source_pad(source, line);
            show_source_line(source, line, col, &pad);
        }
    } else {
        eprintln!("  {}", msg);
    }
}

pub fn show_config_error(error: &ClsError) {
    eprintln!("Error de configuración: {}", error);
}

// ─── Limpieza de mensajes ────────────────────────────────────────────────────

/// Quita el prefijo "Error de X: " pero conserva el "Call stack:" embebido
fn clean_error_msg(msg: &str) -> String {
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

/// Quita prefijo y "(línea N, columna M)" del final
fn strip_label_and_span(msg: &str) -> String {
    let without_label = clean_error_msg(msg);
    if let Some(pos) = without_label.rfind(" (línea ") {
        without_label[..pos].to_string()
    } else {
        without_label.to_string()
    }
}
