let CLSEngine= require('./CLSJS');
let fs= require('fs');
let N = `
`

let aplicacion = CLSEngine.App("Aplication")

function main(g) {
    let data = fs.readFileSync(g, "utf-8");
    let app = aplicacion.Script(data, g);
    let d_crudo = app.desline(data)
    let parseado = app.parselex(d_crudo);
    let estructurado = app.estructuration(parseado)
    let generado = app.generator(estructurado, "normal")
    let salida = app.jump(generado, 0)

    fs.writeFileSync(g +".map", JSON.stringify(estructurado, N, 4))
    fs.writeFileSync(g +".js", salida)
};




main(process.argv[2])


