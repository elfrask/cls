
let var_Hola
var_Hola = Api.Function(  (...arg) => {
    let rt = var_Any

    let var_e = (arg[0]||(undefined))
    var_e = app.dim(var_e, var_Any)


    app.index = 0
    try {

        return app.dim(((var_e)), rt)

    } catch (e) {
        app.error(e, 'ErrorExec', 0)
    }

});
app.index = 39
try {
    var_print  (var_Hola({"tag":"Hola","arg":{},"nodes":[]}))
} catch (e) {
    app.error(e, 'ErrorExec', 39)
}