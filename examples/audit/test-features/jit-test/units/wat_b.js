const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b1b.wat', 'utf8');
// types 58..66 crudos
const raw = t.match(/\(type \(;5\d;\)[^)]*\([^)]*\)[^)]*\)[^)]*\)|\(type \(;6\d;\)[^)]*\([^)]*\)[^)]*\)[^)]*\)/g) || [];
console.log('=== types ===');
for (const r of raw) console.log(r);
for (const n of [60, 61, 62, 63, 64]) {
  const m = t.indexOf(`(func (;${n};)`);
  if (m < 0) { console.log(`--- func ${n}: NO ENCONTRADA`); continue; }
  console.log(`=== func ${n} ===`);
  console.log(t.slice(m, m + 600));
  console.log();
}
