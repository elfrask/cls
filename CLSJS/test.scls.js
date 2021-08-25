
try {
    funca((...arg) => {
    let este = app.func_list[0];
    let rt = var_Any;

    let var_e = (arg[0]||(undefined));
    var_e = app.dim(var_e, var_undefined);


    try {
        var_e  *  Api.int(1)
    } catch (e) {
        app.error(e, 'ErrorExec', undefined)
    }

});
} catch (e) {
    app.error(e, 'ErrorExec', undefined)
}