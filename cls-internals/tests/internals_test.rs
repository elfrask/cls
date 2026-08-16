//! Tests de paridad de `cls-internals`: instancia `INTERNALS_WASM` con wasmi y
//! verifica las funciones `__intr_*` contra los comportamientos documentados
//! (los mismos casos que `cls-jit/src/host.rs` implementa hoy).

use cls_internals::abi::INTERNALS_FUNCTIONS;
use wasmi::{Engine, Linker, Module, Store, TypedFunc};

/// Buffer de tests dentro de la memoria lineal (8MB; se crece la memoria antes).
const BUF: usize = 8 * 1024 * 1024;

struct Rt {
    store: Store<()>,
    instance: wasmi::Instance,
    mem: wasmi::Memory,
}

fn instantiate() -> Rt {
    let engine = Engine::default();
    let module = Module::new(&engine, cls_internals::INTERNALS_WASM).expect("módulo válido");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instanciar")
        .start(&mut store)
        .expect("start");
    let mem = instance
        .get_memory(&store, "memory")
        .expect("memoria exportada");
    // crecer para tener espacio de test
    mem.grow(&mut store, 200).expect("grow");
    Rt { store, instance, mem }
}

fn pack(ptr: usize, len: usize) -> i64 {
    ((ptr as i64) << 32) | (len as i64)
}

fn write_str(m: &mut [u8], addr: usize, s: &str) -> i64 {
    let b = s.as_bytes();
    m[addr..addr + b.len()].copy_from_slice(b);
    pack(addr, b.len())
}

fn read_str(m: &[u8], packed: i64) -> String {
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    String::from_utf8_lossy(&m[ptr..ptr + len]).into_owned()
}

fn rd_i64(m: &[u8], addr: usize) -> i64 {
    i64::from_le_bytes(m[addr..addr + 8].try_into().unwrap())
}

fn wr_i64(m: &mut [u8], addr: usize, v: i64) {
    m[addr..addr + 8].copy_from_slice(&v.to_le_bytes());
}

fn fn3<A: wasmi::WasmTyList, R: wasmi::WasmTyList>(
    rt: &mut Rt,
    name: &str,
) -> TypedFunc<A, R> {
    let f = rt
        .instance
        .get_export(&rt.store, name)
        .expect(name)
        .into_func()
        .expect("func");
    f.typed::<A, R>(&mut rt.store).expect("typed")
}

// ── arrays ────────────────────────────────────────────────────────────────

#[test]
fn arr_push_pop_shift_unshift_reverse() {
    let mut rt = instantiate();
    let push = fn3::<(i64, i64, i64), i64>(&mut rt, "__intr_arr_push");
    let pop = fn3::<(i64, i64), i64>(&mut rt, "__intr_arr_pop");
    let shift = fn3::<(i64, i64), i64>(&mut rt, "__intr_arr_shift");
    let unshift = fn3::<(i64, i64, i64), i64>(&mut rt, "__intr_arr_unshift");
    let reverse = fn3::<(i64, i64), i64>(&mut rt, "__intr_arr_reverse");

    let ptr = BUF as i64;
    {
        let m = rt.mem.data_mut(&mut rt.store);
        wr_i64(m, BUF, 4); // cap
        wr_i64(m, BUF + 8, 0); // len
    }
    // push 1..5 (cap 4 → realloc en el 5º)
    let mut p = ptr;
    for i in 1..=5i64 {
        p = push.call(&mut rt.store, (p, i * 10, 8)).unwrap();
    }
    {
        let m = rt.mem.data(&rt.store);
        assert_eq!(rd_i64(m, p as usize + 8), 5, "len");
        assert_eq!(rd_i64(m, p as usize + 16), 10, "elem0");
        assert_eq!(rd_i64(m, p as usize + 16 + 4 * 8), 50, "elem4");
    }
    // pop → len 4
    let p = pop.call(&mut rt.store, (p, 8)).unwrap();
    {
        let m = rt.mem.data(&rt.store);
        assert_eq!(rd_i64(m, p as usize + 8), 4);
    }
    // shift → [20,30,40,50]
    let p = shift.call(&mut rt.store, (p, 8)).unwrap();
    {
        let m = rt.mem.data(&rt.store);
        assert_eq!(rd_i64(m, p as usize + 8), 3);
        assert_eq!(rd_i64(m, p as usize + 16), 20);
    }
    // unshift 5 → [5,20,30,40]
    let p = unshift.call(&mut rt.store, (p, 5, 8)).unwrap();
    {
        let m = rt.mem.data(&rt.store);
        assert_eq!(rd_i64(m, p as usize + 8), 4);
        assert_eq!(rd_i64(m, p as usize + 16), 5);
    }
    // reverse → [40,30,20,5]
    let p = reverse.call(&mut rt.store, (p, 8)).unwrap();
    {
        let m = rt.mem.data(&rt.store);
        assert_eq!(rd_i64(m, p as usize + 16), 40);
        assert_eq!(rd_i64(m, p as usize + 16 + 3 * 8), 5);
    }
}

