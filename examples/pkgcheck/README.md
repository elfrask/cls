# `pkgcheck` — SHA-256 en CLS puro + verificación (prueba #8)

Implementa **SHA-256 (FIPS 180-4) en CLS puro** (0 FFI) y lo valida contra
oráculos independientes. Es el núcleo del futuro gestor de paquetes
(verificación de integridad de `.clsapp`/`.clslib`): si el SHA-256 en CLS ≠
`sha256sum`, el gestor es inútil.

## Oráculos

1. **`hashlib` (Python)** (`tests/oracle.py`) — reproduce **exactamente** los
   inputs de `src/main.clsx` (mismo PRNG) y compara el digest byte a byte.
2. **`certutil -hashfile SHA256`** (Windows) — cross-check de **archivo**,
   independiente de Python (`src/hashfile.clsx` lee el archivo vía el nodo `fs`).

## Estructura

```
pkgcheck/
├── cls.json
├── verify.ps1              # corre main.clsx + oraculo hashlib + certutil
├── src/
│   ├── sha256.clsx         # SHA-256 puro (arithmetica 32-bit, rotaciones)
│   ├── hashfile.clsx       # sha256 de un archivo (nodo fs)
│   └── main.clsx           # emite 205 digests + linea corrupt=
├── tests/oracle.py         # oraculo hashlib
└── out/                    # salida generada (gitignore)
```

## Correr

```powershell
cargo build --bin clx
powershell -File examples\pkgcheck\verify.ps1
```

## Qué prueba (del lenguaje)

- **Aritmética de 32 bits** sobre `int` (i64) con máscara `0xFFFFFFFF` (sin
  literales hex: las constantes van en decimal).
- **Bitwise** `^`, `&`, `|` y **shifts** `<<`/`>>` (rotaciones `rotr`); overflow
  modular.
- **Arrays** (`int[]`, `int[][]`), `while`, `for each`, módulos fuente
  (`import "sha256" as sha; sha::sha256Hex(...)`), intrinsics `char`/`ord`,
  `String.toBytes()`.
- **Vectores NIST**: `""`, `"abc"`, 448-bit, 896-bit y 1.000.000 `'a'`.
- **200 inputs pseudoaleatorios** (LCG determinista idéntico en CLS y Python).
- **Integridad**: corromper 1 bit ⇒ hash distinto.

## Resultado

`verify.ps1` termina en `RESULTADO: OK` cuando los 205 digests coinciden con
`hashlib` byte a byte, la corrupción de 1 bit cambia el hash y `certutil`
coincide sobre el archivo.
