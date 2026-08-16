//! Runtime wasmi (intérprete WASM puro, para wasm32/navegador).
//!
//! Implementa [`HostCtx`] sobre `wasmi::Caller` y registra las mismas host
//! functions (adaptadores de una línea a los cuerpos genéricos de `crate::host`).
//!
//! Limitación conocida: wasmi NO implementa la propuesta de exception-handling
//! (try_table/tag). El backend emite en modo sin excepciones (`WasmBackend::without_exceptions`):
//! los errores de runtime (div 0, índice) son traps `unreachable` y los
//! `try/catch`/`throw` fallan en compilación con error claro.

#![cfg(feature = "wasmi-runtime")]

use cls_core::frontend::ast::Module as ClsModule;
use std::sync::Arc;
use std::time::Instant;
use wasmi::{Caller, Engine, Linker, Memory, Module, Store, Val};

use crate::engine::{finish_run_error, module_offsets};
use crate::host::{self, HostCtx};
use crate::state::HostState;
use crate::timing::tick;
use crate::JitContext;

pub(crate) const HOST: &str = "env";

// ── HostCtx para wasmi ──────────────────────────────────────────────────────

impl HostCtx for Caller<'_, HostState> {
    fn state(&self) -> &HostState {
        self.data()
    }

    fn state_mut(&mut self) -> &mut HostState {
        self.data_mut()
    }

    fn read_str(&mut self, packed: i64) -> String {
        let ptr = (packed >> 32) as usize;
        let len = (packed & 0xffff_ffff) as usize;
        if let Some(memory) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = memory.data(self);
            if ptr + len <= data.len() {
                return String::from_utf8_lossy(&data[ptr..ptr + len]).into_owned();
            }
        }
        String::new()
    }

    fn write_str(&mut self, s: &str) -> i64 {
        let len = s.len() as i64;
        let cap = (len * 2 + 16).max(64);
        let ptr = self.alloc(cap);
        if ptr == 0 {
            return 0;
        }
        self.write_bytes(ptr as usize, s.as_bytes());
        self.data_mut().string_caps.insert(ptr, cap);
        (ptr << 32) | len
    }

    fn alloc(&mut self, n: i64) -> i64 {
        let func = self.get_export("alloc").and_then(|e| e.into_func());
        let func = match func {
            Some(f) => f,
            None => return 0,
        };
        let mut results = [Val::I64(0)];
        if let Err(e) = func.call(&mut *self, &[Val::I64(n)], &mut results) {
            eprintln!("Error de ejecución: memoria insuficiente (out of memory) al alocar {} bytes: {}", n, e);
            std::process::exit(1);
        }
        let ptr = results[0].i64().unwrap_or(0);
        if ptr == 0 {
            eprintln!("Error de ejecución: memoria insuficiente (out of memory) al alocar {} bytes", n);
            std::process::exit(1);
        }
        ptr
    }

    fn read_i64(&mut self, addr: usize) -> i64 {
        if let Some(mem) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(self);
            if addr + 8 <= data.len() {
                return i64::from_le_bytes(data[addr..addr + 8].try_into().unwrap());
            }
        }
        0
    }

    fn write_i64(&mut self, addr: usize, v: i64) {
        if let Some(mem) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data_mut(&mut *self);
            if addr + 8 <= data.len() {
                data[addr..addr + 8].copy_from_slice(&v.to_le_bytes());
            }
        }
    }

    fn read_i32(&mut self, addr: usize) -> i32 {
        if let Some(mem) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data(self);
            if addr + 4 <= data.len() {
                return i32::from_le_bytes(data[addr..addr + 4].try_into().unwrap());
            }
        }
        0
    }

    fn write_i32(&mut self, addr: usize, v: i32) {
        if let Some(mem) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data_mut(&mut *self);
            if addr + 4 <= data.len() {
                data[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
    }

    fn write_bytes(&mut self, addr: usize, bytes: &[u8]) -> bool {
        if let Some(mem) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data_mut(&mut *self);
            if addr + bytes.len() <= data.len() {
                data[addr..addr + bytes.len()].copy_from_slice(bytes);
                return true;
            }
        }
        false
    }
}

// ── Registro de host functions (adaptadores) ────────────────────────────────

