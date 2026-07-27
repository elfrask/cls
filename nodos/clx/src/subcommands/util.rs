use cls_runtime::ImportFrame;
use std::fs;

pub fn show_error(source: &str, error_msg: &str, path: &str) {
    eprintln!("Error en '{}':", path);
    eprintln!("  {}", error_msg);
    if source.is_empty() { return; }
    let line_col = error_msg.split("línea").nth(1).and_then(|s| {
        let parts: Vec<&str> = s.splitn(2, ',').collect();
        let line = parts.first()?.trim().parse::<usize>().ok()?;
        let col = parts.get(1).and_then(|c|
            c.split("columna").nth(1).and_then(|c2|
                c2.trim().trim_matches(|p| p == ')' || p == '(').parse::<usize>().ok()
            )
        ).unwrap_or(1);
        Some((line, col))
    });
    if let Some((line, col)) = line_col {
        if let Some(src_line) = source.lines().nth(line.saturating_sub(1)) {
            eprintln!("");
            eprintln!("  {} | {}", line, src_line);
            let pad = " ".repeat(line.to_string().len());
            if col > 1 {
                eprintln!("  {} | {}{}", pad, " ".repeat(col.saturating_sub(1) as usize), "^");
            } else {
                eprintln!("  {} | ^", pad);
            }
        }
    }
}

pub fn show_runtime_error(error_msg: &str, trace: &[ImportFrame], source_file: &str) {
    let file_hint = if error_msg.contains("Error en '") {
        error_msg.split('\'').nth(1).map(|s| s.trim())
    } else { None };

    let error_desc = match error_msg.rfind(": Error de ") {
        Some(pos) => &error_msg[pos + 2..],
        None => error_msg,
    };

    let error_line_col = error_msg.split("línea").nth(1).and_then(|s| {
        let end = s.find(')').unwrap_or(s.len());
        let inner = &s[..end];
        let parts: Vec<&str> = inner.splitn(2, ',').collect();
        let line = parts.first()?.trim().parse::<usize>().ok()?;
        let col = parts.get(1).and_then(|c|
            c.split("columna").nth(1).and_then(|c2| c2.trim().parse::<usize>().ok())
        ).unwrap_or(1);
        Some((line, col))
    });

    let src_file = if let Some(module) = file_hint {
        format!("{}.clsx", module)
    } else {
        source_file.to_string()
    };

    if file_hint.is_some() {
        eprintln!("Error al importar módulo '{}':\n", file_hint.unwrap());
    } else {
        eprintln!("Error de ejecución:\n");
    }

    for (i, frame) in trace.iter().enumerate() {
        let num = i + 1;
        let src = frame.source_file.clone();
        if let Ok(s) = fs::read_to_string(&src) {
            let line_txt = s.lines().nth((frame.line.saturating_sub(1)) as usize).unwrap_or("");
            eprintln!("{}. En {}:{}:{}", num, src, frame.line, frame.col);
            eprintln!("  {} | {}", frame.line, line_txt);
            let pad = " ".repeat(frame.line.to_string().len());
            eprintln!("  {} | {}^^^^^^", pad, " ".repeat(frame.col.saturating_sub(1) as usize));
        } else {
            eprintln!("{}. import '{}' desde {}:{}:{}", num, frame.module_name, src, frame.line, frame.col);
        }
    }

    let step = trace.len() + 1;
    if let Ok(source) = fs::read_to_string(&src_file) {
        if let Some((line, col)) = error_line_col {
            if let Some(src_line) = source.lines().nth(line.saturating_sub(1)) {
                let label = if file_hint.is_some() { "[Sintaxis Inválida]" } else { "[Runtime Error]" };
                eprintln!("{}. En {}:{}:{} {}", step, src_file, line, col, label);
                eprintln!("  {} | {}", line, src_line);
                let pad = " ".repeat(line.to_string().len());
                eprintln!("  {} | {}{}", pad, " ".repeat(col.saturating_sub(1) as usize), "^");
            }
        }
    }
    eprintln!("  Error: {}", error_desc.trim());
}
