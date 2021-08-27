
let var_lista
var_main = (() => {
    let este = var_main;
    let var_me = {};
    let var_private = {};
    if (!este.__class__)Object.assign(este, {__class__:"Namespace", __clase__:Api.Namespace});
    let me = var_me;
    let private = var_private;

    me.saluda = Api.Function(  (...arg) => {
        let este = me.saluda
        let rt = var_float

        app.index = 0
        try {

            return app.dim(((Api.int(12))), rt)

        } catch (e) {
            app.error(e, 'ErrorExec', 0)
        }

    });

    return Api.Namespace(me)
});var_main = (var_main)();
app.index = 108
try {
    var_print  (Api.obj({'hola' :  app.tstr['']("saludos")})  .hola)
} catch (e) {
    app.error(e, 'ErrorExec', 108)
}
app.index = 146
try {


        var_lista = (Api.Array([Api.int(1) ,  Api.int(2) ,  Api.int(3) ,  Api.int(4)])  .map  (funca((...arg) => {
    let este = app.func_list[0];
    let rt = var_String;

    let var_e = (arg[0]||(undefined));
    var_e = app.dim(var_e, var_Any);


    app.index = 0
    try {

        return app.dim(((app.tstr['']("valor: ")  +  var_e)), rt)

    } catch (e) {
        app.error(e, 'ErrorExec', 0)
    }

})));
        var_lista = app.dim(var_lista, var_Array)
        


} catch (e) {
    app.error(e, 'ErrorExec', 146)
}
app.index = 224
try {
    var_print  (var_lista  [Api.int(2)])
} catch (e) {
    app.error(e, 'ErrorExec', 224)
}