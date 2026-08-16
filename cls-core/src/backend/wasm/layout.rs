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
/// (0=int, 1=string, 2=float, 3=bool, 4=char, 5=cmx).
pub(super) fn arr_kind_code(t: &Type) -> i64 {
    match t {
        Type::String => 1,
        Type::Float | Type::F32 | Type::F64 => 2,
        Type::Bool => 3,
        Type::Char => 4,
        Type::Cmx => 5,
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
/// Layout de memoria: `[0 .. data_len)` = bytes de los strings (append-only,
/// en orden de interning) y `[STRING_TABLE_BASE .. + 8N)` = tabla. Con base
/// FIJA, los offsets de los datos no dependen del tamaño total del pool: el
/// REPL JIT (estado persistente) transfiere punteros de strings entre
/// instancias y estos siguen siendo válidos (las entradas compartidas
/// conservan su posición si las nuevas se agregan al final).
pub(super) const STRING_TABLE_BASE: u32 = 524_288; // 512KB - por debajo del heap (1MB)

/// Sentinel de fin de iteración del protocolo `__next` (el `return null` de un
/// método `__next` se emite con este valor; un iterador puede devolver 0 como
/// valor legítimo, así que el null NO puede ser 0).
pub(super) const NULL_ITER_SENTINEL: i64 = i64::MIN;

// ── Shadow call stack en memoria lineal (plan-performance/shadow-stack-wasm.md) ──
// Región fija FUERA del heap del usuario (HEAP_START = 1 MB). Todo queda por
// debajo de 1 MB: data (strings) en [0..512KB+8N], slots de error/call-site y
// frames del shadow stack por debajo del heap. El REPL (`transfer_state`) copia
// solo `[HEAP_START..]`, así que estas regiones NO se transfieren entre líneas.

/// Base de la región de frames del shadow stack (768 KB).
pub(super) const SHADOW_STACK_BASE: u32 = 786_432;
/// Tamaño de cada frame (name_idx:u32 + line:u32 + col:u32). line/col en u32 y
/// NO en u16: los spans de los módulos importados se desplazan a `100000*n +
/// línea` y u16 los truncaría (perdían el offset del módulo -> trace 65535:N).
pub(super) const FRAME_SIZE: u32 = 12;
/// Tope de frames (igual al límite de `call_stack` del host actual).
pub(super) const SHADOW_MAX_FRAMES: u32 = 1000;
/// `shadow_ptr` alcanzó el tope → no escribir más frames.
pub(super) const SHADOW_LIMIT: u32 = SHADOW_STACK_BASE + FRAME_SIZE * SHADOW_MAX_FRAMES;
/// Call site pendiente del llamador (line:u32, col:u32) — el `fn_enter` del
/// callee lo usa como span del frame (paridad con `pending_call_site` del host).
/// En u32 (no u16) por los spans desplazados de módulos importados (100000*n).
pub(super) const PENDING_CALL_SLOT_ADDR: u32 = 778_240; // 760 KB
/// Slot del reporte del error no capturado (futuro wrapper try_table).
#[allow(dead_code)]
pub(super) const ERROR_SLOT_ADDR: u32 = 770_048; // 752 KB
