app.variables.append(locals())
sta_values = stasta.get('tae', {})
stasta = {}
app.foins()
app.index = 1
try:
    try:
        sta_var = var_std_intbit
    except:
        try:
            sta_var = var_std_intbit
        except:
            app.error('the intbit class not found', 'ErrorName', app.index)
    try:
        var_std_valor = (var_std_intbit  [app.values['int'](8)]  (app.values['int'](256)))
    except:
        var_std_valor = None
    sta_values['var_std_valor'] = sta_var
    var_std_valor = app.dim(var_std_valor, sta_var)

except Exception as e:
    app.error(e, 'ErrorExecute', 1)

app.foins()
app.index = 37
try:
    var_std_print  (app.str['']("valor:")  ,  var_std_valor)

except Exception as e:
    app.error(e, 'ErrorExecute', 37)

app.variables.pop()
