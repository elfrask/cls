app.variables.append(locals())
def var_char_getsh(*arg):
    try:
        f_rt = (var_char_void)
    except:
        f_rt = (var_std_void)
    try:
        var_char_v = arg[0]
    except:
        var_char_v = (None)
    app.dim(var_char_v, var_char_char)
    try:
        var_char_msg = arg[1]
    except:
        var_char_msg = (app.str[''](""))
    app.dim(var_char_msg, var_char_str)
    app.variables.append(locals())
    app.index = 60
    try:
        var_char_v.set  (var_char_input  (var_char_msg))

    except Exception as e:
        app.error(e, 'ErrorExecute', 60)

    app.variables.pop()

    pass
def var_std_main(*arg):
    try:
        f_rt = (var_std_int)
    except:
        f_rt = (var_std_int)
    app.variables.append(locals())
    app.index = 121
    try:
        var_std_value = (var_std_char  [app.values['int'](20)]  (app.str['']("Hola mundo")))

    except Exception as e:
        app.error(e, 'ErrorExecute', 121)

    app.index = 158
    try:
        var_char_getsh (var_std_value  ,  app.str['']("cual es tu nombre? "))

    except Exception as e:
        app.error(e, 'ErrorExecute', 158)

    app.index = 205
    try:
        var_std_print  (app.str['']("tu nombre es:")  ,  var_std_value)

    except Exception as e:
        app.error(e, 'ErrorExecute', 205)

    app.index = 240
    try:
        return app.dim((app.values['int'](0)), f_rt)

    except Exception as e:
        app.error(e, 'ErrorExecute', 240)

    app.variables.pop()

    pass
app.index = 257
try:
    var_std_main  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 257)

app.variables.pop()