/// Registra las host functions `env.*` (adaptadores). Público para que el nodo
/// de bindings (`clxb`) construya su propio Linker.
pub fn register_host_functions(linker: &mut Linker<HostState>) -> Result<(), String> {
    macro_rules! w {
        ($name:literal, $f:expr) => {
            linker.func_wrap(HOST, $name, $f).map_err(|e| e.to_string())?;
        };
    }
    w!("print_int", |mut c: Caller<'_, HostState>, v: i64| host::host_print_int(&mut c, v));
    w!("print_float", |mut c: Caller<'_, HostState>, v: f64| host::host_print_float(&mut c, v));
    w!("print_bool", |mut c: Caller<'_, HostState>, v: i32| host::host_print_bool(&mut c, v));
    w!("print_char", |mut c: Caller<'_, HostState>, v: i32| host::host_print_char(&mut c, v));
    w!("print_str", |mut c: Caller<'_, HostState>, v: i64| host::host_print_str(&mut c, v));
    w!("print_end", |mut c: Caller<'_, HostState>| host::host_print_end(&mut c));
    w!("print_any", |mut c: Caller<'_, HostState>, v: i64, t: i64| host::host_print_any(&mut c, v, t));
    w!("now", |mut c: Caller<'_, HostState>| -> i64 { host::host_now(&mut c) });
    w!("exit", |mut c: Caller<'_, HostState>, code: i64| host::host_exit(&mut c, code));
    w!("sleep", |mut c: Caller<'_, HostState>, ms: i64| host::host_sleep(&mut c, ms));
    w!("trap", |mut c: Caller<'_, HostState>, m: i64, s: i64| host::host_trap(&mut c, m, s));
    w!("parse_int", |mut c: Caller<'_, HostState>, v: i64| -> Result<i64, wasmi::Error> {
        host::host_parse_int(&mut c, v).map_err(wasmi::Error::new)
    });
    w!("parse_float", |mut c: Caller<'_, HostState>, v: i64| -> Result<f64, wasmi::Error> {
        host::host_parse_float(&mut c, v).map_err(wasmi::Error::new)
    });
    w!("parse_bool", |mut c: Caller<'_, HostState>, v: i64| host::host_parse_bool(&mut c, v));
    w!("str_concat", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
        host::host_str_concat(&mut c, a, b)
    });
    w!("str_int", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_str_int(&mut c, v) });
    w!("str_float", |mut c: Caller<'_, HostState>, v: f64| -> i64 { host::host_str_float(&mut c, v) });
    w!("str_bool", |mut c: Caller<'_, HostState>, v: i32| -> i64 { host::host_str_bool(&mut c, v) });
    w!("str_char", |mut c: Caller<'_, HostState>, v: i32| -> i64 { host::host_str_char(&mut c, v) });
    w!("str_upper", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_str_upper(&mut c, v) });
    w!("str_lower", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_str_lower(&mut c, v) });
    w!("str_trim", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_str_trim(&mut c, v) });
    w!("str_contains", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i32 {
        host::host_str_contains(&mut c, a, b)
    });
    w!("str_starts_with", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i32 {
        host::host_str_starts_with(&mut c, a, b)
    });
    w!("str_ends_with", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i32 {
        host::host_str_ends_with(&mut c, a, b)
    });
    w!("str_is_empty", |mut c: Caller<'_, HostState>, v: i64| -> i32 {
        host::host_str_is_empty(&mut c, v)
    });
    w!("str_repr", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_str_repr(&mut c, v) });
    w!("str_length", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_str_length(&mut c, v) });
    w!("int_abs", |mut c: Caller<'_, HostState>, v: i64| -> i64 { host::host_int_abs(&mut c, v) });
    w!("float_abs", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_float_abs(&mut c, v) });
    w!("pow_num", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i64 { host::host_pow_num(&mut c, a, b) });
    w!("fmod", |mut c: Caller<'_, HostState>, a: f64, b: f64| -> f64 { host::host_fmod(&mut c, a, b) });
    w!("input", |mut c: Caller<'_, HostState>| -> i64 { host::host_input(&mut c) });
    w!("arr_push", |mut c: Caller<'_, HostState>, p: i64, v: i64, e: i64| -> i64 {
        host::host_arr_push(&mut c, p, v, e)
    });
    w!("arr_pop", |mut c: Caller<'_, HostState>, p: i64, e: i64| -> i64 {
        host::host_arr_pop(&mut c, p, e)
    });
    w!("arr_shift", |mut c: Caller<'_, HostState>, p: i64, e: i64| -> i64 {
        host::host_arr_shift(&mut c, p, e)
    });
    w!("arr_unshift", |mut c: Caller<'_, HostState>, p: i64, v: i64, e: i64| -> i64 {
        host::host_arr_unshift(&mut c, p, v, e)
    });
    w!("arr_reverse", |mut c: Caller<'_, HostState>, p: i64, e: i64| -> i64 {
        host::host_arr_reverse(&mut c, p, e)
    });
    w!("arr_to_string", |mut c: Caller<'_, HostState>, p: i64, e: i64, k: i64| -> i64 {
        host::host_arr_to_string(&mut c, p, e, k)
    });
    w!("arr_index_of", |mut c: Caller<'_, HostState>, p: i64, n: i64, e: i64| -> i64 {
        host::host_arr_index_of(&mut c, p, n, e)
    });
    w!("arr_includes", |mut c: Caller<'_, HostState>, p: i64, n: i64, e: i64| -> i32 {
        host::host_arr_includes(&mut c, p, n, e)
    });
    w!("arr_join", |mut c: Caller<'_, HostState>, p: i64, s: i64, e: i64, k: i64| -> i64 {
        host::host_arr_join(&mut c, p, s, e, k)
    });
    w!("math_sqrt", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_sqrt(&mut c, v) });
    w!("math_pow", |mut c: Caller<'_, HostState>, a: f64, b: f64| -> f64 { host::host_math_pow(&mut c, a, b) });
    w!("math_min", |mut c: Caller<'_, HostState>, a: f64, b: f64| -> f64 { host::host_math_min(&mut c, a, b) });
    w!("math_max", |mut c: Caller<'_, HostState>, a: f64, b: f64| -> f64 { host::host_math_max(&mut c, a, b) });
    w!("math_floor", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_floor(&mut c, v) });
    w!("math_ceil", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_ceil(&mut c, v) });
    w!("math_round", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_round(&mut c, v) });
    w!("math_random", |mut c: Caller<'_, HostState>| -> f64 { host::host_math_random(&mut c) });
    w!("math_sin", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_sin(&mut c, v) });
    w!("math_cos", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_cos(&mut c, v) });
    w!("math_tan", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_tan(&mut c, v) });
    w!("math_log", |mut c: Caller<'_, HostState>, v: f64| -> f64 { host::host_math_log(&mut c, v) });
    w!("math_range", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
        host::host_math_range(&mut c, a, b)
    });
    w!("json_stringify", |mut c: Caller<'_, HostState>, v: i64, k: i64| -> i64 {
        host::host_json_stringify(&mut c, v, k)
    });
    w!("json_parse", |mut c: Caller<'_, HostState>, s: i64| -> i64 { host::host_json_parse(&mut c, s) });
    w!("fs_exists", |mut c: Caller<'_, HostState>, p: i64| -> i32 { host::host_fs_exists(&mut c, p) });
    w!("fs_cwd", |mut c: Caller<'_, HostState>| -> i64 { host::host_fs_cwd(&mut c) });
    w!("fs_read_file", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_fs_read_file(&mut c, p)
    });
    w!("fs_write_file", |mut c: Caller<'_, HostState>, p: i64, d: i64| -> i64 {
        host::host_fs_write_file(&mut c, p, d)
    });
    w!("fs_list_dir", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_fs_list_dir(&mut c, p)
    });
    w!("fs_mkdir", |mut c: Caller<'_, HostState>, p: i64| -> i64 { host::host_fs_mkdir(&mut c, p) });
    w!("fs_rm", |mut c: Caller<'_, HostState>, p: i64| -> i64 { host::host_fs_rm(&mut c, p) });
    w!("record_new", |mut c: Caller<'_, HostState>, cap: i64| -> i64 {
        host::host_record_new(&mut c, cap)
    });
    w!("record_set", |mut c: Caller<'_, HostState>, p: i64, k: i64, v: i64, t: i64| -> i64 {
        host::host_record_set(&mut c, p, k, v, t)
    });
    w!("record_get", |mut c: Caller<'_, HostState>, p: i64, k: i64| -> i64 {
        host::host_record_get(&mut c, p, k)
    });
    w!("record_has", |mut c: Caller<'_, HostState>, p: i64, k: i64| -> i32 {
        host::host_record_has(&mut c, p, k)
    });
    w!("record_tag", |mut c: Caller<'_, HostState>, p: i64, k: i64| -> i64 {
        host::host_record_tag(&mut c, p, k)
    });
    w!("record_len", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_record_len(&mut c, p)
    });
    w!("record_keys", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_record_keys(&mut c, p)
    });
    w!("record_values", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_record_values(&mut c, p)
    });
    w!("record_to_string", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_record_to_string(&mut c, p)
    });
    w!("any_member", |mut c: Caller<'_, HostState>, v: i64, t: i64, k: i64| -> (i64, i64) {
        host::host_any_member(&mut c, v, t, k)
    });
    w!("any_index", |mut c: Caller<'_, HostState>, v: i64, t: i64, i: i64| -> (i64, i64) {
        host::host_any_index(&mut c, v, t, i)
    });
    w!("http_get", |mut c: Caller<'_, HostState>, u: i64| -> i64 { host::host_http_get(&mut c, u) });
    w!("http_post", |mut c: Caller<'_, HostState>, u: i64, d: i64| -> i64 {
        host::host_http_post(&mut c, u, d)
    });
    w!("cmx_new", |mut c: Caller<'_, HostState>, t: i64, k: i64| -> i64 {
        host::host_cmx_new(&mut c, t, k)
    });
    w!("cmx_set_prop", |mut c: Caller<'_, HostState>, p: i64, k: i64, v: i64, t: i64| -> i64 {
        host::host_cmx_set_prop(&mut c, p, k, v, t)
    });
    w!("cmx_add_child", |mut c: Caller<'_, HostState>, p: i64, v: i64, t: i64| -> i64 {
        host::host_cmx_add_child(&mut c, p, v, t)
    });
    w!("cmx_to_string", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_cmx_to_string(&mut c, p)
    });
    w!("fn_handle", |mut c: Caller<'_, HostState>, ti: i64, n: i64, cap: i64| -> i64 {
        host::host_fn_handle(&mut c, ti, n, cap)
    });
    w!("fn_to_string", |mut c: Caller<'_, HostState>, h: i64| -> i64 {
        host::host_fn_to_string(&mut c, h)
    });
    w!("fn_enter", |mut c: Caller<'_, HostState>, n: i64, l: i64, col: i64| {
        host::host_fn_enter(&mut c, n, l, col)
    });
    w!("fn_exit", |mut c: Caller<'_, HostState>| host::host_fn_exit(&mut c));
    w!("fn_call_site", |mut c: Caller<'_, HostState>, l: i64, col: i64| {
        host::host_fn_call_site(&mut c, l, col)
    });
    w!("host_call", |mut c: Caller<'_, HostState>, id: i64, ptr: i64, n: i64| -> i64 {
        host::host_host_call(&mut c, id, ptr, n)
    });
    // Módulo os
    w!("os_platform", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_platform(&mut c) });
    w!("os_arch", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_arch(&mut c) });
    w!("os_version", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_version(&mut c) });
    w!("os_hostname", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_hostname(&mut c) });
    w!("os_home", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_home(&mut c) });
    w!("os_tempdir", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_tempdir(&mut c) });
    w!("os_cpus", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_cpus(&mut c) });
    w!("os_pid", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_pid(&mut c) });
    w!("os_uptime", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_uptime(&mut c) });
    w!("os_env", |mut c: Caller<'_, HostState>, k: i64| -> i64 { host::host_os_env(&mut c, k) });
    w!("os_sep", |mut c: Caller<'_, HostState>| -> i64 { host::host_os_sep(&mut c) });
    w!("os_is_windows", |mut c: Caller<'_, HostState>| -> i32 { host::host_os_is_windows(&mut c) });
    w!("os_is_unix", |mut c: Caller<'_, HostState>| -> i32 { host::host_os_is_unix(&mut c) });
    // Módulo path
    w!("path_join", |mut c: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
        host::host_path_join(&mut c, a, b)
    });
    w!("path_basename", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_path_basename(&mut c, p)
    });
    w!("path_dirname", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_path_dirname(&mut c, p)
    });
    w!("path_extname", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_path_extname(&mut c, p)
    });
    w!("path_resolve", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_path_resolve(&mut c, p)
    });
    w!("path_normalize", |mut c: Caller<'_, HostState>, p: i64| -> i64 {
        host::host_path_normalize(&mut c, p)
    });
    w!("path_is_absolute", |mut c: Caller<'_, HostState>, p: i64| -> i32 {
        host::host_path_is_absolute(&mut c, p)
    });
    w!("path_sep", |mut c: Caller<'_, HostState>| -> i64 { host::host_path_sep(&mut c) });
    // Módulo process
    w!("process_args", |mut c: Caller<'_, HostState>| -> i64 { host::host_process_args(&mut c) });
    w!("process_cwd", |mut c: Caller<'_, HostState>| -> i64 { host::host_process_cwd(&mut c) });
    w!("process_env", |mut c: Caller<'_, HostState>, k: i64| -> i64 {
        host::host_process_env(&mut c, k)
    });
    w!("process_exit", |mut c: Caller<'_, HostState>, code: i64| host::host_process_exit(&mut c, code));
    w!("process_pid", |mut c: Caller<'_, HostState>| -> i64 { host::host_process_pid(&mut c) });
    w!("process_platform", |mut c: Caller<'_, HostState>| -> i64 {
        host::host_process_platform(&mut c)
    });
    w!("process_title", |mut c: Caller<'_, HostState>| -> i64 { host::host_process_title(&mut c) });
    // Módulo time
    w!("time_now", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_now(&mut c) });
    w!("time_seconds", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_seconds(&mut c) });
    w!("time_iso", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_iso(&mut c) });
    w!("time_date", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_date(&mut c) });
    w!("time_clock", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_clock(&mut c) });
    w!("time_year", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_year(&mut c) });
    w!("time_month", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_month(&mut c) });
    w!("time_day", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_day(&mut c) });
    w!("time_hour", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_hour(&mut c) });
    w!("time_minute", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_minute(&mut c) });
    w!("time_second", |mut c: Caller<'_, HostState>| -> i64 { host::host_time_second(&mut c) });
    w!("time_sleep", |mut c: Caller<'_, HostState>, ms: i64| host::host_time_sleep(&mut c, ms));
    // Módulo random
    w!("random_random", |mut c: Caller<'_, HostState>| -> f64 { host::host_random_random(&mut c) });
    w!("random_int", |mut c: Caller<'_, HostState>, min: i64, max: i64| -> i64 {
        host::host_random_int(&mut c, min, max)
    });
    w!("random_float", |mut c: Caller<'_, HostState>, min: f64, max: f64| -> f64 {
        host::host_random_float(&mut c, min, max)
    });
    w!("random_uuid", |mut c: Caller<'_, HostState>| -> i64 { host::host_random_uuid(&mut c) });
    Ok(())
}

/// Registra hosts para las extensiones (`env.<sym>__<sig>@<lib>`) que delegan en
/// el backend nativo del nodo. En wasmi (browser) el nodo puede pasar un backend
/// que rechaza el acceso (o no registrar extensiones).
fn register_native_hosts(
    linker: &mut Linker<HostState>,
    module: &wasmi::Module,
    backend: Arc<dyn cls_runtime::ffi::NativeBackend>,
) -> Result<usize, String> {
    use cls_runtime::ffi::NativeType;
    use cls_runtime::Value;
    let mut count = 0;
    let imports: Vec<(String, String)> = module
        .imports()
        .map(|it| (it.module().to_string(), it.name().to_string()))
        .collect();
    for (m, n) in imports {
        if m != "env" {
            continue;
        }
        let (rest, lib) = match n.split_once('@') {
            Some(x) => x,
            None => continue,
        };
        let (sym, sig) = match rest.split_once("__") {
            Some(x) => x,
            None => continue,
        };
        let sym = sym.to_string();
        let lib = lib.to_string();
        let sig = sig.to_string();
        let name = n.clone();
        let native_type = |c: char| match c {
            'f' => NativeType::Float,
            'b' => NativeType::Bool,
            'c' => NativeType::CInt,
            's' => NativeType::CString,
            _ => NativeType::Int,
        };
        let ret_to_i64 = |v: Result<Value, cls_core::error::ClsError>| -> i64 {
            match v {
                Ok(Value::Int(n)) => n,
                Ok(Value::Float(f)) => f as i64,
                Ok(Value::Bool(b)) => {
                    if b {
                        1
                    } else {
                        0
                    }
                }
                Ok(Value::Char(ch)) => ch as i64,
                Ok(_) => 0,
                Err(_) => 0,
            }
        };
        let ret_to_f64 = |v: Result<Value, cls_core::error::ClsError>| -> f64 {
            match v {
                Ok(Value::Float(f)) => f,
                Ok(Value::Int(n)) => n as f64,
                Ok(_) => 0.0,
                Err(_) => 0.0,
            }
        };
        let params: Vec<char> = sig.chars().skip(1).collect();
        let rcode = sig.chars().next().unwrap_or('i');
        let lib2 = lib.clone();
        let sym2 = sym.clone();
        match params.as_slice() {
            [] => {
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                let backend = backend.clone();
                linker
                    .func_wrap(HOST, &name, move |_: Caller<'_, HostState>| -> i64 {
                        let _ = &lib3;
                        let _ = &sym3;
                        let r = backend.call_function(&lib3, &sym3, &[], &[], native_type(ret));
                        ret_to_i64(r)
                    })
                    .map_err(|e| e.to_string())?;
            }
            [p0] if *p0 == 'f' => {
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                let backend = backend.clone();
                linker
                    .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: f64| -> f64 {
                        let r = backend.call_function(
                            &lib3,
                            &sym3,
                            &[Value::Float(a)],
                            &[NativeType::Float],
                            native_type(ret),
                        );
                        ret_to_f64(r)
                    })
                    .map_err(|e| e.to_string())?;
            }
            [p0] => {
                let p0 = *p0;
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                let backend = backend.clone();
                linker
                    .func_wrap(HOST, &name, move |mut caller: Caller<'_, HostState>, a: i64| -> i64 {
                        let arg = match p0 {
                            's' => Value::String(caller.read_str(a)),
                            _ => Value::Int(a),
                        };
                        let r = backend.call_function(&lib3, &sym3, &[arg], &[native_type(p0)], native_type(ret));
                        match r {
                            Ok(Value::String(s)) => caller.write_str(&s),
                            Ok(v) => match v {
                                Value::Float(f) => f as i64,
                                Value::Int(n) => n,
                                _ => 0,
                            },
                            Err(_) => 0,
                        }
                    })
                    .map_err(|e| e.to_string())?;
            }
            [p0, p1] if *p0 == 'f' || *p1 == 'f' => {
                let p0 = *p0;
                let p1 = *p1;
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                if p0 == 'f' && p1 == 'f' {
                    let backend = backend.clone();
                    linker
                        .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: f64, b: f64| -> f64 {
                            let r = backend.call_function(
                                &lib3,
                                &sym3,
                                &[Value::Float(a), Value::Float(b)],
                                &[NativeType::Float, NativeType::Float],
                                native_type(ret),
                            );
                            ret_to_f64(r)
                        })
                        .map_err(|e| e.to_string())?;
                } else if p0 == 'f' {
                    let lib4 = lib2.clone();
                    let sym4 = sym2.clone();
                    let backend = backend.clone();
                    linker
                        .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: f64, b: i64| -> f64 {
                            let r = backend.call_function(
                                &lib4,
                                &sym4,
                                &[Value::Float(a), Value::Int(b)],
                                &[NativeType::Float, NativeType::Int],
                                native_type(ret),
                            );
                            ret_to_f64(r)
                        })
                        .map_err(|e| e.to_string())?;
                } else {
                    let lib4 = lib2.clone();
                    let sym4 = sym2.clone();
                    let backend = backend.clone();
                    linker
                        .func_wrap(HOST, &name, move |_: Caller<'_, HostState>, a: i64, b: f64| -> f64 {
                            let r = backend.call_function(
                                &lib4,
                                &sym4,
                                &[Value::Int(a), Value::Float(b)],
                                &[NativeType::Int, NativeType::Float],
                                native_type(ret),
                            );
                            ret_to_f64(r)
                        })
                        .map_err(|e| e.to_string())?;
                }
            }
            [p0, p1] => {
                let p0 = *p0;
                let p1 = *p1;
                let ret = rcode;
                let lib3 = lib2.clone();
                let sym3 = sym2.clone();
                let backend = backend.clone();
                linker
                    .func_wrap(HOST, &name, move |mut caller: Caller<'_, HostState>, a: i64, b: i64| -> i64 {
                        let arg0 = match p0 {
                            's' => Value::String(caller.read_str(a)),
                            _ => Value::Int(a),
                        };
                        let arg1 = match p1 {
                            's' => Value::String(caller.read_str(b)),
                            _ => Value::Int(b),
                        };
                        let r = backend.call_function(
                            &lib3,
                            &sym3,
                            &[arg0, arg1],
                            &[native_type(p0), native_type(p1)],
                            native_type(ret),
                        );
                        ret_to_i64(r)
                    })
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                return Err(format!(
                    "[JIT] Extensión '{}': el JIT soporta natives de hasta 2 argumentos por ahora",
                    n
                ))
            }
        }
        count += 1;
    }
    Ok(count)
}

