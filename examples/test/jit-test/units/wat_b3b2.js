const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b3b.wat', 'utf8');
const defs = [...t.matchAll(/\(func \(;(\d+);\)/g)].map(m => parseInt(m[1], 10));
console.log('definidas:', defs.filter(n => n > 68).join(', '));
console.log('=== tabla ===');
const tb = t.indexOf('(table');
console.log(t.slice(tb, tb + 120));
const el = t.indexOf('(elem');
console.log(t.slice(el, el + 120));
// buscar i64.load con 'i32.wrap' mal, y el is
const ci = t.indexOf('call_indirect');
console.log('=== zona call_indirect (p.hablar) ===');
console.log(t.slice(ci - 500, ci + 150));
