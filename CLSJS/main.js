let CLSAPP= require('./CLSJS');
let fs= require('fs');
let N = `
`


function main(g) {
    let data = fs.readFileSync(g, "utf-8");
    let app = CLSAPP.clsapp();
    let d_crudo = app.desline(data, g)
    let parseado = app.parselex(d_crudo);
    let estructurado = app.estructuration(parseado)
    let salida = app.generator(estructurado, "normal")

    fs.writeFileSync(g +".json", JSON.stringify(estructurado, N, 4))
    fs.writeFileSync(g +".js", salida)
};




main(process.argv[2])


