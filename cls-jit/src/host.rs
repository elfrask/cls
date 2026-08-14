//! Cuerpos genéricos de las host functions `env.*` (compartidos por todos los
//! runtimes: wasmtime desktop, wasmi navegador).
//!
//! Cada host function es una función genérica `host_xxx<C: HostCtx>(ctx, args...)`.
//! Cada runtime implementa [`HostCtx`] sobre su tipo de caller y registra los
//! hosts con adaptadores de una línea. Así el comportamiento es UNO solo y la
//! duplicación por runtime es mínima.

use crate::state::HostState;
use cls_core::error::{ClsError, Span};
use cls_runtime::error_report::{format_error, ErrorFormat, ErrorReport};

/// Un argumento de una función host del nodo (canal `env.host_call`).
/// `tag` = `cls_kind_code` (0=int 1=float 2=bool 3=char 4=string 5=array
/// 6=record 7=tuple 8=otro-i64 9=void 10=cmx 11=función 12=null).
/// Para strings (tag 4) `text` ya viene resuelto; `bits` es el packed
/// `(ptr<<32)|len` original.
#[derive(Debug, Clone)]
pub struct HostCallArg {
    pub tag: i64,
    pub bits: i64,
    pub text: Option<String>,
}

/// Resultado de una función host del nodo.
/// Para retorno de string (tag 4) se usa `text` (el motor lo escribe en la
/// memoria del módulo); para el resto, `bits` (float = bits de f64, bool =
/// 0/1, enum = `(def_id<<32)|index`).
#[derive(Debug, Clone)]
pub struct HostCallResult {
    pub tag: i64,
    pub bits: i64,
    pub text: Option<String>,
}

/// Handler del canal `env.host_call` que el NODO provee para sus intrinsics.
pub trait HostCallHandler: Send + Sync {
    /// Despacha una llamada por id. El nodo conoce la firma (la registró en
    /// `JitContext.host_intrinsics`); los args llegan resueltos.
    fn call(&self, id: u32, args: &[HostCallArg]) -> Result<HostCallResult, String>;
}

/// Destino de la salida de `print` del script. Si el nodo registra uno, todo
/// `print`/`printEnd` se redirige aquí (en vez de stdout).
pub trait OutputSink: Send + Sync {
    /// Un valor de print (sin separador; el motor agrega los espacios).
    fn write(&self, s: &str);
    /// Fin de línea (`print()` sin args / fin de la llamada print).
    fn end_line(&self);
}

/// Resolver de módulos del NODO: provee el source de `import "path"` que no se
/// resuelve en disco (módulos en memoria, VFS, red, ...).
pub trait ModuleSourceResolver: Send + Sync {
    fn resolve_source(&self, path: &str, base_dir: &std::path::Path) -> Option<String>;
}

/// Implementación de `env.host_call(id, ptr, n)`: lee el bloque empaquetado
/// `[n:i64][(val:i64, tag:i64)*n]`, despacha al handler del nodo y escribe el
/// retorno (los strings se escriben en la memoria del módulo).
pub fn host_host_call<C: HostCtx>(ctx: &mut C, id: i64, ptr: i64, n: i64) -> i64 {
    let handler = match &ctx.state().host_call {
        Some(h) => h.clone(),
        None => {
            eprintln!("[JIT] host_call id={} pero el nodo no registró handler", id);
            return 0;
        }
    };
    let mut args: Vec<HostCallArg> = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        let base = ptr as usize + 8 + i * 16;
        let bits = ctx.read_i64(base);
        let tag = ctx.read_i64(base + 8);
        let text = if tag == 4 { Some(ctx.read_str(bits)) } else { None };
        args.push(HostCallArg { tag, bits, text });
    }
    match handler.call(id as u32, &args) {
        Ok(r) => match r.text {
            Some(s) => ctx.write_str(&s),
            None => r.bits,
        },
        Err(msg) => {
            eprintln!("[JIT] host_call id={} falló: {}", id, msg);
            0
        }
    }
}

/// Acceso del host a la memoria lineal del módulo, al allocator y al estado.
/// Lo implementa cada runtime (wasmtime: `Caller<'_, HostState>`; wasmi: el suyo).
pub trait HostCtx {
    fn state(&self) -> &HostState;
    fn state_mut(&mut self) -> &mut HostState;
    /// Lee un string empaquetado `(ptr<<32)|len` de la memoria del módulo.
    fn read_str(&mut self, packed: i64) -> String;
    /// Aloca + escribe un string en la memoria del módulo y lo empaqueta.
    /// Registra la capacidad para que `str_concat` reutilice el buffer.
    fn write_str(&mut self, s: &str) -> i64;
    /// Llama al allocator exportado del módulo. Aborta con mensaje claro si la
    /// memoria no alcanza (devuelve 0 = inválido).
    fn alloc(&mut self, n: i64) -> i64;
    fn read_i64(&mut self, addr: usize) -> i64;
    fn write_i64(&mut self, addr: usize, v: i64);
    fn read_i32(&mut self, addr: usize) -> i32;
    fn write_i32(&mut self, addr: usize, v: i32);
    /// Copia bytes a la memoria del módulo (bounds-checked, sin panic).
    /// Devuelve `true` si la escritura entró completa.
    fn write_bytes(&mut self, addr: usize, bytes: &[u8]) -> bool;
}

fn format_float(v: f64) -> String {
    format!("{}", v)
}

fn print_arg<C: HostCtx>(ctx: &mut C, value: &str) {
    let state = ctx.state_mut();
    if let Some(out) = &state.output {
        if !state.first_in_line {
            out.write(" ");
        }
        out.write(value);
        state.first_in_line = false;
        return;
    }
    if !state.first_in_line {
        print!(" ");
    }
    print!("{}", value);
    state.first_in_line = false;
}

// ── print ───────────────────────────────────────────────────────────────────

pub fn host_print_int<C: HostCtx>(ctx: &mut C, v: i64) {
    print_arg(ctx, &v.to_string());
}

pub fn host_print_float<C: HostCtx>(ctx: &mut C, v: f64) {
    print_arg(ctx, &format_float(v));
}

pub fn host_print_bool<C: HostCtx>(ctx: &mut C, v: i32) {
    print_arg(ctx, if v != 0 { "true" } else { "false" });
}

pub fn host_print_char<C: HostCtx>(ctx: &mut C, v: i32) {
    let c = char::from_u32(v as u32).unwrap_or('?');
    print_arg(ctx, &c.to_string());
}

pub fn host_print_str<C: HostCtx>(ctx: &mut C, v: i64) {
    let s = ctx.read_str(v);
    print_arg(ctx, &s);
}

pub fn host_print_end<C: HostCtx>(ctx: &mut C) {
    if let Some(out) = &ctx.state().output {
        out.end_line();
        ctx.state_mut().first_in_line = true;
        return;
    }
    println!();
    ctx.state_mut().first_in_line = true;
}

pub fn host_print_any<C: HostCtx>(ctx: &mut C, val: i64, tag: i64) {
    let s = fmt_val_to_string(ctx, val, tag);
    print_arg(ctx, &s);
}

// ── sistema ─────────────────────────────────────────────────────────────────

pub fn host_now<C: HostCtx>(_ctx: &mut C) -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn host_exit<C: HostCtx>(_ctx: &mut C, code: i64) {
    std::process::exit(code as i32);
}

