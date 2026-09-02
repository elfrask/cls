//! Formateo de valores por tag (paridad con `cls-jit/src/host.rs`:
//! `format_float`/`json_escape`/`fmt_val_to_string`/`arr_to_string`/
//! `record_to_string`/`cmx_format`).

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::arrays::{arr_elem, arr_len};
use crate::mem;

/// Formatea un f64 como el host (`format!("{}", v)`).
pub fn format_float(v: f64) -> String {
    format!("{}", v)
}

pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
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
pub fn fmt_val_to_string(val: i64, tag: i64) -> String {
    unsafe {
        let t = tag_type(tag);
        let kind = (tag & 0xff) as i32;
        match t {
            1 => mem::read_str(val),
            2 => format_float(f64::from_bits(val as u64)),
            3 => {
                if val != 0 {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            4 => char::from_u32(val as u32).unwrap_or('?').to_string(),
            5 => cmx_format(val as usize),
            6 => {
                // kind 5 = array de Cmx (entradas `[val, tag]` stride 16);
                // cualquier otro kind = array/tupla contigua con es=8 (ints/ptr).
                let es = if kind == 5 { 16 } else { 8 };
                let arr_kind = if kind == 5 { 5 } else { kind };
                arr_to_string(val, es, arr_kind as i64)
            }
            7 => record_to_string(val),
            _ => val.to_string(),
        }
    }
}

/// `[e1, e2, ...]` - kind 5 = array de Cmx (entradas `[val, tag]` stride 16).
pub fn arr_to_string(ptr: i64, es: i64, kind: i64) -> String {
    if ptr == 0 {
        return String::from("[]");
    }
    unsafe {
        let p = ptr as usize;
        let es = es as usize;
        let len = arr_len(p);
        let mut out = String::from("[");
        for i in 0..len as usize {
            if i > 0 {
                out.push_str(", ");
            }
            let e = arr_elem(p, i, es);
            match kind {
                1 => {
                    out.push('"');
                    out.push_str(&json_escape(&mem::read_str(e)));
                    out.push('"');
                }
                2 => out.push_str(&format_float(f64::from_bits(e as u64))),
                3 => out.push_str(if e != 0 { "true" } else { "false" }),
                4 => out.push(char::from_u32(e as u32).unwrap_or('?')),
                5 => {
                    let tg = mem::read_i64(p + 16 + i * 16 + 8);
                    let tv = tag_type(tg);
                    if tv == 1 {
                        out.push('"');
                        out.push_str(&json_escape(&mem::read_str(e)));
                        out.push('"');
                    } else if tv == 5 {
                        let ck = mem::read_i64((e as usize) + 24);
                        if ck == 1 {
                            let ctag = mem::read_i64(e as usize);
                            out.push('"');
                            out.push_str(&json_escape(&mem::read_str(ctag)));
                            out.push('"');
                        } else {
                            out.push_str(&cmx_format(e as usize));
                        }
                    } else {
                        out.push_str(&fmt_val_to_string(e, tg));
                    }
                }
                // Contenedores (6=array/tupla, 7=record): cada elemento es un
                // ptr a un contenedor anidado; despachar por su tag compuesto.
                // Bug dev-2: `enums: [{...}]` imprimía punteros crudos porque
                // el kind 0 (default) los trataba como ints.
                6 | 7 => out.push_str(&fmt_val_to_string(e, kind << 8)),
                _ => out.push_str(&e.to_string()),
            }
        }
        out.push(']');
        out
    }
}

/// `{k: v, ...}` - claves ordenadas (como el walker).
pub fn record_to_string(ptr: i64) -> String {
    if ptr == 0 {
        return String::from("{}");
    }
    unsafe {
        let p = ptr as usize;
        let len = arr_len(p);
        let mut entries: Vec<(String, i64, i64)> = Vec::with_capacity(len as usize);
        for i in 0..len as usize {
            let key = mem::read_i64(p + 16 + i * 24);
            let val = mem::read_i64(p + 16 + i * 24 + 8);
            let tag = mem::read_i64(p + 16 + i * 24 + 16);
            entries.push((mem::read_str(key), val, tag));
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
                out.push_str(&json_escape(&mem::read_str(*val)));
                out.push('"');
            } else {
                out.push_str(&fmt_val_to_string(*val, *tag));
            }
        }
        out.push('}');
        out
    }
}

/// Formatea un CmxValue. Un CmxValue de "texto" (kind=1) se muestra plano.
/// NOTA: los fn handles (`tag>>32 == 0 && tag != 0`) se muestran como
/// `<function N>` — el host usa el nombre real vía estado (Fase 3).
pub fn cmx_format(p: usize) -> String {
    unsafe {
        let tag = mem::read_i64(p);
        let props = mem::read_i64(p + 8) as usize;
        let children = mem::read_i64(p + 16) as usize;
        let kind = mem::read_i64(p + 24);
        let nprops = if props != 0 { arr_len(props) as usize } else { 0 };
        let nchild = if children != 0 { arr_len(children) as usize } else { 0 };
        if kind == 1 {
            return mem::read_str(tag);
        }
        let mut out = String::from("<");
        if tag >> 32 == 0 && tag != 0 {
            out.push_str(&format!("<function {}>", tag));
        } else {
            out.push_str(&mem::read_str(tag));
        }
        let mut prop_entries: Vec<(String, i64, i64)> = Vec::with_capacity(nprops);
        for i in 0..nprops {
            let key = mem::read_i64(props + 16 + i * 24);
            let val = mem::read_i64(props + 16 + i * 24 + 8);
            let t = mem::read_i64(props + 16 + i * 24 + 16);
            prop_entries.push((mem::read_str(key), val, t));
        }
        prop_entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (key, val, t) in prop_entries {
            out.push(' ');
            out.push_str(&key);
            out.push_str("=\"");
            let tv = tag_type(t);
            if tv == 1 {
                out.push_str(&mem::read_str(val));
            } else {
                out.push_str(&fmt_val_to_string(val, t));
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
                out.push_str(&format!("<function {}>", tag));
            } else {
                out.push_str(&mem::read_str(tag));
            }
            out.push('>');
        }
        out
    }
}
