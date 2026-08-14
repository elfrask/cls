const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b1t.wat', 'utf8');
const defs = [...t.matchAll(/\(func \(;(\d+);\)/g)].map(m => parseInt(m[1], 10)).filter(n => n > 59);
console.log('funciones definidas:', defs.join(', '));
// init_globals: buscar el cuerpo con global.set 1
for (const n of defs) {
  const m = t.indexOf(`(func (;${n};)`);
  const slice = t.slice(m, m + 900);
  if (slice.includes('global.set 1')) {
    console.log(`=== func ${n} (init_globals) ===`);
    console.log(slice);
  }
}
