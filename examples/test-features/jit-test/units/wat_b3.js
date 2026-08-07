const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b3.wat', 'utf8');
// buscar las funciones definidas (idx > imports). imports ~66 ahora (60 + record 7 + http 2)
const defs = [...t.matchAll(/\(func \(;(\d+);\)/g)].map(m => parseInt(m[1], 10));
const impMax = 69;
const defined = defs.filter(n => n > impMax);
console.log('funciones definidas:', defined.join(', '));
// tabla
const tb = t.indexOf('(table');
console.log('=== tabla ===');
console.log(tb >= 0 ? t.slice(tb, tb + 300) : 'NO HAY TABLA');
// elemento
const el = t.indexOf('(elem');
console.log('=== element ===');
console.log(el >= 0 ? t.slice(el, el + 300) : 'NO HAY ELEMENT');
// las funciones del ctor y metodos: buscar 'Contador' no; buscar global.set? mejor buscar por cuerpo
// buscar la ultima funcion (main) y las que tienen call_indirect
const ci = t.indexOf('call_indirect');
console.log('=== zona call_indirect ===');
console.log(ci >= 0 ? t.slice(ci - 700, ci + 300) : 'NO HAY call_indirect');
