const fs = require('fs');
const t = fs.readFileSync('C:/Users/Frask/AppData/Local/Temp/opencode/b3b.wat', 'utf8');
// main = func 75
const m = t.indexOf('(func (;75;)');
console.log('=== main ===');
console.log(t.slice(m, m + 2600));