// ── Ejecución ───────────────────────────────────────────────────────────────

pub(crate) fn run_wasm_wasmi(
    wasm_bytes: &[u8],
    entry: &str,
    app_args: &[String],
    timing: bool,
    mut t: Instant,
    cache_path: Option<std::path::PathBuf>,
    modules: &[(String, String, ClsModule)],
    ctx: &JitContext,
) -> i32 {
    let engine = Engine::default();
    t = tick(timing, "Engine (wasmi)", t);

    let module = match Module::new(&engine, wasm_bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[JIT] Módulo WASM inválido para '{}' (wasmi):\n{:?}", entry, e);
            if let Some(p) = &cache_path {
                let _ = std::fs::remove_file(p);
            }
            return 1;
        }
    };
    t = tick(timing, "Module::new (wasmi)", t);

    // El WASM es válido: persistirlo en el caché CLS->WASM (fallo silencioso).
    if let Some(p) = &cache_path {
        let _ = std::fs::create_dir_all(crate::resolve::cache_dir())
            .and_then(|_| crate::resolve::atomic_write(p, wasm_bytes));
        crate::engine::maybe_write_module_index(entry, modules, ctx);
    }

    let mut store = Store::new(
        &engine,
        HostState {
            first_in_line: true,
            source_file: entry.to_string(),
            modules: module_offsets(modules),
            string_caps: std::collections::HashMap::new(),
            call_stack: Vec::new(),
            pending_call_site: None,
            simple_fn_names: std::collections::HashMap::new(),
            host_call: ctx.host_call_handler.clone(),
            output: ctx.output.clone(),
            app_args: app_args.to_vec(),
        },
    );
    let mut linker = Linker::new(&engine);
    t = tick(timing, "Store+Linker", t);

    if let Err(e) = register_host_functions(&mut linker) {
        eprintln!("[JIT] Error registrando funciones host: {}", e);
        return 1;
    }
    if let Err(e) = register_native_hosts(&mut linker, &module, ctx.native_backend.clone()) {
        eprintln!("[JIT] Error registrando hosts de extensiones: {}", e);
        return 1;
    }
    t = tick(timing, "register hosts", t);

    let instance = match linker.instantiate(&mut store, &module) {
        Ok(pre) => match pre.start(&mut store) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("[JIT] Error de arranque para '{}': {}", entry, e);
                return 1;
            }
        },
        Err(e) => {
            eprintln!("[JIT] Error de instanciación para '{}': {}", entry, e);
            return 1;
        }
    };
    t = tick(timing, "instantiate", t);

    let alloc = match instance.get_typed_func::<i64, i64>(&store, "alloc") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'alloc' no disponible: {}", e);
            return 1;
        }
    };
    let memory = match instance.get_memory(&store, "memory") {
        Some(m) => m,
        None => {
            eprintln!("[JIT] Export 'memory' no disponible");
            return 1;
        }
    };

    let args_ptr = write_args_store(&mut store, &memory, &alloc, app_args);
    t = tick(timing, "write_args", t);

    let main = match instance.get_typed_func::<i64, i64>(&store, "main") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'main' no disponible: {}", e);
            return 1;
        }
    };

    let result = match main.call(&mut store, args_ptr) {
        Ok(code) => code as i32,
        Err(e) => {
            // wasmi no implementa excepciones CLS: todo error es un trap.
            // El shadow call stack (fn_enter) da el trace; el span del CLS no
            // viaja (los errores de runtime se emitieron como `unreachable`).
            let short = e.to_string();
            let call_stack = store.data().call_stack.clone();
            let pending = store.data().pending_call_site;
            let modules = store.data().modules.clone();
            finish_run_error(
                String::new(),
                None,
                call_stack,
                pending,
                short,
                entry,
                &modules,
            )
        }
    };
    tick(timing, "ejecución main", t);
    result
}

