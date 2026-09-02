//! Layout de memoria y helpers de layout del backend WASM (Fase 1: extraido de wasm/mod.rs).

use crate::frontend::token::Operator;
use crate::middleware::types::{LitVal, Type};
use wasm_encoder::Instruction;

use super::types::WasTy;
pub(super) fn elem_size_bytes(w: WasTy) -> i64 {
    match w {
        WasTy::I64 | WasTy::F64 => 8,
        WasTy::I32 => 4,
    }
}
/// Aplica el operador compuesto a los dos valores del stack (según el tipo).
pub(super) fn apply_compound_ty(
    body: &mut Vec<Instruction>,
    op: Operator,
    ty: WasTy,
) -> Result<(), crate::error::ClsError> {
    let inst = match (op, ty) {
        (Operator::PlusEqual, WasTy::F64) => Instruction::F64Add,
        (Operator::MinusEqual, WasTy::F64) => Instruction::F64Sub,
        (Operator::StarEqual, WasTy::F64) => Instruction::F64Mul,
        (Operator::SlashEqual, WasTy::F64) => Instruction::F64Div,
        (Operator::PercentEqual, WasTy::F64) => {
            return Err(crate::error::ClsError::CompileError(
                "`%=` sobre un elemento float no soportado por el JIT (usa el identificador)".to_string(),
            ))
        }
        (Operator::PlusEqual, _) => Instruction::I64Add,
        (Operator::MinusEqual, _) => Instruction::I64Sub,
        (Operator::StarEqual, _) => Instruction::I64Mul,
        (Operator::SlashEqual, _) => Instruction::I64DivS,
        (Operator::PercentEqual, _) => Instruction::I64RemS,
        _ => {
            return Err(crate::error::ClsError::CompileError(
                "Operador compuesto no soportado por el JIT".to_string(),
            ))
        }
    };
    body.push(inst);
    Ok(())
}
/// Tag compuesto para valores del Cmx: `tipo<<8 | kind`. tipo: 0=int, 1=string,
/// 2=float, 3=bool, 4=char, 5=cmx, 6=array, 7=record. kind = tipo del elem (arrays).
pub(super) fn cmx_tag_for_type(t: &Type) -> i64 {
    match t {
        Type::String => 1 << 8,
        Type::Float | Type::F32 | Type::F64 => 2 << 8,
        Type::Bool => 3 << 8,
        Type::Char => 4 << 8,
        Type::Cmx => 5 << 8,
        Type::Array(e) => (6 << 8) | arr_kind_code(e),
        Type::Record(_, _) => 7 << 8,
        _ => 0,
    }
}
/// Código del tipo de elemento para `arr_join`/`arr_to_string`
/// (0=int, 1=string, 2=float, 3=bool, 4=char, 5=cmx, 6=array/tuple, 7=record).
/// Los contenedores (6/7) hacen que el formateador despache cada elemento por
/// su tag compuesto (`fmt_val_to_string(e, kind<<8)`), no como int crudo
/// (bug dev-2: `enums: [{...}]` imprimía punteros).
pub(super) fn arr_kind_code(t: &Type) -> i64 {
    match t {
        Type::String => 1,
        Type::Float | Type::F32 | Type::F64 => 2,
        Type::Bool => 3,
        Type::Char => 4,
        Type::Cmx => 5,
        Type::Array(_) | Type::Tuple(_) => 6,
        Type::Record(_, _) | Type::Shape(_) => 7,
        _ => 0,
    }
}
pub(super) fn is_compound(op: Operator) -> bool {
    matches!(
        op,
        Operator::PlusEqual
            | Operator::MinusEqual
            | Operator::StarEqual
            | Operator::SlashEqual
            | Operator::PercentEqual
    )
}
/// Tag del RUNTIME interno para valores dentro de records/arrays heterogéneos:
/// 0=int 1=string 2=float 3=bool 4=char 5=cmx 6=array 7=record (tabla de
/// `fmt_val_to_string`/`record_tag` del host). Distinto de `cls_kind_code`
/// (tabla del binding: 4=string 5=array 6=record).
pub(super) fn runtime_tag_code(t: &Type) -> i64 {
    match t {
        Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64
        | Type::Literal(LitVal::Int(_)) => 0,
        Type::String => 1,
        Type::Float | Type::F32 | Type::F64 | Type::Literal(LitVal::Float(_)) => 2,
        Type::Bool | Type::Literal(LitVal::Bool(_)) => 3,
        Type::Char => 4,
        Type::Cmx => 5,
        Type::Array(_) => 6,
        Type::Record(_, _) | Type::Shape(_) | Type::Tuple(_) => 7,
        Type::Null => 0,
        _ => 8,
    }
}

