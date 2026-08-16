//! Valores CLS en la frontera del host + marshalling a/desde la memoria lineal.
//!
//! [`StoreCtx`] implementa el trait `HostCtx` de cls-jit sobre el par
//! (Store + Memory) de un módulo instanciado: con él, escribir un String/Array/
//! Record como parámetro o leer un retorno es idéntico a lo que hacen los host
//! functions internos.

use cls_jit::host::HostCtx;
use cls_jit::state::HostState;
use cls_jit::TypeDesc;
use wasmtime::{Memory, Store, TypedFunc};

/// Valor CLS en la frontera del host (clxb).
#[derive(Debug, Clone, PartialEq)]
pub enum ClsValue {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    Array(Vec<ClsValue>),
    Record(Vec<(String, ClsValue)>),
}

impl ClsValue {
    /// Kind CLS del valor (códigos de la custom section `clx:exports`).
    pub fn kind(&self) -> i64 {
        match self {
            ClsValue::Null => 12,
            ClsValue::Int(_) => 0,
            ClsValue::Float(_) => 1,
            ClsValue::Bool(_) => 2,
            ClsValue::Char(_) => 3,
            ClsValue::Str(_) => 4,
            ClsValue::Array(_) => 5,
            ClsValue::Record(_) => 6,
        }
    }
}

/// Contexto de memoria para un módulo instanciado: permite usar las funciones
/// del trait `HostCtx` (write_str, alloc, read/write) contra la memoria del
/// módulo, sin un Caller.
pub struct StoreCtx<'a> {
    pub store: &'a mut Store<HostState>,
    pub memory: Memory,
    pub alloc: &'a mut TypedFunc<i64, i64>,
}