pub fn host_sleep<C: HostCtx>(_ctx: &mut C, ms: i64) {
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

pub fn host_trap<C: HostCtx>(ctx: &mut C, msg: i64, span: i64) {
    let s = ctx.read_str(msg);
    let file = ctx.state().source_file.clone();
    let line = ((span >> 32) & 0xffff_ffff) as u32;
    let col = (span & 0xffff_ffff) as u32;
    let err = ClsError::RuntimeError(s);
    let span_s = if line > 0 {
        Some(Span::new(line, col, line, col))
    } else {
        None
    };
    let report = ErrorReport {
        error: err,
        span: span_s,
        stack: vec![],
        import_trace: vec![],
        source_file: file,
        source: None,
    };
    eprintln!("{}", format_error(&report, &ErrorFormat::Console));
    std::process::exit(1);
}

/// Extrae el mensaje + ubicación de un trap CLS sin matar el proceso.
/// Lo usan los embeddings (clxb) que interceptan `env.trap` como error.
pub fn host_trap_message<C: HostCtx>(ctx: &mut C, msg: i64, span: i64) -> String {
    let s = ctx.read_str(msg);
    let line = ((span >> 32) & 0xffff_ffff) as u32;
    let col = (span & 0xffff_ffff) as u32;
    if line > 0 {
        format!("{} (línea {}, columna {})", s, line, col)
    } else {
        s
    }
}

// ── conversiones (errores con trap → mensaje claro) ─────────────────────────

pub fn host_parse_int<C: HostCtx>(ctx: &mut C, v: i64) -> Result<i64, String> {
    let s = ctx.read_str(v);
    let t = s.trim();
    t.parse::<i64>()
        .map_err(|_| format!("int: no se puede convertir '{}'", t))
}

pub fn host_parse_float<C: HostCtx>(ctx: &mut C, v: i64) -> Result<f64, String> {
    let s = ctx.read_str(v);
    let t = s.trim();
    t.parse::<f64>()
        .map_err(|_| format!("float: no se puede convertir '{}'", t))
}

pub fn host_parse_bool<C: HostCtx>(ctx: &mut C, v: i64) -> i32 {
    // Truthiness de string (paridad walker): vacío → false, no vacío → true.
    let s = ctx.read_str(v);
    if s.is_empty() {
        0
    } else {
        1
    }
}

// ── strings ─────────────────────────────────────────────────────────────────

pub fn host_str_concat<C: HostCtx>(ctx: &mut C, a: i64, b: i64) -> i64 {
    let a_ptr = (a >> 32) as usize;
    let a_len = (a & 0xffff_ffff) as usize;
    let sa = ctx.read_str(a);
    let sb = ctx.read_str(b);
    let out = format!("{}{}", sa, sb);
    // Reutilizar el buffer de `a` si tiene capacidad (amortizado): evita alocar
    // nuevo en cada iteración de un loop de concat.
    let cap = ctx.state().string_caps.get(&(a_ptr as i64)).copied();
    if let Some(cap) = cap {
        if (out.len() as i64) <= cap && out.len() >= a_len {
            if ctx.write_bytes(a_ptr, out.as_bytes()) {
                return ((a_ptr as i64) << 32) | (out.len() as i64);
            }
        }
    }
    ctx.write_str(&out)
}

pub fn host_str_int<C: HostCtx>(ctx: &mut C, v: i64) -> i64 {
    ctx.write_str(&v.to_string())
}

pub fn host_str_float<C: HostCtx>(ctx: &mut C, v: f64) -> i64 {
    ctx.write_str(&format_float(v))
}

pub fn host_str_bool<C: HostCtx>(ctx: &mut C, v: i32) -> i64 {
    ctx.write_str(if v != 0 { "true" } else { "false" })
}

pub fn host_str_char<C: HostCtx>(ctx: &mut C, v: i32) -> i64 {
    let c = char::from_u32(v as u32).unwrap_or('?');
    ctx.write_str(&c.to_string())
}

pub fn host_str_upper<C: HostCtx>(ctx: &mut C, v: i64) -> i64 {
    let s = ctx.read_str(v);
    ctx.write_str(&s.to_uppercase())
}

pub fn host_str_lower<C: HostCtx>(ctx: &mut C, v: i64) -> i64 {
    let s = ctx.read_str(v);
    ctx.write_str(&s.to_lowercase())
}

pub fn host_str_trim<C: HostCtx>(ctx: &mut C, v: i64) -> i64 {
    let s = ctx.read_str(v);
    ctx.write_str(s.trim())
}

pub fn host_str_contains<C: HostCtx>(ctx: &mut C, a: i64, b: i64) -> i32 {
    let sa = ctx.read_str(a);
    let sb = ctx.read_str(b);
    if sa.contains(&sb) {
        1
    } else {
        0
    }
}

pub fn host_str_starts_with<C: HostCtx>(ctx: &mut C, a: i64, b: i64) -> i32 {
    let sa = ctx.read_str(a);
    let sb = ctx.read_str(b);
    if sa.starts_with(&sb) {
        1
    } else {
        0
    }
}

pub fn host_str_ends_with<C: HostCtx>(ctx: &mut C, a: i64, b: i64) -> i32 {
    let sa = ctx.read_str(a);
    let sb = ctx.read_str(b);
    if sa.ends_with(&sb) {
        1
    } else {
        0
    }
}

pub fn host_str_is_empty<C: HostCtx>(ctx: &mut C, v: i64) -> i32 {
    let s = ctx.read_str(v);
    if s.is_empty() {
        1
    } else {
        0
    }
}

pub fn host_str_repr<C: HostCtx>(ctx: &mut C, v: i64) -> i64 {
    let s = ctx.read_str(v);
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    ctx.write_str(&format!("\"{}\"", escaped))
}

pub fn host_str_length<C: HostCtx>(ctx: &mut C, v: i64) -> i64 {
    let s = ctx.read_str(v);
    s.len() as i64
}

pub fn host_int_abs<C: HostCtx>(_ctx: &mut C, v: i64) -> i64 {
    v.abs()
}

pub fn host_float_abs<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.abs()
}

// ── arrays ──────────────────────────────────────────────────────────────────

fn arr_len<C: HostCtx>(ctx: &mut C, ptr: usize) -> i64 {
    ctx.read_i64(ptr + 8)
}

fn arr_cap<C: HostCtx>(ctx: &mut C, ptr: usize) -> i64 {
    ctx.read_i64(ptr)
}

fn arr_elem<C: HostCtx>(ctx: &mut C, ptr: usize, idx: usize, es: usize) -> i64 {
    let addr = ptr + 16 + idx * es;
    if es == 4 {
        ctx.read_i32(addr) as i64
    } else {
        ctx.read_i64(addr)
    }
}

fn arr_set<C: HostCtx>(ctx: &mut C, ptr: usize, idx: usize, es: usize, v: i64) {
    let addr = ptr + 16 + idx * es;
    if es == 4 {
        ctx.write_i32(addr, v as i32);
    } else {
        ctx.write_i64(addr, v);
    }
}

fn arr_realloc<C: HostCtx>(ctx: &mut C, ptr: usize, new_cap: usize, es: usize) -> usize {
    let len = arr_len(ctx, ptr) as usize;
    let size = (new_cap * es + 16) as i64;
    let new_ptr = ctx.alloc(size) as usize;
    ctx.write_i64(new_ptr, new_cap as i64);
    ctx.write_i64(new_ptr + 8, len as i64);
    // Copiar el bloque de elementos (bounds-checked vía read/write).
    for i in 0..len {
        let e = arr_elem(ctx, ptr, i, es);
        arr_set(ctx, new_ptr, i, es, e);
    }
    new_ptr
}

