//! Smoke test del runtime wasmi (feature `wasmi-runtime`): valida que el
//! intérprete ejecute un módulo trivial con host function, memory, alloc y main.

#[cfg(feature = "wasmi-runtime")]
#[test]
fn wasmi_smoke() {
    let wat = r#"
        (module
          (import "env" "f" (func $f (param i64) (result i64)))
          (func (export "alloc") (param i64) (result i64)
            local.get 0)
          (func (export "main") (param i64) (result i64)
            local.get 0
            call $f)
          (memory (export "memory") 1))
    "#;
    let bytes = wat::parse_str(wat).expect("wat");
    let engine = wasmi::Engine::default();
    let module = wasmi::Module::new(&engine, &bytes).expect("module");
    let mut store = wasmi::Store::new(&engine, ());
    let mut linker = wasmi::Linker::new(&engine);
    linker
        .func_wrap("env", "f", |_: wasmi::Caller<'_, ()>, x: i64| -> i64 { x + 1 })
        .expect("host");
    let instance = linker
        .instantiate(&mut store, &module).expect("instantiate").start(&mut store).expect("start");
            let alloc = instance
        .get_typed_func::<i64, i64>(&store, "alloc")
        .expect("alloc");
    let p = alloc.call(&mut store, 16).expect("alloc call");
    assert_eq!(p, 16);
    let main = instance
        .get_typed_func::<i64, i64>(&store, "main")
        .expect("main");
    let r = main.call(&mut store, p).expect("main call");
    assert_eq!(r, 17);
}

/// Smoke con `Memory::read`/`Memory::write` DESDE UN HOST (el patrón que usan
/// los hosts de cls-jit). `Memory::data/data_mut` desde un caller crashea en
/// wasmi 2.0-beta -> solo se usan las APIs seguras.
#[cfg(feature = "wasmi-runtime")]
#[test]
fn wasmi_host_read_write() {
    let wat = r#"
        (module
          (import "env" "poke" (func $poke (param i64 i64)))
          (func (export "alloc") (param i64) (result i64)
            local.get 0)
          (func (export "main") (param i64) (result i64)
            local.get 0
            i64.const 7
            call $poke
            local.get 0
            i32.wrap_i64
            i64.load)
          (memory (export "memory") 1))
    "#;
    let bytes = wat::parse_str(wat).expect("wat");
    let engine = wasmi::Engine::default();
    let module = wasmi::Module::new(&engine, &bytes).expect("module");
    let mut store = wasmi::Store::new(&engine, ());
    let mut linker = wasmi::Linker::new(&engine);
    linker
        .func_wrap("env", "poke", |mut c: wasmi::Caller<'_, ()>, addr: i64, v: i64| {
            let mem = c
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("mem");
            mem.write(&mut c, addr as usize, &v.to_le_bytes())
                .expect("write");
        })
        .expect("host");
    let instance = linker
        .instantiate(&mut store, &module).expect("instantiate").start(&mut store).expect("start");
            let alloc = instance
        .get_typed_func::<i64, i64>(&store, "alloc")
        .expect("alloc");
    let p = alloc.call(&mut store, 64).expect("alloc call");
    let main = instance
        .get_typed_func::<i64, i64>(&store, "main")
        .expect("main");
    let r = main.call(&mut store, p).expect("main call");
    assert_eq!(r, 7);
}

