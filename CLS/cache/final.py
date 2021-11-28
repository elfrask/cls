app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
def t_tmp0(*arg):
    try:        f_rt = (var_Any)
    except:        f_rt = (var_Any)
    try:
        sta_var = var_Any
    except:
        try:        sta_var = var_Any
        except:
            app.error('the Any class not found', errores.ErrorName)
    try:
        var_res = arg[0]
    except:
        var_res = (None)
    app.dim(var_res, sta_var)
    try:
        sta_var = var_Any
    except:
        try:        sta_var = var_Any
        except:
            app.error('the Any class not found', errores.ErrorName)
    try:
        var_err = arg[1]
    except:
        var_err = (None)
    app.dim(var_err, sta_var)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 31
    try:
        if 'var_funciona' in constant: app.constE('var_funciona')
        try:
            sta_var = var_Any
        except:
            try:
                sta_var = var_Any
            except:
                app.error('the Any class not found', 'ErrorName', app.index)
        try:
            var_funciona = (var_input  (app.str['']("funciona?: ")))
        except:
            var_funciona = None
        sta_values['var_funciona'] = sta_var
        var_funciona = app.dim(var_funciona, sta_var)
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 31)

    app.foins()
    app.index = 77
    try:
        if (var_funciona  ==  app.str['']("si")):
            app.foins()
            app.index = 109
            try:
                var_res  (app.str['']("si funciona!"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 109)


            pass
        else:
            app.foins()
            app.index = 150
            try:
                var_err  (app.str['']("no funciona :("))

            except Exception as e:
                app.error(e, 'ErrorExecute', 150)


            pass

    except Exception as e:
        app.error(e, 'ErrorExecute', 77)

    app.variables.pop()

    pass
def t_tmp1(*arg):
    try:        f_rt = (var_Any)
    except:        f_rt = (var_Any)
    try:
        sta_var = var_Any
    except:
        try:        sta_var = var_Any
        except:
            app.error('the Any class not found', errores.ErrorName)
    try:
        var_x = arg[0]
    except:
        var_x = (None)
    app.dim(var_x, sta_var)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 201
    try:
        var_print  (app.str['']("si,")  ,  var_x)

    except Exception as e:
        app.error(e, 'ErrorExecute', 201)

    app.variables.pop()

    pass
def t_tmp2(*arg):
    try:        f_rt = (var_Any)
    except:        f_rt = (var_Any)
    try:
        sta_var = var_Any
    except:
        try:        sta_var = var_Any
        except:
            app.error('the Any class not found', errores.ErrorName)
    try:
        var_x = arg[0]
    except:
        var_x = (None)
    app.dim(var_x, sta_var)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 239
    try:
        var_print  (app.str['']("no,")  ,  var_x)

    except Exception as e:
        app.error(e, 'ErrorExecute', 239)

    app.variables.pop()

    pass
app.foins()
app.index = 3
try:
    var_Promise  (t_tmp0) .then  (t_tmp1) .catch  (t_tmp2)

except Exception as e:
    app.error(e, 'ErrorExecute', 3)

app.variables.pop()