/// Tag COMPUESTO para guardar un valor en un record/array heterogéneo: para
/// arrays usa `6<<8 | arr_kind` (el formateador `fmt_val_to_string` interpreta
/// kind=6 como "array de Cmx" con es=16 — un tag plano 6 hace que lea ints a
/// saltos de 16 bytes -> basura). Las TUPLAS se guardan como array (layout
/// contiguo `[cap][len][slots]`, igual que un array) — tag 6<<8|0, NO 7 (record):
/// el formateador de records asume hashmap y una tupla no lo es (bug dev-2:
/// `enums: [{e: (1,2)}]` trapeaba al formatear la tupla como record).
/// Los demás valores usan `runtime_tag_code`.
pub(super) fn runtime_tag_code_compound(t: &Type) -> i64 {
    match t {
        Type::Array(e) => (6 << 8) | arr_kind_code(e),
        Type::Tuple(_) => 6 << 8,
        _ => runtime_tag_code(t),
    }
}
/// host usa para el marshalling de valores):
/// 0=int 1=float 2=bool 3=char 4=string 5=array 6=record/shape 7=tuple
/// 8=otro i64 (enum/struct/clase/named/union) 9=void 10=cmx 11=funcion 12=null.
pub(super) fn cls_kind_code(t: &Type) -> i64 {
    match t {
        Type::Int | Type::I8 | Type::I16 | Type::I32 | Type::I64
        | Type::Literal(LitVal::Int(_)) => 0,
        Type::Float | Type::F32 | Type::F64 | Type::Literal(LitVal::Float(_)) => 1,
        Type::Bool | Type::Literal(LitVal::Bool(_)) => 2,
        Type::Char => 3,
        Type::String => 4,
        Type::Array(_) => 5,
        Type::Record(_, _) | Type::Shape(_) => 6,
        Type::Tuple(_) => 7,
        Type::Void | Type::Empty => 9,
        Type::Cmx => 10,
        Type::Fun(..) => 11,
        Type::Null => 12,
        _ => 8,
    }
}
/// Base de la tabla de índices de strings (8 bytes por entrada: offset, len).
/// Layout de memoria:
///   `[STRING_DATA_BASE .. +data_len)`  = bytes de los strings (append-only)
///   `[STRING_TABLE_BASE .. +8N)`       = tabla: cada entrada (offset, len)
/// Con bases FIJAS, los offsets de los datos no dependen del tamaño total del
/// pool: el REPL JIT (estado persistente) transfiere punteros de strings entre
/// instancias y estos siguen siendo válidos.
pub const STRING_DATA_BASE: u32 = INTERNALS_WINDOW_END; // justo tras la ventana de internals
pub const STRING_TABLE_BASE: u32 = STRING_DATA_BASE + 512 * 1024; // 1.5625MB
pub const HEAP_START: u32 = STRING_TABLE_BASE + 256 * 1024; // 1.8125MB - tras la tabla

/// Sentinel de fin de iteración del protocolo `__next` (el `return null` de un
/// método `__next` se emite con este valor; un iterador puede devolver 0 como
/// valor legítimo, así que el null NO puede ser 0).
pub const NULL_ITER_SENTINEL: i64 = i64::MIN;

// ── Shadow call stack en memoria lineal (plan-performance/shadow-stack-wasm.md) ──
// Región fija FUERA del heap del usuario. El REPL (`transfer_state`) copia
// solo `[HEAP_START..]`, así que estas regiones NO se transfieren entre líneas.

/// Fin de la ventana de las internals WASM (el sub-crate compila a 17 páginas =
/// 1_114_112 bytes; vive en `[0..INTERNALS_WINDOW_END)` con sus direcciones
/// internas INTACTAS — data/bss/shadow stack del runtime de Rust). Todo el CLS
/// queda por ENCIMA de esta ventana para no colisionar (ver fusion.rs).
pub const INTERNALS_WINDOW_END: u32 = 1_114_112;

/// Base de la región de frames del shadow stack (tras el heap, en [2MB..)).
/// El heap del CLS arranca en HEAP_START; el shadow stack y sus slots quedan
/// por encima (el heap crece hacia arriba con memory.grow).
pub const SHADOW_STACK_BASE: u32 = HEAP_START + 128 * 1024; // HEAP_START + 128KB
/// Tamaño de cada frame (name_idx:u32 + line:u32 + col:u32). line/col en u32 y
/// NO en u16: los spans de los módulos importados se desplazan a `100000*n +
/// línea` y u16 los truncaría (perdían el offset del módulo -> trace 65535:N).
pub const FRAME_SIZE: u32 = 12;
/// Tope de frames (igual al límite de `call_stack` del host actual).
pub const SHADOW_MAX_FRAMES: u32 = 1000;
/// `shadow_ptr` alcanzó el tope → no escribir más frames.
pub const SHADOW_LIMIT: u32 = SHADOW_STACK_BASE + FRAME_SIZE * SHADOW_MAX_FRAMES;
/// Call site pendiente del llamador (line:u32, col:u32) — el `fn_enter` del
/// callee lo usa como span del frame (paridad con `pending_call_site` del host).
/// En u32 (no u16) por los spans desplazados de módulos importados (100000*n).
pub const PENDING_CALL_SLOT_ADDR: u32 = SHADOW_STACK_BASE - 8 * 1024;
/// Slot del reporte del error no capturado (futuro wrapper try_table).
#[allow(dead_code)]
pub const ERROR_SLOT_ADDR: u32 = SHADOW_STACK_BASE - 16 * 1024;
