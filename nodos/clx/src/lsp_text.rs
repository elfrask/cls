//! Conversiones de posiciones LSP (UTF-16) y utilidades de texto para el LSP.

use cls_core::error::Span;
use tower_lsp::lsp_types::{Position, Range};

/// Texto de una línea (0-based). Devuelve `None` si la línea no existe.
pub fn line_text(source: &str, line: u32) -> Option<&str> {
    source.lines().nth(line as usize)
}

/// Convierte una columna LSP (unidades UTF-16) al índice de `char` de la línea.
pub fn utf16_col_to_char_index(line: &str, character: u32) -> usize {
    let mut units = 0usize;
    for (idx, ch) in line.chars().enumerate() {
        if units >= character as usize {
            return idx;
        }
        units += ch.len_utf16();
    }
    line.chars().count()
}

/// Convierte un índice de `char` de la línea a columna LSP (UTF-16).
pub fn char_index_to_utf16_col(line: &str, char_index: usize) -> u32 {
    line.chars()
        .take(char_index)
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32
}

/// Posición LSP -> índice de byte en `source`.
pub fn position_to_byte_index(source: &str, position: Position) -> Option<usize> {
    let mut offset = 0usize;
    for (line_no, line) in source.split_inclusive('\n').enumerate() {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let line_without_cr = line_without_newline
            .strip_suffix('\r')
            .unwrap_or(line_without_newline);
        if line_no as u32 == position.line {
            let char_index = utf16_col_to_char_index(line_without_cr, position.character);
            let byte_in_line = line_without_cr
                .char_indices()
                .nth(char_index)
                .map(|(index, _)| index)
                .unwrap_or(line_without_cr.len());
            return Some(offset + byte_in_line);
        }
        offset += line.len();
    }
    // Última línea vacía cuando el archivo termina en `\n`.
    if position.line as usize == source.matches('\n').count() {
        return Some(source.len());
    }
    None
}

/// `Span` del parser (líneas/columnas 1-based, columnas en chars) -> `Range` LSP.
pub fn span_to_range(source: &str, span: &Span) -> Range {
    let start_line = span.start_line.saturating_sub(1);
    let end_line = span.end_line.saturating_sub(1);
    let start_text = line_text(source, start_line).unwrap_or("");
    let end_text = line_text(source, end_line).unwrap_or(start_text);

    let start_char = (span.start_col.saturating_sub(1)) as usize;
    let end_char = span.end_col.max(span.start_col) as usize;

    Range {
        start: Position {
            line: start_line,
            character: char_index_to_utf16_col(start_text, start_char),
        },
        end: Position {
            line: end_line,
            character: char_index_to_utf16_col(end_text, end_char),
        },
    }
}

/// Palabra (identificador simple) bajo la posición, con su rango UTF-16.
pub fn word_at(source: &str, position: Position) -> Option<(String, Range)> {
    let line = line_text(source, position.line)?;
    let chars: Vec<char> = line.chars().collect();
    let idx = utf16_col_to_char_index(line, position.character).min(chars.len());

    let mut start = idx;
    let mut end = idx;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    let word: String = chars[start..end].iter().collect();
    Some((
        word,
        Range {
            start: Position {
                line: position.line,
                character: char_index_to_utf16_col(line, start),
            },
            end: Position {
                line: position.line,
                character: char_index_to_utf16_col(line, end),
            },
        },
    ))
}

/// Detecta un acceso a miembro antes del cursor.
///
/// Devuelve `(segmentos_del_receptor, prefijo_parcial)` para casos como:
/// `math.` -> `(["math"], "")`
/// `obj.campo.` -> `(["obj", "campo"], "")`
/// `obj.campo.sq` -> `(["obj", "campo"], "sq")`
pub fn member_access_before_cursor(
    source: &str,
    position: Position,
) -> Option<(Vec<String>, String)> {
    let line = line_text(source, position.line)?;
    let idx = utf16_col_to_char_index(line, position.character).min(line.chars().count());
    let before: String = line.chars().take(idx).collect();

    let dot = before.rfind('.').map(|i| (i, 1));
    let double_colon = before.rfind("::").map(|i| (i, 2));
    let (separator, width) = match (dot, double_colon) {
        (Some(d), Some(c)) if c.0 > d.0 => (c.0, c.1),
        (Some(d), _) => (d.0, d.1),
        (None, Some(c)) => (c.0, c.1),
        (None, None) => return None,
    };

    let receiver = &before[..separator];
    let partial = &before[separator + width..];
    if !partial.chars().all(|c| is_ident_char(c)) {
        return None;
    }

    let chars: Vec<char> = receiver.chars().collect();
    let mut start = chars.len();
    while start > 0
        && (is_ident_char(chars[start - 1]) || chars[start - 1] == '.' || chars[start - 1] == ':')
    {
        start -= 1;
    }
    let chain: String = chars[start..].iter().collect();
    let segments: Vec<String> = chain
        .replace("::", ".")
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if segments.is_empty() {
        return None;
    }
    Some((segments, partial.to_string()))
}

pub fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convierte_columnas_utf16() {
        let line = "😀x";
        assert_eq!(utf16_col_to_char_index(line, 0), 0);
        assert_eq!(utf16_col_to_char_index(line, 2), 1);
        assert_eq!(char_index_to_utf16_col(line, 1), 2);
    }

    #[test]
    fn convierte_posicion_a_byte() {
        let source = "a😀b\n";
        assert_eq!(
            position_to_byte_index(
                source,
                Position {
                    line: 0,
                    character: 3,
                }
            ),
            Some(5)
        );
        assert_eq!(
            position_to_byte_index(
                source,
                Position {
                    line: 1,
                    character: 0,
                }
            ),
            Some(7)
        );
    }

    #[test]
    fn detecta_miembro_simple_y_encadenado() {
        let (segments, partial) = member_access_before_cursor(
            "math.",
            Position {
                line: 0,
                character: 5,
            },
        )
        .unwrap();
        assert_eq!(segments, vec!["math"]);
        assert_eq!(partial, "");

        let source = "obj.campo.sq";
        let (segments, partial) = member_access_before_cursor(
            source,
            Position {
                line: 0,
                character: source.len() as u32,
            },
        )
        .unwrap();
        assert_eq!(segments, vec!["obj", "campo"]);
        assert_eq!(partial, "sq");
    }

    #[test]
    fn detecta_namespace_con_doble_dos_puntos() {
        let (segments, partial) = member_access_before_cursor(
            "math::",
            Position {
                line: 0,
                character: 6,
            },
        )
        .unwrap();
        assert_eq!(segments, vec!["math"]);
        assert_eq!(partial, "");
    }
}
