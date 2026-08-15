# Fase 1 — Modularización y refactorización atómica del código

> **Estado**: PLAN aprobado. El backend está en 9 706 líneas en un solo archivo
> (`cls-core/src/backend/wasm.rs`) "a punto de estallar". Este documento define
> la separación en **archivos atómicos** por área, sin cambios de comportamiento.

## Objetivo

Dividir los archivos gigantes en módulos de **200–900 líneas** localizables por
área, manteniendo la API pública intacta y las suites verdes en cada paso. El
refactor es **prerrequisito** de las fases 2 y 3 (crate de internos + adaptación
del emisor): cada optimización futura caerá en un archivo pequeño.

## Reglas de trabajo (obligatorias)

1. **Refactor puro**: NO se cambia comportamiento, mensajes de error, ni salida.
   Cualquier "mejora de paso" va en commit separado marcado como tal.
2. **Un bloque por commit**: cada commit mueve UNA pieza y el workspace compila
   (`cargo check -p cls-core -p cls-runtime -p cls-jit -p clx -p clxb`).
3. **Red de seguridad por paso**: `cargo test` + `run-availible.ps1` (25) +
   `run-tests.ps1` (20) deben quedar verdes tras cada commit.
4. Los doc-comments y la convención en español se preservan tal cual (sin
   reescribir documentación durante el refactor).
5. La API pública (`WasmBackend`, `WasmBackendOptions`, `HostFn`,
   `frontend::ast::*`, `frontend::token::*`, `cls_runtime::*`) queda estable:
   los consumidores (`cls-jit`, `clxb`, REPL, bindings) no cambian ni una línea.

---

## 1.1 `cls-core/src/backend/` → módulo `backend/wasm/` (~20 archivos)

```
cls-core/src/backend/
├── wasm/
│   ├── mod.rs            # re-exports públicos: WasmBackend, WasmBackendOptions, HostFn
│   ├── host_fn.rs        # enum HostFn + impl (import_name/signature)        [wasm.rs:55–478]
│   ├── types.rs          # WasTy, was_type, BuiltinTypeName, ty_code,         [479–650, 7945–7972]
│   │                     #   code_to_was, was_to_val, annotation_to_type
│   ├── layout.rs         # consts STRING_TABLE_BASE/NULL_ITER_SENTINEL,        [30–52, 560–597, 7503–7636]
│   │                     #   cls_kind_code, runtime_tag_code, elem_size_bytes,
│   │                     #   apply_compound_ty, arr_kind_code, cmx_tag_for_type
│   ├── emitter/
│   │   ├── mod.rs        # struct FuncEmitter + constructor + helpers base      [668–1055]
│   │   │                 #   (fresh_local, local_for, declare_var_ty, value_type,
│   │   │                 #    emit_drop, emit_i64_store, intern_string, emit_load_str)
│   │   ├── statements.rs # emit_statement, foreach(+magic+array+next),          [1055–2016]
│   │   │                 #   switch, with, try, if/while/loop/for
│   │   ├── expressions.rs# emit_expression, emit_literal, unary, incdec,        [2016–2156, 2893–3005]
│   │   │                 #   conditional, interpolation
│   │   ├── binary.rs     # emit_binary, push_eq/cmp, f64_promote,               [2294–2893]
│   │   │                 #   coerce_to_bool, div_zero_trap, emit_throw
│   │   ├── assignment.rs # emit_assignment                                   [3005–3560]
│   │   ├── calls.rs      # emit_call (dividido por sub-dispatch) +              [4020–4985, 2189–2274]
│   │   │                 #   emit_host_call + emit_fn_enter/exit/call_site
│   │   ├── modules.rs    # emit_math_call, fs, http, os, path, process, time,   [3560–4020]
│   │   │                 #   random, tuple_join, module_call_ret, call_arg
│   │   ├── classes.rs    # emit_class_method*, class_magic_method,              [4985–5136, 6409–6510]
│   │   │                 #   magic_ret_*, try_binary_magic, check_field/method_access
│   │   ├── strings.rs    # emit_print_arg, emit_to_string family,               [5136–6345]
│   │   │                 #   struct/shape to_string, to_int/float/bool
│   │   ├── containers.rs # array, record, shape_record, shape_layout, tuple,    [6345–6794, 6821–7489]
│   │   │                 #   cmx, index_get/set, bounds_check, writeback_array
│   │   └── member.rs     # emit_member_access, emit_any_chain                 [6526–6794]
│   ├── engine/
│   │   ├── mod.rs        # struct Engine + new + emit + build_module            [7836–8090, 8175–9004]
│   │   ├── functions.rs  # collect_functions, compile_function,                 [8090–8175, 9004–9265]
│   │   │                 #   declare_class_function, resolve_method_index
│   │   ├── globals.rs    # build_global_init, build_allocator, build_load_str,  [9159–9414]
│   │   │                 #   build_string_data
│   │   └── metadata.rs   # ClassInfo, StructInfo, FieldVis, shape_layout,       [7904–7982]
│   │                     #   collect de clases/structs/enums
│   ├── arrows.rs         # collect_arrows_*, collect_free_vars_*               [9414–9706]
│   └── helpers.rs        # expr_display, statement_display, type_name_str,      [7503–7712, 7644–7666]
│                         #   is_compound, is_math_range_call, cmx_literal_type,
│                         #   union_base, noop_main_decl
└── mod.rs                # pub mod wasm; (json.rs y visitor.rs quedan igual)
```

