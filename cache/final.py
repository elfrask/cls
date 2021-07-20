app.variables.append(locals())
app.foins()
app.foins()
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
    app.foins()
    app.index = 61
    try:
        var_char_v.set  (var_std_input (var_char_msg))

    except Exception as e:
        app.error(e, 'ErrorExecute', 61)

    app.variables.pop()

    pass
app.foins()
app.foins()
def var_std_main(*arg):
    try:
        f_rt = (var_std_void)
    except:
        f_rt = (var_std_void)
    app.variables.append(locals())
    app.foins()
    app.index = 328
    try:
        var_std_value = (var_std_char  [app.values['int'](20)]  ())

    except Exception as e:
        app.error(e, 'ErrorExecute', 328)

    app.foins()
    app.index = 386
    try:
        while (var_std_true):
            app.foins()
            app.index = 408
            try:
                var_char_getsh (var_std_value  ,  app.str['']("pon una nota de 20 caracteres: "))

            except Exception as e:
                app.error(e, 'ErrorExecute', 408)

            app.foins()
            app.index = 471
            try:
                if (var_std_input  (app.str['']("esto es correcto (Y/N)? "))  in  [app.str['']("y")  ,  app.str['']("Y")]):
                    app.foins()
                    app.index = 532
                    try:
                        break

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 532)


                    pass

            except Exception as e:
                app.error(e, 'ErrorExecute', 471)


            pass

    except Exception as e:
        app.error(e, 'ErrorExecute', 386)

    app.foins()
    app.index = 551
    try:
        var_std_print  (app.str['']("nota:")  ,  [var_std_value  ,  app.str['c']('test')])

    except Exception as e:
        app.error(e, 'ErrorExecute', 551)

    app.foins()
    app.index = 589
    try:
        var_std_print  ()

    except Exception as e:
        app.error(e, 'ErrorExecute', 589)

    app.foins()
    app.index = 602
    try:
        var_std_print_debug  (app.str['']("saludos"))

    except Exception as e:
        app.error(e, 'ErrorExecute', 602)

    app.variables.pop()

    pass
app.foins()
app.index = 630
try:
    var_std_main  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 630)

app.variables.pop()
