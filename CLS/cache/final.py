app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
def t_tmp0(*arg):
    try:        f_rt = (var_int)
    except:        f_rt = (var_int)
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
    app.index = 189
    try:
        return app.dim((var_int  (var_x  [app.str['']("value")])), f_rt)

    except Exception as e:
        app.error(e, 'ErrorExecute', 189)

    app.variables.pop()

    pass
app.foins()
app.index = 0
try:
    var_req = app.getlib('request')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 26
try:
    if 'var_valor' in constant: app.constE('var_valor')
    try:
        sta_var = var_Array
    except:
        try:
            sta_var = var_Array
        except:
            app.error('the Array class not found', 'ErrorName', app.index)
    try:
        var_valor = (app.fist( [] ))
    except:
        var_valor = None
    sta_values['var_valor'] = sta_var
    var_valor = app.dim(var_valor, sta_var)
    

except Exception as e:
    app.error(e, 'ErrorExecute', 26)

app.foins()
app.index = 49
try:
    for var_i in (var_range  (app.values['int'](0)  ,  app.values['int'](100))):
        app.foins()
        app.index = 82
        try:
            var_valor.push  (app.fist( {app.str['']('value')  :  var_String  (var_i)} ))

        except Exception as e:
            app.error(e, 'ErrorExecute', 82)


        pass

except Exception as e:
    app.error(e, 'ErrorExecute', 49)

app.foins()
app.index = 160
try:
    var_print  (var_valor.map  (t_tmp0))

except Exception as e:
    app.error(e, 'ErrorExecute', 160)

app.variables.pop()