pub fn host_arr_push<C: HostCtx>(ctx: &mut C, ptr: i64, val: i64, es: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    let cap = arr_cap(ctx, p);
    let new_p = if len + 1 > cap {
        arr_realloc(ctx, p, ((cap * 2 + 1).max(len + 1)) as usize, es as usize)
    } else {
        p
    };
    arr_set(ctx, new_p, len as usize, es as usize, val);
    ctx.write_i64(new_p + 8, len + 1);
    new_p as i64
}

pub fn host_arr_pop<C: HostCtx>(ctx: &mut C, ptr: i64, _es: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    if len <= 0 {
        return p as i64;
    }
    ctx.write_i64(p + 8, len - 1);
    p as i64
}

pub fn host_arr_shift<C: HostCtx>(ctx: &mut C, ptr: i64, es: i64) -> i64 {
    let p = ptr as usize;
    let es = es as usize;
    let len = arr_len(ctx, p);
    if len <= 0 {
        return p as i64;
    }
    for i in 0..(len - 1) as usize {
        let e = arr_elem(ctx, p, i + 1, es);
        arr_set(ctx, p, i, es, e);
    }
    ctx.write_i64(p + 8, len - 1);
    p as i64
}

pub fn host_arr_unshift<C: HostCtx>(ctx: &mut C, ptr: i64, val: i64, es: i64) -> i64 {
    let p = ptr as usize;
    let es = es as usize;
    let len = arr_len(ctx, p);
    let cap = arr_cap(ctx, p);
    let new_p = if len + 1 > cap {
        arr_realloc(ctx, p, ((cap * 2 + 1).max(len + 1)) as usize, es)
    } else {
        p
    };
    for i in (0..len as usize).rev() {
        let e = arr_elem(ctx, new_p, i, es);
        arr_set(ctx, new_p, i + 1, es, e);
    }
    arr_set(ctx, new_p, 0, es, val);
    ctx.write_i64(new_p + 8, len + 1);
    new_p as i64
}

pub fn host_arr_reverse<C: HostCtx>(ctx: &mut C, ptr: i64, es: i64) -> i64 {
    let p = ptr as usize;
    let es = es as usize;
    let len = arr_len(ctx, p);
    for i in 0..(len as usize / 2) {
        let a = arr_elem(ctx, p, i, es);
        let b = arr_elem(ctx, p, (len as usize) - 1 - i, es);
        arr_set(ctx, p, i, es, b);
        arr_set(ctx, p, (len as usize) - 1 - i, es, a);
    }
    p as i64
}

pub fn host_arr_to_string<C: HostCtx>(ctx: &mut C, ptr: i64, es: i64, kind: i64) -> i64 {
    let s = arr_to_string(ctx, ptr, es, kind);
    ctx.write_str(&s)
}

pub fn host_arr_index_of<C: HostCtx>(ctx: &mut C, ptr: i64, needle: i64, es: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    for i in 0..len as usize {
        if arr_elem(ctx, p, i, es as usize) == needle {
            return i as i64;
        }
    }
    -1
}

pub fn host_arr_includes<C: HostCtx>(ctx: &mut C, ptr: i64, needle: i64, es: i64) -> i32 {
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    for i in 0..len as usize {
        if arr_elem(ctx, p, i, es as usize) == needle {
            return 1;
        }
    }
    0
}

pub fn host_arr_join<C: HostCtx>(ctx: &mut C, ptr: i64, sep: i64, es: i64, kind: i64) -> i64 {
    let p = ptr as usize;
    let es = es as usize;
    let len = arr_len(ctx, p);
    let separator = ctx.read_str(sep);
    let mut out = String::new();
    for i in 0..len as usize {
        if i > 0 {
            out.push_str(&separator);
        }
        let e = arr_elem(ctx, p, i, es);
        match kind {
            1 => out.push_str(&ctx.read_str(e)),
            2 => out.push_str(&format_float(f64::from_bits(e as u64))),
            3 => out.push_str(if e != 0 { "true" } else { "false" }),
            4 => out.push(char::from_u32(e as u32).unwrap_or('?')),
            _ => out.push_str(&e.to_string()),
        }
    }
    ctx.write_str(&out)
}

// ── stdlib: math ────────────────────────────────────────────────────────────

pub fn host_math_sqrt<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.sqrt()
}

pub fn host_math_pow<C: HostCtx>(_ctx: &mut C, a: f64, b: f64) -> f64 {
    a.powf(b)
}

pub fn host_math_min<C: HostCtx>(_ctx: &mut C, a: f64, b: f64) -> f64 {
    a.min(b)
}

pub fn host_math_max<C: HostCtx>(_ctx: &mut C, a: f64, b: f64) -> f64 {
    a.max(b)
}

pub fn host_math_floor<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.floor()
}

pub fn host_math_ceil<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.ceil()
}

pub fn host_math_round<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.round()
}

pub fn host_math_sin<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.sin()
}

pub fn host_math_cos<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.cos()
}

pub fn host_math_tan<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.tan()
}

pub fn host_math_log<C: HostCtx>(_ctx: &mut C, v: f64) -> f64 {
    v.ln()
}

/// LCG compartido: se siembra UNA vez con entropía del sistema. Lo usan
/// `math.random` y el módulo `random`.
fn rng_state() -> &'static std::sync::Mutex<u64> {
    use std::sync::OnceLock;
    static RNG_STATE: OnceLock<std::sync::Mutex<u64>> = OnceLock::new();
    RNG_STATE.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seed = (nanos as u64) ^ ((std::process::id() as u64) << 32);
        std::sync::Mutex::new(seed | 1)
    })
}

/// Siguiente valor del LCG (u64) — avanza el estado compartido.
fn rng_next_u64() -> u64 {
    let mut s = rng_state().lock().unwrap();
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *s
}

