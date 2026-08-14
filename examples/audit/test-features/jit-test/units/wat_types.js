const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b1b.wat', 'utf8');
const types = t.match(/\(type \(;(\d+);\) \(func[^)]*\([^)]*\)[^)]*\)[^)]*\)/g) || [];
console.log('=== types (del 55 al 70) ===');
for (const line of types) {
  const m = line.match(/\(type \(;(\d+);\)/);
  const n = parseInt(m[1], 10);
  if (n >= 55) console.log(line.replace(/\n/g, ''));
}
console.log('=== error ===');
const err = t.match(/Caused by:[\s\S]{0,200}/);
console.log(err ? err[0] : '');