#[test]
fn arr_index_of_includes() {
    let mut rt = instantiate();
    let index_of = fn3::<(i64, i64, i64), i64>(&mut rt, "__intr_arr_index_of");
    let includes = fn3::<(i64, i64, i64), i32>(&mut rt, "__intr_arr_includes");
    let ptr = BUF as i64;
    {
        let m = rt.mem.data_mut(&mut rt.store);
        wr_i64(m, BUF, 8);
        wr_i64(m, BUF + 8, 3);
        wr_i64(m, BUF + 16, 7);
        wr_i64(m, BUF + 24, 8);
        wr_i64(m, BUF + 32, 9);
    }
    assert_eq!(index_of.call(&mut rt.store, (ptr, 8, 8)).unwrap(), 1);
    assert_eq!(index_of.call(&mut rt.store, (ptr, 99, 8)).unwrap(), -1);
    assert_eq!(includes.call(&mut rt.store, (ptr, 9, 8)).unwrap(), 1);
    assert_eq!(includes.call(&mut rt.store, (ptr, 42, 8)).unwrap(), 0);
}

#[test]
fn arr_join_strings() {
    let mut rt = instantiate();
    let join = fn3::<(i64, i64, i64, i64), i64>(&mut rt, "__intr_arr_join");
    let ptr = BUF as i64;
    let (s1, s2, s3, sep) = (BUF + 64, BUF + 128, BUF + 192, BUF + 256);
    let sp;
    {
        let m = rt.mem.data_mut(&mut rt.store);
        wr_i64(m, BUF, 8);
        wr_i64(m, BUF + 8, 3);
        let a = write_str(m, s1, "hola");
        let b = write_str(m, s2, "mundo");
        let c = write_str(m, s3, "cls");
        sp = write_str(m, sep, "-");
        wr_i64(m, BUF + 16, a);
        wr_i64(m, BUF + 24, b);
        wr_i64(m, BUF + 32, c);
        wr_i64(m, BUF + 40, sp);
    }
    let out = join
        .call(&mut rt.store, (ptr, sp, 8, 1))
        .unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(read_str(m, out), "hola-mundo-cls");
}

#[test]
fn arr_to_string() {
    let mut rt = instantiate();
    let to_string = fn3::<(i64, i64, i64), i64>(&mut rt, "__intr_arr_to_string");
    let ptr = BUF as i64;
    {
        let m = rt.mem.data_mut(&mut rt.store);
        wr_i64(m, BUF, 8);
        wr_i64(m, BUF + 8, 4);
        wr_i64(m, BUF + 16, 1);
        wr_i64(m, BUF + 24, 2);
        wr_i64(m, BUF + 32, 3);
        wr_i64(m, BUF + 40, 4);
    }
    let out = to_string.call(&mut rt.store, (ptr, 8, 0)).unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(read_str(m, out), "[1, 2, 3, 4]");
}

// ── strings ───────────────────────────────────────────────────────────────

#[test]
fn str_upper_lower_trim() {
    let mut rt = instantiate();
    let upper = fn3::<(i64,), i64>(&mut rt, "__intr_str_upper");
    let lower = fn3::<(i64,), i64>(&mut rt, "__intr_str_lower");
    let trim = fn3::<(i64,), i64>(&mut rt, "__intr_str_trim");
    let (a, b, c) = (BUF + 64, BUF + 128, BUF + 192);
    let (pa, pb, pc);
    {
        let m = rt.mem.data_mut(&mut rt.store);
        pa = write_str(m, a, "Hola MunDo");
        pb = write_str(m, b, "HOLA");
        pc = write_str(m, c, "  xyz  ");
    }
    let r_upper = upper.call(&mut rt.store, (pa,)).unwrap();
    let r_lower = lower.call(&mut rt.store, (pb,)).unwrap();
    let r_trim = trim.call(&mut rt.store, (pc,)).unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(read_str(m, r_upper), "HOLA MUNDO");
    assert_eq!(read_str(m, r_lower), "hola");
    assert_eq!(read_str(m, r_trim), "xyz");
}

