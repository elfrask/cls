const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b1b.wat', 'utf8');
const imports = t.match(/\(import "env" "[^"]+" \(func \(;(\d+);\)/g) || [];
console.log('numero de imports env:', imports.length);
console.log('primer:', imports[0]);
console.log('ultimo:', imports[imports.length - 1]);
// tipos 0, 59, 60, 61
const types = t.match(/\(type \(;(\d+);\) \(func[^)]*\)[^)]*\)[^)]*\)|\(type \(;(\d+);\) \(func\)\)/g) || [];
for (const ty of types) {
  const m = ty.match(/\(;(\d+);\)/);
  const n = parseInt(m[1], 10);
  if (n <= 2 || (n >= 58 && n <= 65)) console.log(ty);
}
