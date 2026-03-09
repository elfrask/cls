// --- SIMBOLOS ---
// solo son delimitadores
pub const SIMBOLOS: &[&str] = &["(", ")", "[", "]", "{", "}", ",", "."];

// --- Delimitadores y espacios ---
pub const DELIMITADORES: &[&str] = &["\n", "\t", "\r", " "];


pub const DELIMITADORES_STRINGS: &[&str] = &["'", "\"", "`"];



// --- OPERADORES (Simples) ---
// Operadores de un solo carácter
pub const OPERADORES: &[&str] = &[
  "+", "-", "*", "/", "%", "=", "<", ">", "!", "&", "|", "^", "~", "?", "¿", ":", "@", "#",
];

// --- COMPUESTOS ---
// Operadores y símbolos de 2 o más caracteres.
// NOTA: El Lexer siempre debe buscar estos PRIMERO para evitar
// falsos positivos con los operadores simples.
pub const COMPUESTOS: &[&str] = &[
  "==", "!=", ">=", "<=", "&&", "||", "++", "--", "**", "+=", "-=", "*=", "/=", "=>", "->", "::",
  "..", "<<", ">>", "//", "/*"
];