/// Float en [0, 1) desde el LCG compartido.
fn rng_next_f64() -> f64 {
    (rng_next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

pub fn host_math_random<C: HostCtx>(_ctx: &mut C) -> f64 {
    rng_next_f64()
}

pub fn host_math_range<C: HostCtx>(ctx: &mut C, a: i64, b: i64) -> i64 {
    let n = (b - a).max(0);
    let size = (n * 8 + 16) as i64;
    let ptr = ctx.alloc(size) as usize;
    ctx.write_i64(ptr, n);
    ctx.write_i64(ptr + 8, n);
    for i in 0..n {
        arr_set(ctx, ptr, i as usize, 8, a + i);
    }
    ptr as i64
}

pub fn host_pow_num<C: HostCtx>(_ctx: &mut C, a: i64, b: i64) -> i64 {
    if b == 0 {
        1
    } else {
        (a as f64).powi(b as i32) as i64
    }
}

pub fn host_fmod<C: HostCtx>(_ctx: &mut C, a: f64, b: f64) -> f64 {
    a % b
}

pub fn host_input<C: HostCtx>(ctx: &mut C) -> i64 {
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    let line = line.trim_end_matches(['\r', '\n']);
    ctx.write_str(line)
}

// ── stdlib: json ────────────────────────────────────────────────────────────

/// tags de tipo para JSON: 0=int, 1=string, 2=float, 3=bool, 4=char, 5=array, 6=record.
fn json_build<C: HostCtx>(ctx: &mut C, v: &serde_json::Value) -> (i64, i64) {
    // Tags CLS estándar (paridad con fmt_val_to_string/tag_type):
    // 1=string, 2=float, 3=bool, 4=char, 6=array, 7=record.
    match v {
        serde_json::Value::Null => (0, 0),
        serde_json::Value::Bool(b) => ((if *b { 1 } else { 0 }), 3),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                (i, 0)
            } else if let Some(f) = n.as_f64() {
                (f.to_bits() as i64, 2)
            } else {
                (0, 0)
            }
        }
        serde_json::Value::String(s) => (ctx.write_str(s), 1),
        serde_json::Value::Array(items) => {
            let n = items.len();
            // Entradas `[val, tag]` stride 16 (como los arrays de Cmx).
            let ptr = ctx.alloc((n * 16 + 16) as i64) as usize;
            ctx.write_i64(ptr, n as i64);
            ctx.write_i64(ptr + 8, n as i64);
            for (i, it) in items.iter().enumerate() {
                let (val, tag) = json_build(ctx, it);
                ctx.write_i64(ptr + 16 + i * 16, val);
                ctx.write_i64(ptr + 16 + i * 16 + 8, tag);
            }
            (ptr as i64, 6)
        }
        serde_json::Value::Object(map) => {
            let n = map.len();
            let ptr = ctx.alloc((n * 24 + 16) as i64) as usize;
            ctx.write_i64(ptr, n as i64);
            ctx.write_i64(ptr + 8, n as i64);
            let mut i = 0;
            for (k, val) in map {
                let key = ctx.write_str(k);
                let (vv, tag) = json_build(ctx, val);
                ctx.write_i64(ptr + 16 + i * 24, key);
                ctx.write_i64(ptr + 16 + i * 24 + 8, vv);
                ctx.write_i64(ptr + 16 + i * 24 + 16, tag);
                i += 1;
            }
            (ptr as i64, 7)
        }
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn json_serialize_val<C: HostCtx>(ctx: &mut C, val: i64, tag: i64, out: &mut String) {
    match tag {
        1 => {
            out.push('"');
            out.push_str(&json_escape(&ctx.read_str(val)));
            out.push('"');
        }
        2 => out.push_str(&format_float(f64::from_bits(val as u64))),
        3 => out.push_str(if val != 0 { "true" } else { "false" }),
        4 => out.push(char::from_u32(val as u32).unwrap_or('?')),
        5 => json_serialize_array(ctx, val, out),
        6 => json_serialize_record(ctx, val, out),
        _ => out.push_str(&val.to_string()),
    }
}

fn json_serialize_record<C: HostCtx>(ctx: &mut C, ptr: i64, out: &mut String) {
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    out.push('{');
    for i in 0..len as usize {
        if i > 0 {
            out.push(',');
        }
        let key = ctx.read_i64(p + 16 + i * 24);
        let val = ctx.read_i64(p + 16 + i * 24 + 8);
        let tag = ctx.read_i64(p + 16 + i * 24 + 16);
        out.push('"');
        out.push_str(&json_escape(&ctx.read_str(key)));
        out.push_str("\":");
        json_serialize_val(ctx, val, tag, out);
    }
    out.push('}');
}

fn json_serialize_array<C: HostCtx>(ctx: &mut C, ptr: i64, out: &mut String) {
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    out.push('[');
    for i in 0..len as usize {
        if i > 0 {
            out.push(',');
        }
        let val = arr_elem(ctx, p, i, 8);
        json_serialize_val(ctx, val, 0, out);
    }
    out.push(']');
}

pub fn host_json_stringify<C: HostCtx>(ctx: &mut C, v: i64, kind: i64) -> i64 {
    match kind {
        1 => {
            let mut out = String::new();
            json_serialize_record(ctx, v, &mut out);
            ctx.write_str(&out)
        }
        2 => {
            let mut out = String::new();
            json_serialize_array(ctx, v, &mut out);
            ctx.write_str(&out)
        }
        _ => v,
    }
}

pub fn host_json_parse<C: HostCtx>(ctx: &mut C, s: i64) -> i64 {
    let text = ctx.read_str(s);
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(serde_json::Value::Array(items)) => {
            let n = items.len();
            let ptr = ctx.alloc((n * 24 + 16) as i64) as usize;
            ctx.write_i64(ptr, n as i64);
            ctx.write_i64(ptr + 8, n as i64);
            for (i, it) in items.iter().enumerate() {
                let key = ctx.write_str(&i.to_string());
                let (vv, tag) = json_build(ctx, it);
                ctx.write_i64(ptr + 16 + i * 24, key);
                ctx.write_i64(ptr + 16 + i * 24 + 8, vv);
                ctx.write_i64(ptr + 16 + i * 24 + 16, tag);
            }
            ptr as i64
        }
        Ok(v) => json_build(ctx, &v).0,
        Err(_) => 0,
    }
}

// ── fs (nodo desktop; en browser se deshabilitan) ───────────────────────────

pub fn host_fs_exists<C: HostCtx>(ctx: &mut C, p: i64) -> i32 {
    let s = ctx.read_str(p);
    if std::path::Path::new(&s).exists() {
        1
    } else {
        0
    }
}

pub fn host_fs_cwd<C: HostCtx>(ctx: &mut C) -> i64 {
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    ctx.write_str(&cwd)
}

pub fn host_fs_read_file<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    match std::fs::read_to_string(&s) {
        Ok(contents) => ctx.write_str(&contents),
        Err(_) => 0,
    }
}

pub fn host_fs_write_file<C: HostCtx>(ctx: &mut C, p: i64, d: i64) -> i64 {
    let path = ctx.read_str(p);
    let data = ctx.read_str(d);
    let _ = std::fs::write(&path, data);
    0
}

pub fn host_fs_list_dir<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let names: Vec<String> = std::fs::read_dir(&s)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    // Array<String> en memoria: header [cap:i64][len:i64] + elems packed.
    let n = names.len() as i64;
    let array_ptr = ctx.alloc(n * 8 + 16);
    if array_ptr == 0 {
        return 0;
    }
    ctx.write_i64(array_ptr as usize, n);
    ctx.write_i64(array_ptr as usize + 8, n);
    for (i, name) in names.iter().enumerate() {
        let sp = ctx.write_str(name);
        ctx.write_i64(array_ptr as usize + 16 + i * 8, sp);
    }
    array_ptr
}

pub fn host_fs_mkdir<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let _ = std::fs::create_dir_all(&s);
    0
}

pub fn host_fs_rm<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let _ = std::fs::remove_file(&s);
    0
}

// ── http ────────────────────────────────────────────────────────────────────

