app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
def var_suma(*arg):
    try:
        f_rt = (var_int)
    except:
        f_rt = (var_int)
    try:
        sta_var = var_int
    except:
        try:        sta_var = var_int
        except:
            app.error('the int class not found', errores.ErrorName)
    try:
        var_a = arg[0]
    except:
        var_a = (app.values['int'](0))
    app.dim(var_a, sta_var)
    try:
        sta_var = var_int
    except:
        try:        sta_var = var_int
        except:
            app.error('the int class not found', errores.ErrorName)
    try:
        var_b = arg[1]
    except:
        var_b = (app.values['int'](0))
    app.dim(var_b, sta_var)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 40
    try:
        return app.dim((var_a  +  var_b), f_rt)

    except Exception as e:
        app.error(e, 'ErrorExecute', 40)

    app.variables.pop()

    pass
app.foins()
app.index = 58
try:
    var_print  (var_suma  (app.values['int'](10)  ,  app.values['int'](20)))

except Exception as e:
    app.error(e, 'ErrorExecute', 58)

app.variables.pop()
