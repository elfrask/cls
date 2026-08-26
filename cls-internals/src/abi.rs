//! ABI interno: firma WASM de cada función `__intr_<area>_<op>`.
//!
//! Los tipos y layouts matchean los del backend (`cls-core/src/backend/wasm/`):
//! - enteros/strings/arrays/records: `i64`
//! - floats: `f64`
//! - bools: `i32` (0/1)
//! - strings empaquetadas: `(ptr<<32)|len`
//! - arrays: `[cap:i64][len:i64][elems*es]` (es = 4 o 8)
//! - records: `[cap:i64][len:i64][(key:packed,val:i64,tag:i64)*24]`

use wasm_encoder::ValType;

/// Firma de una función interna.
pub struct InternalsFn {
    pub name: &'static str,
    pub params: &'static [ValType],
    pub results: &'static [ValType],
}

const I64: ValType = ValType::I64;
const I32: ValType = ValType::I32;
const F64: ValType = ValType::F64;

/// Catálogo de funciones internas (mismo ABI que los `HostFn` actuales que
/// reemplazan — ver `cls-core/src/backend/wasm/host_fn.rs`).
pub static INTERNALS_FUNCTIONS: &[InternalsFn] = &[
    // ── arrays ────────────────────────────────────────────────────────────
    InternalsFn { name: "__intr_arr_push", params: &[I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_pop", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_shift", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_unshift", params: &[I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_reverse", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_index_of", params: &[I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_includes", params: &[I64, I64, I64], results: &[I32] },
    InternalsFn { name: "__intr_arr_join", params: &[I64, I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_to_string", params: &[I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_arr_realloc", params: &[I64, I64, I64], results: &[I64] },
    // ── strings ───────────────────────────────────────────────────────────
    InternalsFn { name: "__intr_str_concat", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_str_int", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_str_float", params: &[F64], results: &[I64] },
    InternalsFn { name: "__intr_str_bool", params: &[I32], results: &[I64] },
    InternalsFn { name: "__intr_str_char", params: &[I32], results: &[I64] },
    InternalsFn { name: "__intr_str_upper", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_str_lower", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_str_trim", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_str_contains", params: &[I64, I64], results: &[I32] },
    InternalsFn { name: "__intr_str_starts_with", params: &[I64, I64], results: &[I32] },
    InternalsFn { name: "__intr_str_ends_with", params: &[I64, I64], results: &[I32] },
    InternalsFn { name: "__intr_str_is_empty", params: &[I64], results: &[I32] },
    InternalsFn { name: "__intr_str_repr", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_str_length", params: &[I64], results: &[I64] },
    // Módulo strings (utilidades de parseo por bytes): indexOf, slice, split.
    // Paridad con los hosts `str_index_of`/`str_slice`/`str_split`.
    InternalsFn { name: "__intr_str_index_of", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_str_slice", params: &[I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_str_split", params: &[I64, I64], results: &[I64] },
    // Append in-place (dev-2): concat con slack + append que escribe in-place
    // mientras haya capacidad (header mágico en ptr-8).
    InternalsFn { name: "__intr_str_concat_slack", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_str_append", params: &[I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_str_eq", params: &[I64, I64], results: &[I32] },
    InternalsFn { name: "__intr_any_to_string", params: &[I64, I64], results: &[I64] },
    // ── records ───────────────────────────────────────────────────────────
    InternalsFn { name: "__intr_record_new", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_record_set", params: &[I64, I64, I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_record_get", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_record_has", params: &[I64, I64], results: &[I32] },
    InternalsFn { name: "__intr_record_tag", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_record_len", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_record_keys", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_record_values", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_record_to_string", params: &[I64], results: &[I64] },
    // ── math ──────────────────────────────────────────────────────────────
    InternalsFn { name: "__intr_math_sqrt", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_pow", params: &[F64, F64], results: &[F64] },
    InternalsFn { name: "__intr_math_min", params: &[F64, F64], results: &[F64] },
    InternalsFn { name: "__intr_math_max", params: &[F64, F64], results: &[F64] },
    InternalsFn { name: "__intr_math_floor", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_ceil", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_round", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_sin", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_cos", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_tan", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_log", params: &[F64], results: &[F64] },
    InternalsFn { name: "__intr_math_fmod", params: &[F64, F64], results: &[F64] },
    InternalsFn { name: "__intr_pow_num", params: &[I64, I64], results: &[I64] },
    InternalsFn { name: "__intr_math_range", params: &[I64, I64], results: &[I64] },
    // abs entero: i64.abs no existe como instrucción WASM (el float es inline F64Abs).
    InternalsFn { name: "__intr_int_abs", params: &[I64], results: &[I64] },
    // ── conversiones / intrinsics puros ────────────────────────────────────
    InternalsFn { name: "__intr_parse_int", params: &[I64], results: &[I64] },
    InternalsFn { name: "__intr_parse_float", params: &[I64], results: &[F64] },
    InternalsFn { name: "__intr_parse_bool", params: &[I64], results: &[I32] },
    // Flag de error del último parse (0 = ok, 1 = falló).
    InternalsFn { name: "__intr_parse_error_get", params: &[], results: &[I32] },
];