pub fn host_http_get<C: HostCtx>(ctx: &mut C, url: i64) -> i64 {
    let u = ctx.read_str(url);
    match ureq::get(&u).call() {
        Ok(resp) => match resp.into_string() {
            Ok(body) => ctx.write_str(&body),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

pub fn host_http_post<C: HostCtx>(ctx: &mut C, url: i64, data: i64) -> i64 {
    let u = ctx.read_str(url);
    let d = ctx.read_str(data);
    match ureq::post(&u).send_string(&d) {
        Ok(resp) => match resp.into_string() {
            Ok(body) => ctx.write_str(&body),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

// ── os (sistema y entorno) ───────────────────────────────────────────────────

pub fn host_os_platform<C: HostCtx>(ctx: &mut C) -> i64 {
    let p = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        other => other,
    };
    ctx.write_str(p)
}

pub fn host_os_arch<C: HostCtx>(ctx: &mut C) -> i64 {
    ctx.write_str(std::env::consts::ARCH)
}

pub fn host_os_version<C: HostCtx>(ctx: &mut C) -> i64 {
    #[cfg(windows)]
    {
        // Sin crates: reportar windows-<mayor>.<menor> aproximado desde la
        // variable de entorno del sistema si existe; si no, cadena vacía.
        let v = std::env::var("OS").unwrap_or_default();
        if !v.is_empty() {
            return ctx.write_str(&v);
        }
        ctx.write_str("")
    }
    #[cfg(not(windows))]
    {
        // Unix: sin libc disponible, reportar el OS (documentado como aprox.).
        ctx.write_str(std::env::consts::OS)
    }
}

pub fn host_os_hostname<C: HostCtx>(ctx: &mut C) -> i64 {
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    ctx.write_str(&name)
}

pub fn host_os_home<C: HostCtx>(ctx: &mut C) -> i64 {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    ctx.write_str(&home)
}

pub fn host_os_tempdir<C: HostCtx>(ctx: &mut C) -> i64 {
    ctx.write_str(&std::env::temp_dir().to_string_lossy())
}

pub fn host_os_cpus<C: HostCtx>(_ctx: &mut C) -> i64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

pub fn host_os_pid<C: HostCtx>(_ctx: &mut C) -> i64 {
    std::process::id() as i64
}

pub fn host_os_uptime<C: HostCtx>(_ctx: &mut C) -> i64 {
    // Sin crates de sysinfo: uptime real no disponible portablemente.
    // Documentado: devuelve 0 (mejora futura con sysinfo/windows-sys).
    0
}

pub fn host_os_env<C: HostCtx>(ctx: &mut C, key: i64) -> i64 {
    let k = ctx.read_str(key);
    let v = std::env::var(&k).unwrap_or_default();
    ctx.write_str(&v)
}

pub fn host_os_sep<C: HostCtx>(ctx: &mut C) -> i64 {
    ctx.write_str(std::path::MAIN_SEPARATOR_STR)
}

pub fn host_os_is_windows<C: HostCtx>(_ctx: &mut C) -> i32 {
    if cfg!(windows) {
        1
    } else {
        0
    }
}

pub fn host_os_is_unix<C: HostCtx>(_ctx: &mut C) -> i32 {
    if cfg!(windows) {
        0
    } else {
        1
    }
}

// ── path (rutas de archivos) ─────────────────────────────────────────────────

pub fn host_path_join<C: HostCtx>(ctx: &mut C, a: i64, b: i64) -> i64 {
    let sa = ctx.read_str(a);
    let sb = ctx.read_str(b);
    let joined = std::path::Path::new(&sa).join(&sb);
    ctx.write_str(&joined.to_string_lossy())
}

pub fn host_path_basename<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let base = std::path::Path::new(&s)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    ctx.write_str(&base)
}

pub fn host_path_dirname<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let dir = std::path::Path::new(&s)
        .parent()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());
    ctx.write_str(&dir)
}

pub fn host_path_extname<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let ext = std::path::Path::new(&s)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    ctx.write_str(&ext)
}

pub fn host_path_resolve<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    let path = std::path::Path::new(&s);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    ctx.write_str(&resolved.to_string_lossy())
}

pub fn host_path_normalize<C: HostCtx>(ctx: &mut C, p: i64) -> i64 {
    let s = ctx.read_str(p);
    // Aceptar ambos separadores (/ y \) en el input; normalizar con el nativo.
    let mut parts: Vec<&str> = Vec::new();
    for comp in s.split(['/', '\\']) {
        match comp {
            "" | "." => {}
            ".." => {
                if let Some(last) = parts.last() {
                    if *last != ".." {
                        parts.pop();
                        continue;
                    }
                }
                parts.push(comp);
            }
            other => parts.push(other),
        }
    }
    let mut out = String::new();
    if s.starts_with('/') || s.starts_with('\\') {
        out.push(std::path::MAIN_SEPARATOR);
    }
    out.push_str(&parts.join(&std::path::MAIN_SEPARATOR.to_string()));
    if out.is_empty() {
        out.push('.');
    }
    ctx.write_str(&out)
}

pub fn host_path_is_absolute<C: HostCtx>(ctx: &mut C, p: i64) -> i32 {
    let s = ctx.read_str(p);
    if std::path::Path::new(&s).is_absolute() {
        1
    } else {
        0
    }
}

pub fn host_path_sep<C: HostCtx>(ctx: &mut C) -> i64 {
    ctx.write_str(std::path::MAIN_SEPARATOR_STR)
}

// ── process (proceso actual) ─────────────────────────────────────────────────

pub fn host_process_args<C: HostCtx>(ctx: &mut C) -> i64 {
    let names = ctx.state().app_args.clone();
    let n = names.len() as i64;
    let array_ptr = ctx.alloc(n * 8 + 16);
    if array_ptr == 0 {
        return 0;
    }
    ctx.write_i64(array_ptr as usize, n);
    ctx.write_i64(array_ptr as usize + 8, n);
    for (i, name) in names.iter().enumerate() {
        let sp = ctx.write_str(name);
        ctx.write_i64(array_ptr as usize + 16 + i * 8, sp);
    }
    array_ptr
}

pub fn host_process_cwd<C: HostCtx>(ctx: &mut C) -> i64 {
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    ctx.write_str(&cwd)
}

pub fn host_process_env<C: HostCtx>(ctx: &mut C, key: i64) -> i64 {
    let k = ctx.read_str(key);
    let v = std::env::var(&k).unwrap_or_default();
    ctx.write_str(&v)
}

pub fn host_process_exit<C: HostCtx>(_ctx: &mut C, code: i64) {
    std::process::exit(code as i32);
}

pub fn host_process_pid<C: HostCtx>(_ctx: &mut C) -> i64 {
    std::process::id() as i64
}

pub fn host_process_platform<C: HostCtx>(ctx: &mut C) -> i64 {
    ctx.write_str(std::env::consts::OS)
}

pub fn host_process_title<C: HostCtx>(ctx: &mut C) -> i64 {
    // Sin crates: título del proceso no portable (Windows GetConsoleTitle
    // requiere windows-sys). Documentado: cadena vacía.
    let _ = ctx;
    ctx.write_str("")
}

// ── time (fechas y hora; UTC sin crates) ─────────────────────────────────────

/// Descompone un epoch (segundos) en fecha/hora UTC.
fn epoch_fields(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    // Algoritmo civil (Howard Hinnant): días desde 1970-01-01 → (y, m, d).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, hour, minute, second)
}

fn pad2(n: i64) -> String {
    if n < 10 {
        format!("0{}", n)
    } else {
        n.to_string()
    }
}

