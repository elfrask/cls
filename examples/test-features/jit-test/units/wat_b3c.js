const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b3b2.wat', 'utf8');
const defs = [...t.matchAll(/\(func \(;(\d+);\)/g)].map(m => parseInt(m[1], 10));
console.log('definidas:', defs.filter(n => n > 68).join(', '));
const tb = t.indexOf('(table');
console.log('=== tabla ===');
console.log(t.slice(tb, tb + 80));
const el = t.indexOf('(elem');
console.log(t.slice(el, el + 80));
// funciones 72 y 73
for (const n of [71, 72, 73]) {
  const m = t.indexOf(`(func (;${n};)`);
  console.log(`=== func ${n} ===`);
  console.log(t.slice(m, m + 200));
}
// dispatch a.hablar en main: buscar 'local.set 9' o el primer call_indirect despues de "a hablar"
const s = t.indexOf('a hablar');
const ci = t.indexOf('call_indirect');
console.log('=== primer call_indirect (a.hablar?) ===');
console.log(t.slice(ci - 300, ci + 80));
