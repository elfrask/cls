app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
def t_tmp0(*arg):
    try:        f_rt = (var_String)
    except:        f_rt = (var_String)
    try:
        sta_var = var_Any
    except:
        try:        sta_var = var_Any
        except:
            app.error('the Any class not found', errores.ErrorName)
    try:
        var_xd = arg[0]
    except:
        var_xd = (None)
    app.dim(var_xd, sta_var)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 347
    try:
        var_print  (app.str['']("Saludos a")  ,  var_xd  ,  app.str['']("desde una funcion anonima"))

    except Exception as e:
        app.error(e, 'ErrorExecute', 347)

    app.foins()
    app.index = 412
    try:
        return app.dim((var_xd  +  var_xd  +  var_xd), f_rt)

    except Exception as e:
        app.error(e, 'ErrorExecute', 412)

    app.variables.pop()

    pass
app.foins()
app.index = 2
try:
    var_print  (app.str['']("opciones(1, 2, 3): "))

except Exception as e:
    app.error(e, 'ErrorExecute', 2)

app.foins()
app.index = 32
try:
    if 'var_nota' in constant: app.constE('var_nota')
    try:
        sta_var = var_Any
    except:
        try:
            sta_var = var_Any
        except:
            app.error('the Any class not found', 'ErrorName', app.index)
    try:
        var_nota = (var_input  (app.str['']("opcion: ")))
    except:
        var_nota = None
    sta_values['var_nota'] = sta_var
    var_nota = app.dim(var_nota, sta_var)
    

except Exception as e:
    app.error(e, 'ErrorExecute', 32)

app.foins()
app.index = 63
try:
    switch_val = (var_nota)
    if False:
        pass
    elif (app.str['']("1")) == (switch_val):
        app.foins()
        def var_holabb(*arg):
            try:
                f_rt = (var_str)
            except:
                f_rt = (var_str)
            app.variables.append([locals(), globals()])
            sta_values = {}
            constant = {}
            app.foins()
            app.index = 143
            try:
                var_print  (app.str['']("Este es la funcion 'Hola BB'"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 143)

            app.foins()
            app.index = 194
            try:
                return app.dim((app.str['']("retornado")), f_rt)

            except Exception as e:
                app.error(e, 'ErrorExecute', 194)

            app.variables.pop()

            pass
        app.foins()
        app.index = 233
        try:
            var_print  (app.str['']("la funcion retorno:")  ,  var_holabb  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 233)


        pass
    elif (app.str['']("2")) == (switch_val):
        app.foins()
        app.index = 309
        try:
            if 'var_FA' in constant: app.constE('var_FA')
            try:
                sta_var = var_Any
            except:
                try:
                    sta_var = var_Any
                except:
                    app.error('the Any class not found', 'ErrorName', app.index)
            try:
                var_FA = (t_tmp0)
            except:
                var_FA = None
            sta_values['var_FA'] = sta_var
            var_FA = app.dim(var_FA, sta_var)
            

        except Exception as e:
            app.error(e, 'ErrorExecute', 309)

        app.foins()
        app.index = 453
        try:
            var_FA  (app.str['']("frask"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 453)


        pass
    else:
        app.foins()
        app.index = 502
        try:
            var_print  (app.str['']("Caso no disponible"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 502)


        pass

except Exception as e:
    app.error(e, 'ErrorExecute', 63)

app.variables.pop()
