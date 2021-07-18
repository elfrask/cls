app.variables.append(locals())
def var_char_getsh(*arg):
    try:
        f_rt = (var_char_void)
    except:
        f_rt = (var_std_void)
    try:
        sta_var = var_char_char
    except:
        try:        sta_var = var_std_char
        except:
            app.error('the char class not found', errores.ErrorName)
    try:
        var_char_v = arg[0]
    except:
        var_char_v = (None)
    app.dim(var_char_v, sta_var)
    try:
        sta_var = var_char_str
    except:
        try:        sta_var = var_std_str
        except:
            app.error('the str class not found', errores.ErrorName)
    try:
        var_char_msg = arg[1]
    except:
        var_char_msg = (app.str[''](""))
    app.dim(var_char_msg, sta_var)
    app.variables.append(locals())
    app.index = 61
    try:
        var_char_v.set  (var_std_input (var_char_msg))

    except Exception as e:
        app.error(e, 'ErrorExecute', 61)

    app.variables.pop()

    pass
def var_std_main(*arg):
    try:
        f_rt = (var_std_void)
    except:
        f_rt = (var_std_void)
    app.variables.append(locals())
    app.index = 261
    try:
        var_std_value = (var_std_char  [app.values['int'](10)]  (app.str['']("Hola mundo")))

    except Exception as e:
        app.error(e, 'ErrorExecute', 261)

    app.index = 298
    try:
        var_char_getsh (var_std_value  ,  app.str['']("cual es tu nombre? "))

    except Exception as e:
        app.error(e, 'ErrorExecute', 298)

    app.index = 345
    try:
        var_std_print  (app.str['']("tu nombre es:")  ,  var_std_value)

    except Exception as e:
        app.error(e, 'ErrorExecute', 345)

    app.variables.pop()

    pass
app.index = 380
try:
    var_std_main  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 380)

app.variables.pop()