impl HostCtx for StoreCtx<'_> {
    fn state(&self) -> &HostState {
        self.store.data()
    }

    fn state_mut(&mut self) -> &mut HostState {
        self.store.data_mut()
    }

    fn read_str(&mut self, packed: i64) -> String {
        let ptr = (packed >> 32) as usize;
        let len = (packed & 0xffff_ffff) as usize;
        let data = self.memory.data(&*self.store);
        if ptr + len <= data.len() {
            return String::from_utf8_lossy(&data[ptr..ptr + len]).into_owned();
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
        self.state_mut().string_caps.insert(ptr, cap);
        (ptr << 32) | len
    }

    fn alloc(&mut self, n: i64) -> i64 {
        self.alloc.call(&mut *self.store, n).unwrap_or(0)
    }

    fn read_i64(&mut self, addr: usize) -> i64 {
        let data = self.memory.data(&*self.store);
        if addr + 8 <= data.len() {
            return i64::from_le_bytes(data[addr..addr + 8].try_into().unwrap());
        }
        0
    }

    fn write_i64(&mut self, addr: usize, v: i64) {
        let data = self.memory.data_mut(&mut *self.store);
        if addr + 8 <= data.len() {
            data[addr..addr + 8].copy_from_slice(&v.to_le_bytes());
        }
    }

    fn read_i32(&mut self, addr: usize) -> i32 {
        let data = self.memory.data(&*self.store);
        if addr + 4 <= data.len() {
            return i32::from_le_bytes(data[addr..addr + 4].try_into().unwrap());
        }
        0
    }

    fn write_i32(&mut self, addr: usize, v: i32) {
        let data = self.memory.data_mut(&mut *self.store);
        if addr + 4 <= data.len() {
            data[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
        }
    }

    fn write_bytes(&mut self, addr: usize, bytes: &[u8]) -> bool {
        let data = self.memory.data_mut(&mut *self.store);
        if addr + bytes.len() <= data.len() {
            data[addr..addr + bytes.len()].copy_from_slice(bytes);
            return true;
        }
        false
    }
}

// ── Marshalling ─────────────────────────────────────────────────────────────

/// Traduce un tag de la tabla del RUNTIME (los records en memoria usan la tabla
/// interna: 0=int 1=string 2=float 3=bool 4=char 6=array 7=record) a la tabla
/// del BINDING (cls_kind_code: 0=int 1=float 2=bool 3=char 4=string 5=array
/// 6=record).
fn rt_tag_to_kind(tag: i64) -> i64 {
    match tag {
        0 => 0, // int
        1 => 4, // string
        2 => 1, // float
        3 => 2, // bool
        4 => 3, // char
        6 => 5, // array
        7 => 6, // record
        other => other,
    }
}

/// Traduce un kind del binding al tag del runtime (inverso).
fn kind_to_rt_tag(kind: i64) -> i64 {
    match kind {
        0 => 0, // int
        4 => 1, // string
        1 => 2, // float
        2 => 3, // bool
        3 => 4, // char
        5 => 6, // array
        6 => 7, // record
        other => other,
    }
}

/// Escribe un valor CLS en la memoria del módulo y devuelve sus bits i64
/// (int/float-bits/bool/char directos; string -> packed; array/record -> ptr).
/// `desc` = descriptor recursivo del tipo estático (arrays anidados necesitan
/// el tipo del elemento para el stride/layout); `None` = sin información
/// (los arrays se escriben con stride 8).
pub fn write_value(
    ctx: &mut StoreCtx,
    v: &ClsValue,
    desc: Option<&TypeDesc>,
) -> Result<i64, String> {
    match v {
        ClsValue::Null => Ok(0),
        ClsValue::Int(n) => Ok(*n),
        ClsValue::Float(f) => Ok(f.to_bits() as i64),
        ClsValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
        ClsValue::Char(c) => Ok(*c as i64),
        ClsValue::Str(s) => Ok(ctx.write_str(s)),
        ClsValue::Array(items) => write_array(ctx, items, desc.and_then(|d| d.elem.as_deref())),
        ClsValue::Record(entries) => write_record(ctx, entries, desc),
    }
}

fn elem_stride(elem_kind: i64) -> usize {
    // bool/char se empaquetan en i32 (stride 4); el resto en i64.
    if elem_kind == 2 || elem_kind == 3 {
        4
    } else {
        8
    }
}

/// Layout de array CLS: [cap:i64][len:i64][elems...].
fn write_array(ctx: &mut StoreCtx, items: &[ClsValue], elem: Option<&TypeDesc>) -> Result<i64, String> {
    let n = items.len() as i64;
    let elem_kind = elem.map(|d| d.kind).unwrap_or(-1);
    let stride = elem_stride(elem_kind);
    let ptr = ctx.alloc(n * stride as i64 + 16);
    if ptr == 0 {
        return Err("out of memory al alocar array".into());
    }
    ctx.write_i64(ptr as usize, n);
    ctx.write_i64(ptr as usize + 8, n);
    for (i, item) in items.iter().enumerate() {
        let bits = write_value(ctx, item, elem)?;
        let addr = ptr as usize + 16 + i * stride;
        if stride == 4 {
            ctx.write_i32(addr, bits as i32);
        } else {
            ctx.write_i64(addr, bits);
        }
    }
    Ok(ptr)
}

/// Layout de record CLS: [cap:i64][len:i64][(key, val, tag)*24].
fn write_record(ctx: &mut StoreCtx, entries: &[(String, ClsValue)], desc: Option<&TypeDesc>) -> Result<i64, String> {
    let n = entries.len() as i64;
    let ptr = ctx.alloc(n * 24 + 16);
    if ptr == 0 {
        return Err("out of memory al alocar record".into());
    }
    ctx.write_i64(ptr as usize, n);
    ctx.write_i64(ptr as usize + 8, n);
    for (i, (k, v)) in entries.iter().enumerate() {
        let key = ctx.write_str(k);
        let v_desc = desc.and_then(|d| {
            d.shape.iter().find(|(sk, _)| sk == k).map(|(_, d)| d)
                .or_else(|| d.value.as_deref())
        });
        let bits = write_value(ctx, v, v_desc)?;
        // Los records en memoria usan la tabla de tags del RUNTIME.
        let tag = kind_to_rt_tag(v.kind());
        let base = ptr as usize + 16 + i * 24;
        ctx.write_i64(base, key);
        ctx.write_i64(base + 8, bits);
        ctx.write_i64(base + 16, tag);
    }
    Ok(ptr)
}

/// Lee un valor CLS desde la memoria del módulo (bits + kind + desc).
pub fn read_value(ctx: &mut StoreCtx, bits: i64, kind: i64, desc: Option<&TypeDesc>) -> Result<ClsValue, String> {
    match kind {
        0 => Ok(ClsValue::Int(bits)),
        1 => Ok(ClsValue::Float(f64::from_bits(bits as u64))),
        2 => Ok(ClsValue::Bool(bits != 0)),
        3 => Ok(ClsValue::Char(char::from_u32(bits as u32).unwrap_or('?'))),
        4 => Ok(ClsValue::Str(ctx.read_str(bits))),
        5 => read_array(ctx, bits, desc.and_then(|d| d.elem.as_deref())),
        6 => read_record(ctx, bits, desc),
        9 | 12 => Ok(ClsValue::Null),
        other => Err(format!("tipo de retorno no soportado por el binding: kind {}", other)),
    }
}

fn read_array(ctx: &mut StoreCtx, ptr: i64, elem: Option<&TypeDesc>) -> Result<ClsValue, String> {
    let p = ptr as usize;
    let len = ctx.read_i64(p + 8);
    let elem_kind = elem.map(|d| d.kind).unwrap_or(-1);
    let stride = elem_stride(elem_kind);
    let mut items = Vec::with_capacity(len as usize);
    for i in 0..len {
        let addr = p + 16 + i as usize * stride;
        let bits = if stride == 4 {
            ctx.read_i32(addr) as i64
        } else {
            ctx.read_i64(addr)
        };
        let item = match elem {
            Some(d) if d.kind == 4 => ClsValue::Str(ctx.read_str(bits)),
            Some(d) => read_value(ctx, bits, d.kind, Some(d))?,
            None => {
                return Err(
                    "tipo de retorno no soportado por el binding: array sin tipo de elemento (falta el desc del export)".into(),
                )
            }
        };
        items.push(item);
    }
    Ok(ClsValue::Array(items))
}

fn read_record(ctx: &mut StoreCtx, ptr: i64, desc: Option<&TypeDesc>) -> Result<ClsValue, String> {
    let p = ptr as usize;
    let len = ctx.read_i64(p + 8);
    let mut entries = Vec::with_capacity(len as usize);
    for i in 0..len {
        let base = p + 16 + i as usize * 24;
        let kbits = ctx.read_i64(base);
        let key = ctx.read_str(kbits);
        let bits = ctx.read_i64(base + 8);
        let rt_tag = ctx.read_i64(base + 16);
        // Los records en memoria usan la tabla del runtime -> traducir.
        let kind = rt_tag_to_kind(rt_tag);
        let v_desc = desc.and_then(|d| {
            d.shape.iter().find(|(sk, _)| sk == &key).map(|(_, d)| d)
                .or_else(|| d.value.as_deref())
        });
        let val = read_value(ctx, bits, kind, v_desc)?;
        entries.push((key, val));
    }
    Ok(ClsValue::Record(entries))
}
