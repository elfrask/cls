const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b1.wat', 'utf8');
for (const n of [60, 61, 62, 63]) {
  const m = t.indexOf(`(func (;${n};)`);
  if (m < 0) { console.log(`--- func ${n}: NO ENCONTRADA ---`); continue; }
  console.log(`=== func ${n} ===`);
  console.log(t.slice(m, m + 700));
  console.log();
}
