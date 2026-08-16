//! Runtime wasmtime (desktop): impl de `HostCtx` sobre `Caller`, registro de
//! host functions (adaptadores de una línea a los cuerpos genéricos de
//! `crate::host`) y la ejecución del módulo WASM.

use cls_core::error::Span;
use cls_core::frontend::ast::Module as ClsModule;
use std::sync::Arc;
use std::time::Instant;
use wasmtime::{Caller, Engine, Instance, Linker, Memory, Module, Store, Val};

use crate::engine::{finish_run_error, module_offsets, unpack_span};
use crate::host::{self, HostCtx};
use crate::state::HostState;
use crate::timing::tick;
use crate::JitContext;

pub(crate) const HOST: &str = "env";

// ── HostCtx para wasmtime ───────────────────────────────────────────────────

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
        if let Err(e) = func.call(self, &[Val::I64(n)], &mut results) {
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
            let data = mem.data_mut(self);
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
            let data = mem.data_mut(self);
            if addr + 4 <= data.len() {
                data[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
    }

    fn write_bytes(&mut self, addr: usize, bytes: &[u8]) -> bool {
        if let Some(mem) = self.get_export("memory").and_then(|e| e.into_memory()) {
            let data = mem.data_mut(self);
            if addr + bytes.len() <= data.len() {
                data[addr..addr + bytes.len()].copy_from_slice(bytes);
                return true;
            }
        }
        false
    }
}

// ── Registro de host functions (adaptadores) ────────────────────────────────

/// Registra las host functions `env.*` (adaptadores de una línea a los cuerpos
/// genéricos). Público para que el nodo de bindings (`clxb`) construya su propio
/// Linker. `embed_exit = true` omite `exit`/`trap` (el embedding los define para
/// no matar el proceso del host).
pub fn register_host_functions(linker: &mut Linker<HostState>) -> Result<(), String> {
    register_host_functions_opt(linker, false, false)
}

/// Como [`register_host_functions`] con control de `exit`/`trap` y del sandbox.
/// `embed_exit = true`: omite `exit`/`trap` (el embedding los define).
/// `sandbox = true`: omite los módulos del nodo desktop (`fs`, `http`, `os`,
/// `path`, `process`, `time`, `random`) - solo core (print/math/json/strings).
pub fn register_host_functions_opt(
    linker: &mut Linker<HostState>,
    embed_exit: bool,
    sandbox: bool,
) -> Result<(), String> {
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
    if !embed_exit {
        w!("exit", |mut c: Caller<'_, HostState>, code: i64| host::host_exit(&mut c, code));
        w!("trap", |mut c: Caller<'_, HostState>, m: i64, s: i64| host::host_trap(&mut c, m, s));
    }
    w!("sleep", |mut c: Caller<'_, HostState>, ms: i64| host::host_sleep(&mut c, ms));
    w!("parse_int", |mut c: Caller<'_, HostState>, v: i64| -> Result<i64, wasmtime::Error> {
        host::host_parse_int(&mut c, v).map_err(wasmtime::Error::msg)
    });
    w!("parse_float", |mut c: Caller<'_, HostState>, v: i64| -> Result<f64, wasmtime::Error> {
        host::host_parse_float(&mut c, v).map_err(wasmtime::Error::msg)
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
    if !sandbox {
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
    }
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
    if !sandbox {
        w!("http_get", |mut c: Caller<'_, HostState>, u: i64| -> i64 { host::host_http_get(&mut c, u) });
        w!("http_post", |mut c: Caller<'_, HostState>, u: i64, d: i64| -> i64 {
            host::host_http_post(&mut c, u, d)
        });
    }
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
    if !sandbox {
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
    }
    Ok(())
}

/// Registra hosts para las extensiones (`env.<sym>__<sig>@<lib>`) que delegan en
/// el backend nativo del nodo (libloading). `sig` = ret+params: i=int, f=float,
/// b=bool, c=char, s=string, I=int C de 32 bits (CInt/...), v=void.
///
/// Usa `Linker::func_new` con la firma dinámica `(Caller, &[Val], &mut [Val])`
/// y el `FuncType` real del import: un solo wrapper genérico soporta aridad
/// 0..4 (el límite real lo pone `native.rs`, hasta 4 args) y cualquier mezcla
/// de float/int/string, con el retorno tipado por la letra (`f` → f64 siempre,
/// sin corromper bits con `f as i64`).
pub fn register_native_hosts(
    linker: &mut Linker<HostState>,
    module: &wasmtime::Module,
    backend: Arc<dyn cls_runtime::ffi::NativeBackend>,
) -> Result<usize, String> {
    use cls_runtime::ffi::NativeType;
    use cls_runtime::Value;
    let mut count = 0;
    let imports: Vec<(String, String, wasmtime::FuncType)> = module
        .imports()
        .filter_map(|it| {
            let ft = it.ty().func()?.clone();
            Some((it.module().to_string(), it.name().to_string(), ft))
        })
        .collect();
    for (m, n, ft) in imports {
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
            'I' => NativeType::CInt,
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
        let ret_to_i32 = |v: Result<Value, cls_core::error::ClsError>| -> i32 {
            match v {
                Ok(Value::Int(n)) => n as i32,
                Ok(Value::Bool(b)) => {
                    if b {
                        1
                    } else {
                        0
                    }
                }
                Ok(Value::Char(ch)) => ch as i32,
                Ok(_) => 0,
                Err(_) => 0,
            }
        };
        let params: Vec<char> = sig.chars().skip(1).collect();
        let rcode = sig.chars().next().unwrap_or('i');
        let lib2 = lib.clone();
        let sym2 = sym.clone();
        let letters = params.clone();
        let backend = backend.clone();
        let res = linker.func_new(
            HOST,
            &name,
            ft,
            move |mut caller: Caller<'_, HostState>, args: &[Val], results: &mut [Val]| {
                let mut vals: Vec<Value> = Vec::with_capacity(args.len());
                let mut ptypes: Vec<NativeType> = Vec::with_capacity(args.len());
                for (i, a) in args.iter().enumerate() {
                    let letter = letters.get(i).copied().unwrap_or('i');
                    let v = match letter {
                        's' => Value::String(caller.read_str(a.i64().unwrap_or(0))),
                        'f' => Value::Float(a.f64().unwrap_or(0.0)),
                        'b' => Value::Bool(a.i32().unwrap_or(0) != 0),
                        'c' => Value::Int(a.i32().unwrap_or(0) as i64),
                        _ => Value::Int(a.i64().unwrap_or(0)),
                    };
                    vals.push(v);
                    ptypes.push(native_type(letter));
                }
                let r = backend.call_function(&lib2, &sym2, &vals, &ptypes, native_type(rcode));
                match rcode {
                    'v' => Ok(()),
                    's' => {
                        let s = match r {
                            Ok(Value::String(s)) => s,
                            _ => String::new(),
                        };
                        results[0] = Val::I64(caller.write_str(&s));
                        Ok(())
                    }
                    'f' => {
                        results[0] = Val::F64(ret_to_f64(r).to_bits());
                        Ok(())
                    }
                    'b' | 'c' => {
                        results[0] = Val::I32(ret_to_i32(r));
                        Ok(())
                    }
                    _ => {
                        results[0] = Val::I64(ret_to_i64(r));
                        Ok(())
                    }
                }
            },
        );
        if let Err(e) = res {
            return Err(format!("[JIT] Extensión '{}': no se pudo registrar el host: {}", n, e));
        }
        count += 1;
    }
    Ok(count)
}

// ── Ejecución ───────────────────────────────────────────────────────────────

/// Config de wasmtime: habilita la propuesta de excepciones (try/catch).
fn wasmtime_config() -> Option<wasmtime::Config> {
    let mut config = wasmtime::Config::new();
    config.wasm_exceptions(true);
    Some(config)
}

pub(crate) fn run_wasm_wasmtime(
    wasm_bytes: &[u8],
    entry: &str,
    app_args: &[String],
    timing: bool,
    mut t: Instant,
    cache_path: Option<std::path::PathBuf>,
    modules: &[(String, String, ClsModule)],
    ctx: &JitContext,
) -> i32 {
    let engine = match wasmtime_config() {
        Some(config) => match Engine::new(&config) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[JIT] Error creando engine wasmtime: {}", e);
                return 1;
            }
        },
        None => Engine::default(),
    };
    t = tick(timing, "Engine::default", t);

    let module = match Module::new(&engine, wasm_bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[JIT] Módulo WASM inválido para '{}':\n{:?}", entry, e);
            if let Ok(wat) = wasmprinter::print_bytes(wasm_bytes) {
                eprintln!("--- WAT ---\n{}", wat);
            }
            if let Some(p) = &cache_path {
                let _ = std::fs::remove_file(p);
            }
            return 1;
        }
    };
    t = tick(timing, "Module::new (Cranelift)", t);

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
        Ok(i) => i,
        Err(e) => {
            eprintln!("[JIT] Error de instanciación para '{}': {}", entry, e);
            return 1;
        }
    };
    t = tick(timing, "instantiate", t);

    let alloc = match instance.get_typed_func::<i64, i64>(&mut store, "alloc") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'alloc' no disponible: {}", e);
            return 1;
        }
    };
    let memory = match instance.get_memory(&mut store, "memory") {
        Some(m) => m,
        None => {
            eprintln!("[JIT] Export 'memory' no disponible");
            return 1;
        }
    };

    // Escribir los args de la app en la memoria y llamar main(ptr).
    let args_ptr = write_args_store(&mut store, &memory, &alloc, app_args);
    t = tick(timing, "write_args", t);

    let main = match instance.get_typed_func::<i64, i64>(&mut store, "main") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[JIT] Export 'main' no disponible: {}", e);
            return 1;
        }
    };

    let result = match main.call(&mut store, args_ptr) {
        Ok(code) => code as i32,
        Err(_e) => {
            // Excepción CLS no capturada: extraer el payload [msg, span] del tag
            // y formatear el error como runtime (con el mensaje real, no un trap genérico).
            let payload: Vec<Val> = {
                let exn = store.take_pending_exception();
                let mut fields = match exn.as_ref() {
                    Some(e) => e.fields(&mut store).ok(),
                    None => None,
                };
                match fields.as_mut() {
                    Some(iter) => iter.collect(),
                    None => Vec::new(),
                }
            };
            let msg = payload
                .first()
                .and_then(|v| v.i64())
                .map(|packed| read_packed_str(&mut store, &memory, packed))
                .unwrap_or_default();
            let span = payload.get(1).and_then(|v| v.i64()).map(unpack_span);
            let root = _e.root_cause().to_string();
            let short = if root.is_empty() { _e.to_string() } else { root };
            let (call_stack, pending) = read_shadow_trace(&mut store, &instance, &memory);
            finish_run_error(msg, span, call_stack, pending, short, entry, &store.data().modules)
        }
    };
    tick(timing, "ejecución main", t);
    result
}