pub fn host_time_now<C: HostCtx>(_ctx: &mut C) -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn host_time_seconds<C: HostCtx>(_ctx: &mut C) -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn host_time_sleep<C: HostCtx>(_ctx: &mut C, ms: i64) {
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

pub fn host_time_iso<C: HostCtx>(ctx: &mut C) -> i64 {
    let secs = host_time_seconds(ctx);
    let (y, mo, d, h, mi, s) = epoch_fields(secs);
    ctx.write_str(&format!(
        "{:04}-{}-{}T{}:{}:{}Z",
        y,
        pad2(mo),
        pad2(d),
        pad2(h),
        pad2(mi),
        pad2(s)
    ))
}

pub fn host_time_date<C: HostCtx>(ctx: &mut C) -> i64 {
    let secs = host_time_seconds(ctx);
    let (y, mo, d, _, _, _) = epoch_fields(secs);
    ctx.write_str(&format!("{:04}-{}-{}", y, pad2(mo), pad2(d)))
}

pub fn host_time_clock<C: HostCtx>(ctx: &mut C) -> i64 {
    let secs = host_time_seconds(ctx);
    let (_, _, _, h, mi, s) = epoch_fields(secs);
    ctx.write_str(&format!("{}:{}:{}", pad2(h), pad2(mi), pad2(s)))
}

pub fn host_time_year<C: HostCtx>(ctx: &mut C) -> i64 {
    epoch_fields(host_time_seconds(ctx)).0
}

pub fn host_time_month<C: HostCtx>(ctx: &mut C) -> i64 {
    epoch_fields(host_time_seconds(ctx)).1
}

pub fn host_time_day<C: HostCtx>(ctx: &mut C) -> i64 {
    epoch_fields(host_time_seconds(ctx)).2
}

pub fn host_time_hour<C: HostCtx>(ctx: &mut C) -> i64 {
    epoch_fields(host_time_seconds(ctx)).3
}

pub fn host_time_minute<C: HostCtx>(ctx: &mut C) -> i64 {
    epoch_fields(host_time_seconds(ctx)).4
}

pub fn host_time_second<C: HostCtx>(ctx: &mut C) -> i64 {
    epoch_fields(host_time_seconds(ctx)).5
}

// ── random (aleatoriedad) ────────────────────────────────────────────────────

pub fn host_random_random<C: HostCtx>(_ctx: &mut C) -> f64 {
    rng_next_f64()
}

pub fn host_random_int<C: HostCtx>(_ctx: &mut C, min: i64, max: i64) -> i64 {
    if max <= min {
        return min;
    }
    let range = (max - min + 1) as u64;
    min + (rng_next_u64() % range) as i64
}

pub fn host_random_float<C: HostCtx>(_ctx: &mut C, min: f64, max: f64) -> f64 {
    min + rng_next_f64() * (max - min)
}

pub fn host_random_uuid<C: HostCtx>(ctx: &mut C) -> i64 {
    let mut b = [0u8; 16];
    for chunk in b.chunks_mut(8) {
        let v = rng_next_u64().to_le_bytes();
        chunk.copy_from_slice(&v[..chunk.len()]);
    }
    b[6] = (b[6] & 0x0f) | 0x40; // versión 4
    b[8] = (b[8] & 0x3f) | 0x80; // variante 10xx
    let h = b.iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>();
    ctx.write_str(&format!(
        "{}-{}-{}-{}-{}",
        h[0..4].concat(),
        h[4..6].concat(),
        h[6..8].concat(),
        h[8..10].concat(),
        h[10..16].concat()
    ))
}

// ── records ─────────────────────────────────────────────────────────────────

pub fn host_record_new<C: HostCtx>(ctx: &mut C, cap: i64) -> i64 {
    let size = cap * 24 + 16;
    let ptr = ctx.alloc(size) as usize;
    ctx.write_i64(ptr, cap);
    ctx.write_i64(ptr + 8, 0);
    ptr as i64
}

pub fn host_record_set<C: HostCtx>(ctx: &mut C, ptr: i64, key: i64, val: i64, tag: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p) as usize;
    let cap = arr_cap(ctx, p) as usize;
    let k = ctx.read_str(key);
    for i in 0..len {
        let ki = ctx.read_i64(p + 16 + i * 24);
        if ctx.read_str(ki) == k {
            ctx.write_i64(p + 16 + i * 24 + 8, val);
            ctx.write_i64(p + 16 + i * 24 + 16, tag);
            return p as i64;
        }
    }
    let mut new_p = p;
    if len >= cap {
        let new_cap = if cap == 0 { 4 } else { cap * 2 };
        let size = (new_cap * 24 + 16) as i64;
        let np = ctx.alloc(size) as usize;
        ctx.write_i64(np, new_cap as i64);
        ctx.write_i64(np + 8, len as i64);
        for i in 0..len {
            let kk = ctx.read_i64(p + 16 + i * 24);
            let vv = ctx.read_i64(p + 16 + i * 24 + 8);
            let tt = ctx.read_i64(p + 16 + i * 24 + 16);
            ctx.write_i64(np + 16 + i * 24, kk);
            ctx.write_i64(np + 16 + i * 24 + 8, vv);
            ctx.write_i64(np + 16 + i * 24 + 16, tt);
        }
        new_p = np;
    }
    ctx.write_i64(new_p + 16 + len * 24, key);
    ctx.write_i64(new_p + 16 + len * 24 + 8, val);
    ctx.write_i64(new_p + 16 + len * 24 + 16, tag);
    ctx.write_i64(new_p + 8, (len + 1) as i64);
    new_p as i64
}

pub fn host_record_get<C: HostCtx>(ctx: &mut C, ptr: i64, key: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p) as usize;
    let k = ctx.read_str(key);
    for i in 0..len {
        let ki = ctx.read_i64(p + 16 + i * 24);
        if ctx.read_str(ki) == k {
            return ctx.read_i64(p + 16 + i * 24 + 8);
        }
    }
    0
}

pub fn host_record_has<C: HostCtx>(ctx: &mut C, ptr: i64, key: i64) -> i32 {
    let p = ptr as usize;
    let len = arr_len(ctx, p) as usize;
    let k = ctx.read_str(key);
    for i in 0..len {
        let ki = ctx.read_i64(p + 16 + i * 24);
        if ctx.read_str(ki) == k {
            return 1;
        }
    }
    0
}

pub fn host_record_tag<C: HostCtx>(ctx: &mut C, ptr: i64, key: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p) as usize;
    let k = ctx.read_str(key);
    for i in 0..len {
        let ki = ctx.read_i64(p + 16 + i * 24);
        if ctx.read_str(ki) == k {
            return ctx.read_i64(p + 16 + i * 24 + 16);
        }
    }
    0
}

pub fn host_record_len<C: HostCtx>(ctx: &mut C, ptr: i64) -> i64 {
    arr_len(ctx, ptr as usize)
}

pub fn host_record_keys<C: HostCtx>(ctx: &mut C, ptr: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p) as usize;
    let size = (len * 8 + 16) as i64;
    let out = ctx.alloc(size) as usize;
    ctx.write_i64(out, len as i64);
    ctx.write_i64(out + 8, len as i64);
    for i in 0..len {
        let ki = ctx.read_i64(p + 16 + i * 24);
        arr_set(ctx, out, i, 8, ki);
    }
    out as i64
}

pub fn host_record_values<C: HostCtx>(ctx: &mut C, ptr: i64) -> i64 {
    let p = ptr as usize;
    let len = arr_len(ctx, p) as usize;
    let size = (len * 8 + 16) as i64;
    let out = ctx.alloc(size) as usize;
    ctx.write_i64(out, len as i64);
    ctx.write_i64(out + 8, len as i64);
    for i in 0..len {
        let vi = ctx.read_i64(p + 16 + i * 24 + 8);
        arr_set(ctx, out, i, 8, vi);
    }
    out as i64
}

pub fn host_record_to_string<C: HostCtx>(ctx: &mut C, ptr: i64) -> i64 {
    let s = record_to_string(ctx, ptr);
    ctx.write_str(&s)
}

