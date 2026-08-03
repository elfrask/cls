//! Colores ANSI centralizados para la salida de consola.
//!
//! Los nodos pueden usar estos códigos para decorar texto (errores, logs).
//! El runtime los usa para el formato `Console` de los reportes de error.

/// Códigos ANSI (escapes) comunes. `use self::*` para acceder directamente.
pub mod codes {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const UNDERLINE: &str = "\x1b[4m";

    // Foreground
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
    pub const GRAY: &str = "\x1b[90m";

    // Bright foreground
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
    pub const BRIGHT_WHITE: &str = "\x1b[97m";
}

/// Envuelve `text` con un prefijo/sufijo ANSI (si `enabled`).
pub fn paint<T: AsRef<str>>(enabled: bool, prefix: &str, suffix: &str, text: T) -> String {
    if enabled {
        format!("{}{}{}", prefix, text.as_ref(), suffix)
    } else {
        text.as_ref().to_string()
    }
}

/// Envuelve `text` en un color de foreground (si `enabled`).
pub fn fg<T: AsRef<str>>(enabled: bool, color: &str, text: T) -> String {
    paint(enabled, color, codes::RESET, text)
}

/// Envuelve `text` en negrita (si `enabled`).
pub fn bold<T: AsRef<str>>(enabled: bool, text: T) -> String {
    paint(enabled, codes::BOLD, codes::RESET, text)
}
