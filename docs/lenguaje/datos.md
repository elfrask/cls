# Colecciones y strings (runtime)

Arrays, tuplas, records y strings en runtime. Los métodos de primitivos se
resuelven por dispatch tables sin boxing (`cls-runtime/src/stdlib/primitive.rs`)
y se compilan a llamadas directas en el JIT (ver `runtime/jit.md`).

## Arrays (mutables)

```clx
var a = [3, 1, 2];
a.push(4);
var last = a.pop();
a.shift();
a.unshift(9);
a.reverse();
print(a.length);       # getter
print(a.indexOf(3));   # -1 si no está
print(a.includes(3));
print(a.join("-"));    # acepta separador
```

- **Mutadores con write-back**: `push`, `pop`, `shift`, `unshift`, `reverse`
  mutan el array y reasignan automáticamente la variable del receiver
  (`arr.push(x)` muta `arr`).
- `map(fn)`: aplica la función a cada elemento y devuelve un array nuevo.
- Índices fuera de rango → error `Índice fuera de rango`.
- Anidados: `a[1][0]`.

```clx
var anidado = [[1, 2], [3, 4]];
print(anidado[1][0]);   # 3
```

## Tuplas (inmutables)

```clx
var t = (10, 20, 30);
t[0];              # 10 (lectura)
t[0] = 99;         # ERROR: las tuplas son inmutables
t.length;
t.join(",");
```

- Heterogéneas por posición: `(1, "dos", 3.0)`.
- Iterables con `for each`:

```clx
for each x in (t) {
    print(x);
}
```

## Records (diccionarios)

```clx
var r = {nombre: "Ana", edad: 30, activo: true};
r.nombre;              # acceso con punto
r["clave"];            # acceso con índice
r.edad = 31;           # asignación
r.keys();              # array con las claves
r.values();            # array con los valores
r.has("nombre");       # bool
r.length;              # getter
r.size;                # getter
```

## Strings (inmutables)

```clx
var s = "Hello World";
s.upper();
s.lower();
s.trim();
s.contains("World");
s.startsWith("He");
s.endsWith("ld");
s.isEmpty();
s.toString();
s.length;              # getter
```

- Inmutables: `s = s.upper();` devuelve un string nuevo.
- `length`: bytes en el JIT (unicode por carácter en el walker).
- Concatenación con `+`: `"a" + "b"`.
- Interpolación con `$var` y `${expr}` (ver `sintaxis.md`). Backticks también
  interpolan: `` `Template $nombre ${edad + 1}` ``.

## Uso con `with`

`with` itera sobre expresiones; con records itera las entradas (o el valor
único en otros casos):

```clx
with tmp in (40 + 2) {
    print("with:", tmp);
}
```

## Iteración

`for each` recorre arrays y tuplas, con o sin índice:

```clx
for each v in (arr) { }
for each v and idx in (arr) { }
```