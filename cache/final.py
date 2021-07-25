app.variables.append(locals())
sta_values = {}
def t_tmp0(*arg):
    try:        f_rt = (var_std_Any)
    except:        f_rt = (var_std_Any)
    app.variables.append(locals())
    sta_values = {}
    app.variables.pop()

    pass
def t_tmp1(*arg):
    try:        f_rt = (var_std_Any)
    except:        f_rt = (var_std_Any)
    try:
        sta_var = var_std_str
    except:
        try:        sta_var = var_std_str
        except:
            app.error('the str class not found', errores.ErrorName)
    try:
        var_std_data = arg[0]
    except:
        var_std_data = (app.str[''](""))
    app.dim(var_std_data, sta_var)
    app.variables.append(locals())
    sta_values = {}
    def t_tmp0(*arg):
        try:        f_rt = (var_std_Any)
        except:        f_rt = (var_std_Any)
        try:
            sta_var = var_std_Any
        except:
            try:        sta_var = var_std_Any
            except:
                app.error('the Any class not found', errores.ErrorName)
        try:
            var_std_i = arg[0]
        except:
            var_std_i = (None)
        app.dim(var_std_i, sta_var)
        app.variables.append(locals())
        sta_values = {}
        app.foins()
        app.index = 208
        try:
            var_std_print  (app.str['']("i -w-:")  ,  var_std_i)

        except Exception as e:
            app.error(e, 'ErrorExecute', 208)

        app.variables.pop()

        pass
    app.foins()
    app.index = 132
    try:
        var_std_print  (app.str['']("tu data es:")  ,  var_std_data)

    except Exception as e:
        app.error(e, 'ErrorExecute', 132)

    app.foins()
    app.index = 165
    try:
        app.fist( [app.values['int'](1)  ,  app.values['int'](2)  ,  app.values['int'](3)  ,  app.values['int'](4)  ,  app.values['int'](5)  ,  app.values['int'](6)  ,  app.values['int'](7)  ,  app.values['int'](8)  ,  app.values['int'](9)] ) .forEach  (t_tmp0)

    except Exception as e:
        app.error(e, 'ErrorExecute', 165)

    app.variables.pop()

    pass
app.foins()
def var_std_readline(*arg):
    try:
        f_rt = (var_std_Any)
    except:
        f_rt = (var_std_Any)
    try:
        sta_var = var_std_str
    except:
        try:        sta_var = var_std_str
        except:
            app.error('the str class not found', errores.ErrorName)
    try:
        var_std_msg = arg[0]
    except:
        var_std_msg = (app.str[''](""))
    app.dim(var_std_msg, sta_var)
    try:
        sta_var = var_std_function
    except:
        try:        sta_var = var_std_function
        except:
            app.error('the function class not found', errores.ErrorName)
    try:
        var_std_callback = arg[1]
    except:
        var_std_callback = (app.fist( (t_tmp0) ))
    app.dim(var_std_callback, sta_var)
    app.variables.append(locals())
    sta_values = {}
    app.foins()
    app.index = 64
    try:
        var_std_callback  (var_std_input  (var_std_msg))

    except Exception as e:
        app.error(e, 'ErrorExecute', 64)

    app.variables.pop()

    pass
app.foins()
app.index = 90
try:
    var_std_readline  (app.str['']("nota: ")  ,  t_tmp1)

except Exception as e:
    app.error(e, 'ErrorExecute', 90)

app.variables.pop()
