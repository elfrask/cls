const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b1t.wat', 'utf8');
// encontrar la funcion init_globals: busca 'global.set 1' cerca del inicio de funciones
// buscar funciones definidas (no imports) por su indice
const defs = [...t.matchAll(/\(func \(;(\d+);\)/g)].map(m => parseInt(m[1], 10));
const importMax = 59;
console.log('funciones definidas (idx):', defs.filter(n => n > importMax).join(', '));
// buscar init_globals: cuerpo con 'global.set 1'
for (const n of defs) {
  const m = t.indexOf(`(func (;${n};)`);
  const slice = t.slice(m, m + 400);
  if (slice.includes('global.set 1')) {
    console.log(`=== func ${n} (init_globals?) ===`);
    console.log(slice);
    break;
  }
}
// mutar: buscar 'global.set 3' y concat
const mi = t.indexOf('str_concat');
if (mi > 0) {
  console.log('=== zona str_concat ===');
  console.log(t.slice(mi - 400, mi + 200));
}