#[test]
fn str_concat_repr_length() {
    let mut rt = instantiate();
    let concat = fn3::<(i64, i64), i64>(&mut rt, "__intr_str_concat");
    let repr = fn3::<(i64,), i64>(&mut rt, "__intr_str_repr");
    let length = fn3::<(i64,), i64>(&mut rt, "__intr_str_length");
    let (a, b) = (BUF + 64, BUF + 128);
    let (pa, pb);
    {
        let m = rt.mem.data_mut(&mut rt.store);
        pa = write_str(m, a, "ab");
        pb = write_str(m, b, "cd");
    }
    let r_concat = concat.call(&mut rt.store, (pa, pb)).unwrap();
    let r_repr = repr.call(&mut rt.store, (pa,)).unwrap();
    let r_len = length.call(&mut rt.store, (pa,)).unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(read_str(m, r_concat), "abcd");
    assert_eq!(read_str(m, r_repr), "\"ab\"");
    assert_eq!(r_len, 2);
}

#[test]
fn str_contains_starts_ends_empty() {
    let mut rt = instantiate();
    let contains = fn3::<(i64, i64), i32>(&mut rt, "__intr_str_contains");
    let starts = fn3::<(i64, i64), i32>(&mut rt, "__intr_str_starts_with");
    let ends = fn3::<(i64, i64), i32>(&mut rt, "__intr_str_ends_with");
    let is_empty = fn3::<(i64,), i32>(&mut rt, "__intr_str_is_empty");
    let (a, b, c, d, e) = (BUF + 64, BUF + 128, BUF + 192, BUF + 256, BUF + 320);
    let (pa, pb, pc, pd, pe);
    {
        let m = rt.mem.data_mut(&mut rt.store);
        pa = write_str(m, a, "hola mundo");
        pb = write_str(m, b, "mun");
        pc = write_str(m, c, "hola");
        pd = write_str(m, d, "");
        pe = write_str(m, e, "x");
    }
    let r_contains = contains.call(&mut rt.store, (pa, pb)).unwrap();
    let r_starts = starts.call(&mut rt.store, (pa, pc)).unwrap();
    let r_ends1 = ends.call(&mut rt.store, (pa, pb)).unwrap();
    let r_ends2 = ends.call(&mut rt.store, (pa, pe)).unwrap();
    let r_empty1 = is_empty.call(&mut rt.store, (pd,)).unwrap();
    let r_empty2 = is_empty.call(&mut rt.store, (pa,)).unwrap();
    assert_eq!(r_contains, 1);
    assert_eq!(r_starts, 1);
    assert_eq!(r_ends1, 0);
    assert_eq!(r_ends2, 0);
    assert_eq!(r_empty1, 1);
    assert_eq!(r_empty2, 0);
}

#[test]
fn str_int_float_bool_char() {
    let mut rt = instantiate();
    let str_int = fn3::<(i64,), i64>(&mut rt, "__intr_str_int");
    let str_float = fn3::<(f64,), i64>(&mut rt, "__intr_str_float");
    let str_bool = fn3::<(i32,), i64>(&mut rt, "__intr_str_bool");
    let str_char = fn3::<(i32,), i64>(&mut rt, "__intr_str_char");
    let r_int = str_int.call(&mut rt.store, (-42,)).unwrap();
    let r_float = str_float.call(&mut rt.store, (2.5,)).unwrap();
    let r_bool = str_bool.call(&mut rt.store, (1,)).unwrap();
    let r_char = str_char.call(&mut rt.store, (65,)).unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(read_str(m, r_int), "-42");
    assert_eq!(read_str(m, r_float), "2.5");
    assert_eq!(read_str(m, r_bool), "true");
    assert_eq!(read_str(m, r_char), "A");
}

// ── records ───────────────────────────────────────────────────────────────

