# Decisión 002 — Límite de aridad del FFI: 16 args con dispatch dinámico

**Estado**: vigente. **No negociable** para el dispatcher de extension
functions. El límite anterior (4 args) era un bug latente.

**Fecha**: dev-2 (post-eliminación del módulo `net` y la guía `when+extension`).

---

## Regla

Las funciones declaradas con `extension "lib" as C { function f(...) }`
aceptan **hasta 16 argumentos** sin restricción por parte del dispatcher
del runtime. El typeck y el emisor WASM no imponen este límite (siempre
aceptaron N args); el límite es puramente del backend nativo del nodo
`clx`, que tiene un dispatcher fijo para 0..16.

Si la función declarada tiene más de 16 args, el runtime emite un
**error claro** sugiriendo empaquetar en un struct:

```
La función nativa 'f' tiene 17 argumentos: el dispatcher de extension
soporta hasta 16. Para más args, empaquétalos en un struct y pasa
un puntero.
```

(No panickea con `unreachable!()` — ese era el bug del límite anterior.)

## Por qué 16

Las ABIs reales soportan como máximo 8-9 args en registros, el resto va
por stack:

- **x86_64 SysV** (Linux/macOS): 6 registros GP (`rdi`, `rsi`, `rdx`,
  `rcx`, `r8`, `r9`) + 8 registros XMM (`xmm0..xmm7`). Los f64 van en
  XMM, los i64 en GP; los registros no se pisan.
- **x86_64 Windows**: 4 registros GP (`rcx`, `rdx`, `r8`, `r9`) + 4
  registros XMM (`xmm0..xmm3`). Más restrictivo que SysV.
- **ARM64 (AArch64)**: 8 registros (`x0..x7`) que sirven para ambos tipos
  vía NEON/FP.

16 args es **más que cualquier ABI real soporta vía registros**: el
resto va por stack, y el layout del stack es uniforme (8 bytes por
slot). Es el límite que tiene sentido práctico: nadie declara APIs C
con 17+ args que merezcan un wrapper CLS.

Para más args (raro pero posible, p.ej. constructores de struct en
Win32 con muchos campos), la solución canónica es:

```clx
extension "user32.dll" as C {
    structure WndClassEx {
        cbSize: CInt,
        style: CInt,
        lpfnWndProc: CPtr,
        cbClsExtra: CInt,
        cbWndExtra: CInt,
        hInstance: CPtr,
        hIcon: CPtr,
        hCursor: CPtr,
        hbrBackground: CPtr,
        lpszMenuName: CString,
        lpszClassName: CString,
        hIconSm: CPtr,
    };
    function RegisterClassEx(wcx: CPtr) -> CInt;
};
```

## Por qué NO ilimitado

Hay 3 razones técnicas que descartan un dispatcher "completamente
genérico":

1. **El compilador no genera firmas variádicas C en Rust estable.** El
   truco `extern "C" fn(...)` con número variable de args no existe;
   hay que generar firmas concretas. El dispatcher elige entre N firmas
   pre-calculadas (16 lineales, no 2^N).
2. **Costo de un trampolín dinámico** (push a stack uno a uno):
   ~50-100ns extra por llamada. Para sockets/parsing de headers HTTP
   es prohibitivo.
3. **Compatibilidad con el linker dinámico**: el patrón dlsym requiere
   que el caller invoque con la firma exacta del símbolo. Cualquier
   discrepancia corrompe la stack o el retorno.

## Por qué NO matches estáticos por combinación de tipos (código viejo)

El código antes del fix usaba macros `arityN!` que generaban un `match`
sobre **todas las combinaciones de tipos** de los args. Para N=4 con
2 tipos posibles por arg, son 2^4 = 16 arms, cada una con su firma
C exacta. Para N=16 serían 2^16 = 65536 arms — completamente
intratable (y el código que se necesitaría en disco sería enorme).

El approach nuevo usa **una función por cantidad** (no por combinación).
Cada función castea el símbolo a `unsafe extern "C" fn(i64, i64, ..., i64)
-> R` donde los f64 se pasan como sus bits (mismo slot de 8 bytes). El
caller declara los tipos en el `extension`, así que el transmute es
seguro. Costo: 1 `match usize` en el call site, ~3ns. Mucho más rápido
que el match 2^N anterior.

## Limitaciones conocidas (no resueltas en este commit)

1. **El emisor no propaga target a imports de `extension` dentro de
   `when`**. Si declaras:
   ```clx
   when (os: windows) {
       extension "ws2_32.dll" as C { function socket(...) -> CInt; };
   }
   ```
   el emisor actual **no emite la rama que matchea el target** (bug
   pre-existente, no introducido por este commit). El typeck pasa,
   pero el WASM emitido no incluye el import, y la llamada falla con
   "símbolo no encontrado". Workaround temporal: declarar las
   extensiones **fuera** de `when` y aceptar que el linker carga la
   librería del host. Fix en commit separado (refactor del emisor para
   propagar target a los imports).

2. **`RetShape::I32` con 5+ args panickea** (no soportado). El C de 32
   bits (`int`) es raro en APIs modernas; si se necesita, el workaround
   es usar `CInt` (que se traduce a i32 en WASM pero i64 en el host)
   o reordenar para que la firma C lo exponga como i64.

3. **f32 (`CFloat`) no soportado** por el dispatcher. El typeck lo
   rechaza. Usar `CDouble`.

## Reversibilidad

Baja. Cambiar el límite requiere:
- Modificar `MAX_NATIVE_ARGS` en `nodos/clx/src/native.rs`.
- Agregar/eliminar brazos del `match args.len()` en `call_function`.
- Agregar/eliminar las funciones `call_typed_<N>` correspondientes.

Subir el límite (p.ej. a 32) es trivial; bajarlo requiere
rechazar también en typeck (hoy no se valida, solo runtime).

## Cambios derivados de esta decisión

- `nodos/clx/src/native.rs`: nuevo `enum RawArg`, funciones
  `call_typed_0..call_typed_16`, comentario del módulo actualizado.
- `examples/audit/test-features/tests/test-ffi-arity.clsx`: test de
  regresión. Documenta el caso end-to-end conocido (strlen, atoi).
- `docs/lenguaje/extension.md`: actualizar el límite documentado de 4 a 16.