**Viabilidad Rust**: los `impl FuncEmitter { ... }` y `impl Engine { ... }`
pueden vivir en archivos distintos del struct mientras estén en el mismo módulo
padre (`emitter/`/`engine/`); los campos se marcan `pub(super)` donde haga falta.
No se toca lógica — solo se mueven bloques completos y se ajustan `use`s.

**Pasos de migración** (cada uno = 1 commit + suites verdes):

| Paso | Bloque | Riesgo |
|---|---|---|
| 1 | Crear `wasm/mod.rs` que re-exporte el archivo actual (sin mover nada) | nulo |
| 2 | Mover helpers sueltos (`helpers.rs`, `layout.rs`, `types.rs`, `host_fn.rs`) | bajo (funciones libres) |
| 3 | Mover `Engine` completo → `engine/mod.rs` | bajo (1 struct) |
| 4 | Partir `Engine` en functions/globals/metadata | medio |
| 5 | Mover `FuncEmitter` → `emitter/mod.rs` (struct + helpers base) | medio |
| 6 | Partir `FuncEmitter` por área (statements → expressions → binary → …) | medio-alto |
| 7 | `arrows.rs` | bajo |
| 8 | Eliminar `backend/wasm.rs` original (queda solo el dir) | nulo |

---

## 1.2 `cls-core/src/frontend/ast.rs` → módulo `frontend/ast/` (archivos atómicos)

`ast.rs` (990 líneas) concentra 60+ tipos: declaraciones, expresiones,
anotaciones y CMX en un solo archivo.

```
cls-core/src/frontend/ast/
├── mod.rs            # re-exports de todo (API estable: pub use ast::*)
├── module.rs         # Module, Statement                          [ast.rs:7–74]
├── declarations.rs   # VarDecl, FunctionDecl, ExtensionDecl,       [75–332]
│                     #   Target, TargetCond, WhenBlock/Branch, Parameter,
│                     #   FunctionModifier, NativeDecl, ExtensionKind
├── control.rs        # If/Elif/While/For/ForEach/Switch/Case/      [312–397]
│                     #   Try/Catch/With
├── classes.rs        # ClassDecl, ClassMember, StructureDecl,      [399–540]
│                     #   FieldDecl, InterfaceDecl/Field, TypeAliasDecl,
│                     #   EnumDecl, SignatureDecl, ModuleDecl,
│                     #   NamespaceDecl, Import/FromImport/Include,
│                     #   ImportName, ConfigDirective, MetaDirective, Block
├── expressions.rs    # Expression, Literal/LiteralKind, Binary,    [548–755]
│                     #   Unary/UnaryOp, Call, MemberAccess, Index,
│                     #   Array, Tuple, Record, Arrow, Conditional,
│                     #   Assignment, StringInterpolation/Part
├── cmx.rs            # CmxElement, CmxAttribute(Value), CmxChild  [725–755]
├── types_ann.rs      # TypeAnnotation, TypeKind, TypeAccess,       [886–940]
│                     #   TypeParam, Visibility
└── display.rs        # expr_display + impl Display para Statement  [756–885, 955–990]
```

**Nota**: `expr_display` en ast es la fuente canónica — el duplicado de
`typeck.rs` (`expr_short_display`) y el de `wasm.rs` se unificarán aquí durante
la Fase 3 (adaptación del core). Queda pendiente en este refactor.