/// `any_member(val, tag, key)`: acceso a miembro de un valor `Any` en runtime
/// (JSON parse anidado, p.ej. `o.a.c`). Despacha por tag.
pub fn host_any_member<C: HostCtx>(ctx: &mut C, val: i64, tag: i64, key: i64) -> (i64, i64) {
    let t = tag_type(tag);
    match t {
        7 => {
            let p = val as usize;
            let len = arr_len(ctx, p) as usize;
            let k = ctx.read_str(key);
            for i in 0..len {
                let ki = ctx.read_i64(p + 16 + i * 24);
                if ctx.read_str(ki) == k {
                    return (
                        ctx.read_i64(p + 16 + i * 24 + 8),
                        ctx.read_i64(p + 16 + i * 24 + 16),
                    );
                }
            }
            (0, 0)
        }
        6 => {
            let k = ctx.read_str(key);
            if k == "length" || k == "size" {
                (arr_len(ctx, val as usize), 0)
            } else {
                (0, 0)
            }
        }
        1 => {
            let k = ctx.read_str(key);
            if k == "length" || k == "size" {
                ((val & 0xffff_ffff), 0)
            } else {
                (0, 0)
            }
        }
        _ => (0, 0),
    }
}

/// `any_index(val, tag, idx)`: indexar un valor `Any` en runtime.
pub fn host_any_index<C: HostCtx>(ctx: &mut C, val: i64, tag: i64, idx: i64) -> (i64, i64) {
    let t = tag_type(tag);
    match t {
        6 => {
            // Array de JSON heterogéneo: entradas [val, tag] stride 16.
            let p = val as usize;
            let len = arr_len(ctx, p);
            if idx >= 0 && idx < len {
                let i = idx as usize;
                (
                    ctx.read_i64(p + 16 + i * 16),
                    ctx.read_i64(p + 16 + i * 16 + 8),
                )
            } else {
                (0, 0)
            }
        }
        _ => (0, 0),
    }
}

// ── CMX ─────────────────────────────────────────────────────────────────────

pub fn host_cmx_new<C: HostCtx>(ctx: &mut C, tag: i64, kind: i64) -> i64 {
    // layout: [tag][props_ptr][children_ptr][kind] (kind 0=elemento, 1=texto)
    let ptr = ctx.alloc(32) as usize;
    ctx.write_i64(ptr, tag);
    ctx.write_i64(ptr + 8, 0);
    ctx.write_i64(ptr + 16, 0);
    ctx.write_i64(ptr + 24, kind);
    ptr as i64
}

pub fn host_cmx_set_prop<C: HostCtx>(ctx: &mut C, ptr: i64, key: i64, val: i64, tag: i64) -> i64 {
    let p = ptr as usize;
    let mut props = ctx.read_i64(p + 8) as usize;
    if props == 0 {
        let np = ctx.alloc(4 * 24 + 16) as usize;
        ctx.write_i64(np, 4);
        ctx.write_i64(np + 8, 0);
        ctx.write_i64(p + 8, np as i64);
        props = np;
    }
    let pr = props as i64;
    let len = arr_len(ctx, props) as usize;
    let cap = arr_cap(ctx, props) as usize;
    let k = ctx.read_str(key);
    for i in 0..len {
        let ki = ctx.read_i64(props + 16 + i * 24);
        if ctx.read_str(ki) == k {
            ctx.write_i64(props + 16 + i * 24 + 8, val);
            ctx.write_i64(props + 16 + i * 24 + 16, tag);
            return pr;
        }
    }
    if len + 1 > cap {
        let new_cap = if cap == 0 { 4 } else { cap * 2 };
        let new_cap = new_cap.max(len + 1);
        let np = ctx.alloc((new_cap * 24 + 16) as i64) as usize;
        ctx.write_i64(np, new_cap as i64);
        ctx.write_i64(np + 8, len as i64);
        for i in 0..len {
            let kk = ctx.read_i64(props + 16 + i * 24);
            let vv = ctx.read_i64(props + 16 + i * 24 + 8);
            let tt = ctx.read_i64(props + 16 + i * 24 + 16);
            ctx.write_i64(np + 16 + i * 24, kk);
            ctx.write_i64(np + 16 + i * 24 + 8, vv);
            ctx.write_i64(np + 16 + i * 24 + 16, tt);
        }
        props = np;
        ctx.write_i64(p + 8, props as i64);
    }
    ctx.write_i64(props + 16 + len * 24, key);
    ctx.write_i64(props + 16 + len * 24 + 8, val);
    ctx.write_i64(props + 16 + len * 24 + 16, tag);
    ctx.write_i64(props + 8, (len + 1) as i64);
    ctx.read_i64(p + 8)
}

pub fn host_cmx_add_child<C: HostCtx>(ctx: &mut C, ptr: i64, val: i64, tag: i64) -> i64 {
    let p = ptr as usize;
    let mut children = ctx.read_i64(p + 16) as usize;
    if children == 0 {
        let nc = ctx.alloc(4 * 16 + 16) as usize;
        ctx.write_i64(nc, 4);
        ctx.write_i64(nc + 8, 0);
        ctx.write_i64(p + 16, nc as i64);
        children = nc;
    }
    let len = arr_len(ctx, children) as usize;
    let cap = arr_cap(ctx, children) as usize;
    if len + 1 > cap {
        let new_cap = if cap == 0 { 4 } else { cap * 2 };
        let new_cap = new_cap.max(len + 1);
        let np = ctx.alloc((new_cap * 16 + 16) as i64) as usize;
        ctx.write_i64(np, new_cap as i64);
        ctx.write_i64(np + 8, len as i64);
        for i in 0..len {
            let v = ctx.read_i64(children + 16 + i * 16);
            let t = ctx.read_i64(children + 16 + i * 16 + 8);
            ctx.write_i64(np + 16 + i * 16, v);
            ctx.write_i64(np + 16 + i * 16 + 8, t);
        }
        children = np;
        ctx.write_i64(p + 16, children as i64);
    }
    ctx.write_i64(children + 16 + len * 16, val);
    ctx.write_i64(children + 16 + len * 16 + 8, tag);
    ctx.write_i64(children + 8, (len + 1) as i64);
    ctx.read_i64(p + 16)
}

pub fn host_cmx_to_string<C: HostCtx>(ctx: &mut C, ptr: i64) -> i64 {
    let s = cmx_format(ctx, ptr as usize);
    ctx.write_str(&s)
}

// ── funciones como valor + shadow call stack ────────────────────────────────

pub fn host_fn_handle<C: HostCtx>(ctx: &mut C, table_idx: i64, nombre: i64, capturas: i64) -> i64 {
    // Contrato con el backend (dispatch tag-bit en wasm.rs):
    //  - capturas == 0 → handle PAR = (tabla_idx << 1): sin alocar.
    //  - capturas != 0 → handle IMPAR = (ptr << 1) | 1: bloque de 24B.
    if capturas == 0 {
        if nombre != 0 {
            let name = ctx.read_str(nombre);
            ctx.state_mut().simple_fn_names.insert(table_idx, name);
        }
        return table_idx << 1;
    }
    let ptr = ctx.alloc(24) as usize;
    ctx.write_i64(ptr, table_idx);
    ctx.write_i64(ptr + 8, capturas);
    ctx.write_i64(ptr + 16, nombre);
    ((ptr as i64) << 1) | 1
}

pub fn host_fn_to_string<C: HostCtx>(ctx: &mut C, handle: i64) -> i64 {
    let s = fn_to_string_str(ctx, handle);
    ctx.write_str(&s)
}