#[test]
fn record_set_get_has_tag_len_keys_values() {
    let mut rt = instantiate();
    let rec_new = fn3::<(i64,), i64>(&mut rt, "__intr_record_new");
    let rec_set = fn3::<(i64, i64, i64, i64), i64>(&mut rt, "__intr_record_set");
    let rec_get = fn3::<(i64, i64), i64>(&mut rt, "__intr_record_get");
    let rec_has = fn3::<(i64, i64), i32>(&mut rt, "__intr_record_has");
    let rec_tag = fn3::<(i64, i64), i64>(&mut rt, "__intr_record_tag");
    let rec_len = fn3::<(i64,), i64>(&mut rt, "__intr_record_len");
    let rec_keys = fn3::<(i64,), i64>(&mut rt, "__intr_record_keys");
    let rec_values = fn3::<(i64,), i64>(&mut rt, "__intr_record_values");
    let (k1, k2, k3) = (BUF + 64, BUF + 128, BUF + 192);
    let (pk1, pk2, pk3);
    {
        let m = rt.mem.data_mut(&mut rt.store);
        pk1 = write_str(m, k1, "a");
        pk2 = write_str(m, k2, "b");
        pk3 = write_str(m, k3, "z");
    }
    let r = rec_new.call(&mut rt.store, (4,)).unwrap();
    let r = rec_set.call(&mut rt.store, (r, pk1, 1, 0)).unwrap();
    let r = rec_set.call(&mut rt.store, (r, pk2, 2, 0)).unwrap();
    let r = rec_set.call(&mut rt.store, (r, pk3, 3, 1)).unwrap();
    // actualizar "a"
    let r = rec_set.call(&mut rt.store, (r, pk1, 99, 0)).unwrap();
    let r_len_v = rec_len.call(&mut rt.store, (r,)).unwrap();
    let r_get = rec_get.call(&mut rt.store, (r, pk1)).unwrap();
    let r_has1 = rec_has.call(&mut rt.store, (r, pk2)).unwrap();
    let r_has2 = rec_has.call(&mut rt.store, (r, pack(0, 0))).unwrap();
    let r_tag = rec_tag.call(&mut rt.store, (r, pk3)).unwrap();
    let keys = rec_keys.call(&mut rt.store, (r,)).unwrap();
    let values = rec_values.call(&mut rt.store, (r,)).unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(r_len_v, 3);
    assert_eq!(r_get, 99);
    assert_eq!(r_has1, 1);
    assert_eq!(r_has2, 0);
    assert_eq!(r_tag, 1);
    assert_eq!(rd_i64(m, keys as usize + 8), 3);
    assert_eq!(rd_i64(m, values as usize + 16), 99);
}

#[test]
fn record_to_string_sorted() {
    let mut rt = instantiate();
    let rec_new = fn3::<(i64,), i64>(&mut rt, "__intr_record_new");
    let rec_set = fn3::<(i64, i64, i64, i64), i64>(&mut rt, "__intr_record_set");
    let rec_str = fn3::<(i64,), i64>(&mut rt, "__intr_record_to_string");
    let (k1, k2) = (BUF + 64, BUF + 128);
    let (pk1, pk2);
    {
        let m = rt.mem.data_mut(&mut rt.store);
        pk1 = write_str(m, k1, "b");
        pk2 = write_str(m, k2, "a");
    }
    let r = rec_new.call(&mut rt.store, (4,)).unwrap();
    let r = rec_set.call(&mut rt.store, (r, pk1, 2, 0)).unwrap();
    let r = rec_set.call(&mut rt.store, (r, pk2, 1, 0)).unwrap();
    let out = rec_str.call(&mut rt.store, (r,)).unwrap();
    let m = rt.mem.data(&rt.store);
    assert_eq!(read_str(m, out), "{a: 1, b: 2}");
}

// ── math ──────────────────────────────────────────────────────────────────

#[test]
fn math_basics() {
    let mut rt = instantiate();
    let sqrt = fn3::<(f64,), f64>(&mut rt, "__intr_math_sqrt");
    let min = fn3::<(f64, f64), f64>(&mut rt, "__intr_math_min");
    let max = fn3::<(f64, f64), f64>(&mut rt, "__intr_math_max");
    let floor = fn3::<(f64,), f64>(&mut rt, "__intr_math_floor");
    let ceil = fn3::<(f64,), f64>(&mut rt, "__intr_math_ceil");
    let round = fn3::<(f64,), f64>(&mut rt, "__intr_math_round");
    let fmod = fn3::<(f64, f64), f64>(&mut rt, "__intr_math_fmod");
    assert_eq!(sqrt.call(&mut rt.store, (9.0,)).unwrap(), 3.0);
    assert_eq!(min.call(&mut rt.store, (1.5, 2.5)).unwrap(), 1.5);
    assert_eq!(max.call(&mut rt.store, (1.5, 2.5)).unwrap(), 2.5);
    assert_eq!(floor.call(&mut rt.store, (2.7,)).unwrap(), 2.0);
    assert_eq!(ceil.call(&mut rt.store, (2.1,)).unwrap(), 3.0);
    assert_eq!(round.call(&mut rt.store, (2.5,)).unwrap(), 3.0);
    assert_eq!(fmod.call(&mut rt.store, (7.0, 3.0)).unwrap(), 1.0);
}

