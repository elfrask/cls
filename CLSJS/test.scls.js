
let var_main
var_main = (  (...arg) => {
    let este = var_main
    let rt = var_Any

    let var_msg = (arg[0]||( app.tstr['']("Saludos!") ))
    var_msg = app.dim(var_msg, var_String)


    try {
     var_print  ( var_msg ) 
    } catch (e) {
        app.error(e, 'ErrorExec', undefined)
    }

});