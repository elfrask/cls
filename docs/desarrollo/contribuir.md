# Contribuir

## Repositorio y ramas

- El remoto `origin` apunta a `https://github.com/elfrask/cls.git`
  (el metadata del workspace declara `https://github.com/frask/cls` como
  homepage/repository).
- Ramas existentes: `master`, `2.0`, `2.X-jit`, `1.2`, `1.0-1.1`,
  `respaldo-2.0`, `bindings` (rama de trabajo actual).

## Estilo de commits

Los commits se escriben en **español**, con prefijo de tipo y, cuando
aplica, scope. Ejemplos reales (`git log --oneline -15`):

```
fd3924a fix(modules): M1 aridad valida sin panics, M2 time.sleep como arg imprime void
379e14a feat(modules): modulos internos del nodo desktop - os, path, process, time, random
a569e3b feat(bindings): F2b - ABI C clsb_v1_* + header + harness C (contrato validado)
f2c92ea feat(bindings): F2a - nodo clxb, motor de embedding (compile, call, run_main, eval, SDK de nodo)
8d8bd85 feat(jit): bindings F0-b + F1 - runtime wasmi, exports tipados y canal host_call
5378876 audit(migracion-jit): se ha hecho migraciones del motor jit y se han auditado.
041fac2 feat(cli): deprecacion del ast-walker - clx run usa el JIT por defecto
d295614 audit(release2.0-dev1): se han realizado las ultimas auditorias del core...
49be202 fix(jit): cierre de validacion final - X1 math.pow, X2 compound index float, X3 colision de nombres de modulo
e978d11 fix(dev2): cierre de deuda tecnica de auditoria + 2 feature-gaps
871ba91 fix(jit): fase 1-r2 residuos R1-R10 + fase 2-r2 R5/R9 ...
7844988 audit(fase-2): auditoria de la fase 2
9b7aa64 fix(jit): cierra Fase 2 (residuos R1-R6 de la auditoria) ...
d02b643 fix(jit): cierra Fase 1 (residuos R1-R11 de la auditoria) ...
b18ece4 fix(jit): fase 2 altos A-1..A-7 ...
```

Convenciones observadas:

- Prefijos: `feat:`, `fix:`, `wip:`, `audit:`, `docs:`.
- Con scope cuando afecta a una parte: `fix(jit):`, `feat(bindings):`,
  `feat(cli):`, `fix(modules):`, `audit(fase-2):`.
- Los fixes describen el problema y la causa raíz del fix.

## Reglas de trabajo

1. **El JIT es el intérprete objetivo.** `clx run` compila CLS → WASM →
   wasmtime. El tree-walker (`--ast-walker`) está **DEPRECADO** y se elimina
   tras CLS 2.0-dev1: es solo referencia sintáctica.
2. **No invertir tiempo en paridad con el walker** para features nuevas. El
   walker puede quedarse sin soportar algo; lo que importa es que el **JIT**
   lo soporte.
3. **Tests de features** en `examples/audit/test-features/jit-test/`
   comparan JIT vs walker (`run-availible.ps1`, `run-tests.ps1`), pero el
   objetivo real es el JIT. Si un test solo aplica al JIT, va en
   `examples/jit-examples/` o `examples/audit/test-features/jit-test/`
   (ver `desarrollo/testing.md`).
4. **Rendimiento**: cualquier decisión de diseño prioriza el costo en
   runtime del JIT/WASM — nada de boxing, dispatch dinámico ni alocaciones
   innecesarias.
5. **El typeck** (`clx check`) es la fuente de tipos para ambos; el JIT lo
   requiere para emitir (`types_by_span`).
6. **Errores**: el runtime/compilador muestra **siempre** el trace completo
   (import_trace + call stack + caret); el typecheck, un solo nivel. Usa las
   fábricas centralizadas (`ClsError::syntax_at`/`compile_at`) y spans
   estructurados.

## Documentación

- `docs/` documenta **solo lo implementado y accesible hoy**. Si un
  documento menciona algo que no existe en el código, es un error de
  documentación.
- Si una feature o un comportamiento cambia, actualiza `docs/` en el mismo
  cambio. El índice de la documentación vive en `docs/README.md`; el contexto
  del proyecto en `AGENTS.md` (y los planes de features en `agent-context/`).

## Flujo sugerido

1. Verifica el estado del JIT antes de tocar el walker
   (`docs/runtime/jit.md` + `runtime/walker.md`).
2. Implementa por capas (ver `desarrollo/agregar-feature.md`).
3. Agrega tests en cada capa (`cargo test`) y QA en
   `examples/audit/features/` o `jit-test/`.
4. `clx check --strict` + ejecución con `clx run` antes de commitear.
5. Commit en español con prefijo (ej. `feat(jit): ...`).