#[test]
fn math_trig_log_pow() {
    let mut rt = instantiate();
    let sin = fn3::<(f64,), f64>(&mut rt, "__intr_math_sin");
    let cos = fn3::<(f64,), f64>(&mut rt, "__intr_math_cos");
    let tan = fn3::<(f64,), f64>(&mut rt, "__intr_math_tan");
    let log = fn3::<(f64,), f64>(&mut rt, "__intr_math_log");
    let pow = fn3::<(f64, f64), f64>(&mut rt, "__intr_math_pow");
    let eps = 1e-12;
    assert!((sin.call(&mut rt.store, (1.0,)).unwrap() - 1.0f64.sin()).abs() < eps);
    assert!((cos.call(&mut rt.store, (1.0,)).unwrap() - 1.0f64.cos()).abs() < eps);
    assert!((tan.call(&mut rt.store, (0.5,)).unwrap() - 0.5f64.tan()).abs() < eps);
    assert!((log.call(&mut rt.store, (2.0,)).unwrap() - 2.0f64.ln()).abs() < eps);
    assert!((pow.call(&mut rt.store, (2.0, 10.0)).unwrap() - 1024.0).abs() < eps);
}

#[test]
fn math_range_pow_num() {
    let mut rt = instantiate();
    let range = fn3::<(i64, i64), i64>(&mut rt, "__intr_math_range");
    let pow_num = fn3::<(i64, i64), i64>(&mut rt, "__intr_pow_num");
    let ptr = range.call(&mut rt.store, (2, 5)).unwrap();
    {
        let m = rt.mem.data(&rt.store);
        assert_eq!(rd_i64(m, ptr as usize + 8), 3);
        assert_eq!(rd_i64(m, ptr as usize + 16), 2);
        assert_eq!(rd_i64(m, ptr as usize + 24), 3);
        assert_eq!(rd_i64(m, ptr as usize + 32), 4);
    }
    assert_eq!(pow_num.call(&mut rt.store, (2, 10)).unwrap(), 1024);
    assert_eq!(pow_num.call(&mut rt.store, (5, 0)).unwrap(), 1);
}

// ── conversiones ──────────────────────────────────────────────────────────

#[test]
fn parse_int_float_bool() {
    let mut rt = instantiate();
    let parse_int = fn3::<(i64,), i64>(&mut rt, "__intr_parse_int");
    let parse_float = fn3::<(i64,), f64>(&mut rt, "__intr_parse_float");
    let parse_bool = fn3::<(i64,), i32>(&mut rt, "__intr_parse_bool");
    let (a, b, c) = (BUF + 64, BUF + 128, BUF + 192);
    let (pa, pb, pc);
    {
        let m = rt.mem.data_mut(&mut rt.store);
        pa = write_str(m, a, " 123 ");
        pb = write_str(m, b, "3.14");
        pc = write_str(m, c, "x");
    }
    assert_eq!(parse_int.call(&mut rt.store, (pa,)).unwrap(), 123);
    assert_eq!(parse_float.call(&mut rt.store, (pb,)).unwrap(), 3.14);
    assert_eq!(parse_bool.call(&mut rt.store, (pc,)).unwrap(), 1);
    // error → 0 + flag vía getter
    let parse_err = fn3::<(), i32>(&mut rt, "__intr_parse_error_get");
    let bad = write_str(&mut rt.mem.data_mut(&mut rt.store), BUF + 256, "abc");
    assert_eq!(parse_int.call(&mut rt.store, (bad,)).unwrap(), 0);
    assert_eq!(parse_err.call(&mut rt.store, ()).unwrap(), 1);
}

// ── manifiesto ────────────────────────────────────────────────────────────

#[test]
fn manifest_matcher_exported_functions() {
    // Cada firma declarada en abi::INTERNALS_FUNCTIONS debe existir en el wasm.
    let engine = Engine::default();
    let module = Module::new(&engine, cls_internals::INTERNALS_WASM).expect("módulo");
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instanciar")
        .start(&mut store)
        .expect("start");
    for f in INTERNALS_FUNCTIONS {
        assert!(
            instance.get_export(&store, f.name).is_some(),
            "export faltante: {}",
            f.name
        );
    }
    assert!(!INTERNALS_FUNCTIONS.is_empty());
}