## 1.3 `cls-core/src/frontend/token.rs` → módulo `frontend/token/`

```
cls-core/src/frontend/token/
├── mod.rs         # re-exports (API estable)
├── token.rs       # enum Token + impl + Display              [token.rs:7–53, 291–312]
├── keyword.rs     # enum Keyword + Display                   [54–133, 344–410]
├── operator.rs    # enum Operator + impl + Display           [133–243, 411–457]
├── symbol.rs      # enum Symbol + impl + Display             [244–275, 327–343]
└── cmx.rs         # enum CmxToken + Display                  [276–290, 312–326]
```

## 1.4 `cls-runtime/src/` → carpeta `walker/` (marcar qué se depreca)

El intérprete es solo el AST-walker y **será deprecado tras 2.0-dev1**. Se mueve
todo lo que le pertenece a `cls-runtime/src/walker/` para tener una noción clara
de qué desaparecerá y qué se conserva.

```
cls-runtime/src/
├── walker/                    # ⚠️ TODO ESTE SUBÁRBOL SE DEPRECA CON EL WALKER
│   ├── mod.rs                 # re-exports (compat transitoria)
│   ├── interpreter.rs         # 123 KB — el intérprete
│   ├── value.rs               # Value/Promise/ClassDef/ClassInstance + is_truthy
│   ├── environment.rs         # Environment
│   ├── intrinsics.rs          # Intrinsics (globales del walker)
│   ├── sandbox.rs             # Sandbox
│   ├── gc.rs                  # GarbageCollector (stub)
│   ├── modules.rs             # helpers de módulos del walker
│   ├── host_api.rs            # host API del walker
│   ├── stdlib/                # métodos de primitivos del walker (tablas dispatch)
│   ├── resolver.rs            # ModuleResolver (trait del nodo para el walker)
│   ├── vfs/                   # VFS (VfsResolver, LocalFs, ZipFs) — lo usa clxr/clx (walker)
│   └── clslib.rs              # .clslib del walker (Lib.load) — nota: el
│                              #   concepto se reimplementará en cls-internals (Fase 2)
├── error_report.rs            # ✅ CONSERVAR (formateo de errores — lo usan JIT y walker)
├── error.rs                   # ✅ CONSERVAR (re-export de ClsError)
└── ffi.rs                     # ✅ CONSERVAR (NativeBackend — lo usan clx/clxb)
```

**`lib.rs` transitorio** (sin romper API): `pub mod walker;` + los mismos
`pub use` de hoy redirigidos a `walker::`.

**Nota**: `Interpreter`, `Intrinsics`, `ModuleResolver` y `Value` son consumidos
por `nodos/clx` (subcomando `run --ast-walker`) y `nodos/clxr` — los re-exports
mantienen la compilación sin tocar los nodos. La eliminación real ocurre en la
deprecación oficial del walker (post-2.0-dev1), no en este refactor.

**Criterio de conservar vs mover**:

| Archivo | Decisión | Razón |
|---|---|---|
| `error_report.rs` | conservar | Lo usa el JIT (formato de errores), el REPL y los bindings |
| `error.rs` | conservar | Re-export de `cls_core::error` |
| `ffi.rs` | conservar | `NativeBackend` es el contrato de extensiones nativas de `clx`/`clxb` |
| todo lo demás | mover a `walker/` | Es infraestructura del intérprete deprecado |

---

## 1.5 Criterios de aceptación de la Fase 1

- [ ] `cargo check` de TODO el workspace: 0 errores.
- [ ] `cargo test`: sin regresiones nuevas (revisar el pre-existente `host_call_wasmi`).
- [ ] `run-availible.ps1`: 25 PASS · `run-tests.ps1`: 20 PASS.
- [ ] Ningún archivo del backend supera ~900 líneas; la mayoría 200–500.
- [ ] Los consumidores externos (`clxb`, REPL, bindings) sin cambios de código.
- [ ] `git log` = una secuencia de commits atómicos por bloque (revertibles).

## 1.6 Notas

- El parser (96 KB) y el lexer (27 KB) quedan FUERA de esta fase — se tocarán
  solo si la Fase 3 lo exige.
- `wasm` conserva su nombre de módulo (`backend::wasm`) para no tocar imports en
  `cls-jit`; solo cambia de archivo a directorio.