/// Escribe los args como Array<String> en memoria (versión Store/Memory de
/// wasmtime, con el TypedFunc de alloc) y devuelve el ptr.
fn write_args_store(
    store: &mut Store<HostState>,
    memory: &Memory,
    alloc: &wasmtime::TypedFunc<i64, i64>,
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

/// Lee un string empaquetado `(ptr<<32)|len` desde la memoria del módulo.
fn read_packed_str(store: &mut Store<HostState>, memory: &Memory, packed: i64) -> String {
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    memory
        .data(store)
        .get(ptr..ptr.saturating_add(len))
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

/// Lee el shadow call stack del módulo (escrito por el emisor en la memoria
/// lineal). `pending` siempre es `None`: el `fn_enter` del callee ya consumió
/// el pending como span de su frame (paridad con `pending_call_site.take()`).
pub(crate) fn read_shadow_trace(
    store: &mut Store<HostState>,
    instance: &Instance,
    memory: &Memory,
) -> (Vec<(String, Span)>, Option<Span>) {
    let gi32 = |store: &mut Store<HostState>, name: &str| -> u32 {
        instance
            .get_global(&mut *store, name)
            .and_then(|g| g.get(&mut *store).i32())
            .unwrap_or(0) as u32
    };
    let shadow_ptr = gi32(&mut *store, "__shadow_ptr");
    let base = gi32(&mut *store, "__shadow_base");
    let stb = gi32(&mut *store, "__string_table_base");
    let data = memory.data(&mut *store);
    let read_u32 = |addr: usize| -> u32 {
        let mut b = [0u8; 4];
        let _ = data.get(addr..addr.saturating_add(4)).map(|s| b.copy_from_slice(s));
        u32::from_le_bytes(b)
    };
    let read_bytes = |addr: usize, len: usize| -> Vec<u8> {
        data.get(addr..addr.saturating_add(len)).map(|s| s.to_vec()).unwrap_or_default()
    };
    let name_at = |idx: u32| crate::engine::name_at(stb, idx, read_u32, read_bytes);
    let stack = crate::engine::read_shadow_stack(shadow_ptr, base, read_u32, name_at);
    (stack, None)
}