/// Escribe los args como Array<String> en memoria (versión wasmi) y devuelve el ptr.
fn write_args_store(
    store: &mut Store<HostState>,
    memory: &Memory,
    alloc: &wasmi::TypedFunc<i64, i64>,
    app_args: &[String],
) -> i64 {
    let n = app_args.len() as i64;
    let array_ptr = alloc.call(&mut *store, n * 8 + 16).unwrap_or(0);
    let _ = memory.write(&mut *store, array_ptr as usize, &n.to_le_bytes());
    let _ = memory.write(&mut *store, (array_ptr as usize) + 8, &n.to_le_bytes());
    for (i, arg) in app_args.iter().enumerate() {
        let sptr = alloc.call(&mut *store, arg.len() as i64).unwrap_or(0);
        let _ = memory.write(&mut *store, sptr as usize, arg.as_bytes());
        let packed = (sptr << 32) | (arg.len() as i64);
        let _ = memory.write(
            &mut *store,
            (array_ptr as usize) + 16 + i * 8,
            &packed.to_le_bytes(),
        );
    }
    array_ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reproducción del pipeline completo con wasmi, en proceso (para obtener
    /// backtrace si el runtime panica).
    #[test]
    fn run_mini_wasmi() {
        let src = r#"
            function main(args: String[]) -> int {
                var a: int = 2;
                var b: int = 3;
                return a * b + 1;
            };
        "#;
        let dir = std::env::temp_dir().join("cls-wasmi-test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("mini.clsx");
        std::fs::write(&file, src).unwrap();
        let file = file.to_string_lossy().to_string();

        struct NoNative;
        impl cls_runtime::ffi::NativeBackend for NoNative {
            fn call_function(
                &self,
                _lib: &str,
                _sym: &str,
                _args: &[cls_runtime::Value],
                _arg_types: &[cls_runtime::ffi::NativeType],
                _ret: cls_runtime::ffi::NativeType,
            ) -> cls_core::error::ClsResult<cls_runtime::Value> {
                Err(cls_core::error::ClsError::RuntimeError("no native".into()))
            }
            fn get_variable(
                &self,
                _lib: &str,
                _sym: &str,
                _ty: cls_runtime::ffi::NativeType,
            ) -> cls_core::error::ClsResult<cls_runtime::Value> {
                Err(cls_core::error::ClsError::RuntimeError("no native".into()))
            }
            fn set_variable(
                &self,
                _lib: &str,
                _sym: &str,
                _val: cls_runtime::ffi::NativeType,
                _ty: &cls_runtime::Value,
            ) -> cls_core::error::ClsResult<()> {
                Err(cls_core::error::ClsError::RuntimeError("no native".into()))
            }
        }

        let ctx = crate::JitContext {
            native_backend: Arc::new(NoNative),
            module_index: None,
            host_intrinsics: &[],
            host_call_handler: None,
            module_source_resolver: None,
            output: None,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::engine::run_jit_with(
                &file,
                &[],
                None,
                &ctx,
                crate::RuntimeKind::Wasmi,
            )
        }));
        match result {
            // run_jit devuelve el exit code de main: 2*3+1 = 7.
            Ok(code) => assert_eq!(code, 7, "exit code de main"),
            Err(p) => {
                let msg = p
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| p.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("<panic sin mensaje>");
                panic!("wasmi panicked: {}", msg);
            }
        }
    }
}