pub fn host_fn_enter<C: HostCtx>(ctx: &mut C, name_packed: i64, line: i64, col: i64) {
    let name = ctx.read_str(name_packed);
    let state = ctx.state_mut();
    // Si el backend emitó un call site pendiente (justo antes del Call), usarlo
    // como span del frame: el frame apunta al CALL SITE del llamador.
    let span = state
        .pending_call_site
        .take()
        .unwrap_or_else(|| {
            cls_core::error::diagnostic::Span::new(
                line as u32,
                col as u32,
                line as u32,
                col as u32,
            )
        });
    if state.call_stack.len() < 1000 {
        state.call_stack.push((name, span));
    }
}

pub fn host_fn_exit<C: HostCtx>(ctx: &mut C) {
    ctx.state_mut().call_stack.pop();
}

pub fn host_fn_call_site<C: HostCtx>(ctx: &mut C, line: i64, col: i64) {
    ctx.state_mut().pending_call_site = Some(cls_core::error::diagnostic::Span::new(
        line as u32,
        col as u32,
        line as u32,
        col as u32,
    ));
}

// ── formateo compartido (arr/record/cmx a string) ───────────────────────────

/// Nombre de una función desde un handle/valor con tag-bit.
pub(crate) fn fn_to_string_str<C: HostCtx>(ctx: &mut C, handle: i64) -> String {
    if (handle & 1) == 1 {
        let name_addr = ctx.read_i64(((handle >> 1) as usize) + 16);
        ctx.read_str(name_addr)
    } else {
        ctx.state()
            .simple_fn_names
            .get(&(handle >> 1))
            .cloned()
            .unwrap_or_else(|| format!("<function {}>", handle >> 1))
    }
}

/// Tipo de un tag: compuesto (`tipo<<8 | kind`) o legacy (0-5, arr_kind_code).
fn tag_type(tag: i64) -> i32 {
    if tag >= 256 {
        (tag >> 8) as i32
    } else {
        tag as i32
    }
}

/// Formatea un valor según su tag. Tag = `tipo<<8 | kind`.
fn fmt_val_to_string<C: HostCtx>(ctx: &mut C, val: i64, tag: i64) -> String {
    let t = tag_type(tag);
    let kind = (tag & 0xff) as i32;
    match t {
        1 => ctx.read_str(val),
        2 => format_float(f64::from_bits(val as u64)),
        3 => {
            if val != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        4 => char::from_u32(val as u32).unwrap_or('?').to_string(),
        5 => cmx_format(ctx, val as usize),
        6 => {
            let es = if kind == 5 || kind == 6 { 16 } else { 8 };
            let arr_kind = if kind == 6 { 5 } else { kind };
            arr_to_string(ctx, val, es, arr_kind as i64)
        }
        7 => record_to_string(ctx, val),
        _ => val.to_string(),
    }
}

/// `[e1, e2, ...]` — kind 5 = array de Cmx (entradas `[val, tag]` stride 16).
fn arr_to_string<C: HostCtx>(ctx: &mut C, ptr: i64, es: i64, kind: i64) -> String {
    if ptr == 0 {
        return String::from("[]");
    }
    let p = ptr as usize;
    let es = es as usize;
    let len = arr_len(ctx, p);
    let mut out = String::from("[");
    for i in 0..len as usize {
        if i > 0 {
            out.push_str(", ");
        }
        let e = arr_elem(ctx, p, i, es);
        match kind {
            1 => {
                out.push('"');
                out.push_str(&json_escape(&ctx.read_str(e)));
                out.push('"');
            }
            2 => out.push_str(&format_float(f64::from_bits(e as u64))),
            3 => out.push_str(if e != 0 { "true" } else { "false" }),
            4 => out.push(char::from_u32(e as u32).unwrap_or('?')),
            5 => {
                let tg = ctx.read_i64(p + 16 + i * 16 + 8);
                let tv = tag_type(tg);
                if tv == 1 {
                    out.push('"');
                    out.push_str(&json_escape(&ctx.read_str(e)));
                    out.push('"');
                } else if tv == 5 {
                    let ck = ctx.read_i64((e as usize) + 24);
                    if ck == 1 {
                        let ctag = ctx.read_i64(e as usize);
                        out.push('"');
                        out.push_str(&json_escape(&ctx.read_str(ctag)));
                        out.push('"');
                    } else {
                        out.push_str(&cmx_format(ctx, e as usize));
                    }
                } else {
                    out.push_str(&fmt_val_to_string(ctx, e, tg));
                }
            }
            _ => out.push_str(&e.to_string()),
        }
    }
    out.push(']');
    out
}

/// `{k: v, ...}` — formatea cada valor por su tag (claves ordenadas, como el walker).
fn record_to_string<C: HostCtx>(ctx: &mut C, ptr: i64) -> String {
    if ptr == 0 {
        return String::from("{}");
    }
    let p = ptr as usize;
    let len = arr_len(ctx, p);
    let mut entries: Vec<(String, i64, i64)> = Vec::with_capacity(len as usize);
    for i in 0..len as usize {
        let key = ctx.read_i64(p + 16 + i * 24);
        let val = ctx.read_i64(p + 16 + i * 24 + 8);
        let tag = ctx.read_i64(p + 16 + i * 24 + 16);
        entries.push((ctx.read_str(key), val, tag));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::from("{");
    for (i, (key, val, tag)) in entries.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(key);
        out.push_str(": ");
        let t = tag_type(*tag);
        if t == 1 {
            out.push('"');
            out.push_str(&json_escape(&ctx.read_str(*val)));
            out.push('"');
        } else {
            out.push_str(&fmt_val_to_string(ctx, *val, *tag));
        }
    }
    out.push('}');
    out
}

/// Formatea un CmxValue. Un CmxValue de "texto" (kind=1) se muestra plano.
fn cmx_format<C: HostCtx>(ctx: &mut C, p: usize) -> String {
    let tag = ctx.read_i64(p);
    let props = ctx.read_i64(p + 8) as usize;
    let children = ctx.read_i64(p + 16) as usize;
    let kind = ctx.read_i64(p + 24);
    let nprops = if props != 0 { arr_len(ctx, props) as usize } else { 0 };
    let nchild = if children != 0 { arr_len(ctx, children) as usize } else { 0 };
    if kind == 1 {
        return ctx.read_str(tag);
    }
    let mut out = String::from("<");
    if tag >> 32 == 0 && tag != 0 {
        let fn_str = fn_to_string_str(ctx, tag);
        out.push_str(&fn_str);
    } else {
        out.push_str(&ctx.read_str(tag));
    }
    let mut prop_entries: Vec<(String, i64, i64)> = Vec::with_capacity(nprops);
    for i in 0..nprops {
        let key = ctx.read_i64(props + 16 + i * 24);
        let val = ctx.read_i64(props + 16 + i * 24 + 8);
        let t = ctx.read_i64(props + 16 + i * 24 + 16);
        prop_entries.push((ctx.read_str(key), val, t));
    }
    prop_entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, val, t) in prop_entries {
        out.push(' ');
        out.push_str(&key);
        out.push_str("=\"");
        let tv = tag_type(t);
        if tv == 1 {
            out.push_str(&ctx.read_str(val));
        } else {
            out.push_str(&fmt_val_to_string(ctx, val, t));
        }
        out.push('"');
    }
    if nchild == 0 {
        out.push_str(" />");
    } else {
        out.push_str(">... (");
        out.push_str(&nchild.to_string());
        out.push_str(" children)</");
        if tag >> 32 == 0 && tag != 0 {
            let fn_str = fn_to_string_str(ctx, tag);
            out.push_str(&fn_str);
        } else {
            out.push_str(&ctx.read_str(tag));
        }
        out.push('>');
    }
    out
}
