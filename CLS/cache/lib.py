app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
def var_error(*arg):
    try:
        f_rt = (var_Any)
    except:
        f_rt = (var_Any)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 21
    try:
        var_catch  (app.str['']("FALLO!")  ,  app.str['']("meh dyo anziedat :c"))

    except Exception as e:
        app.error(e, 'ErrorExecute', 21)

    app.variables.pop()

    pass
app.foins()
app.index = 65
try:
    var_export.hola = (app.str['']("Hola mundo"))

except Exception as e:
    app.error(e, 'ErrorExecute', 65)

app.foins()
app.index = 93
try:
    var_export.error = (var_error)

except Exception as e:
    app.error(e, 'ErrorExecute', 93)

app.variables.pop()
