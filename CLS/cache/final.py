app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 0
try:
    var_lib = app.getlib('lib.scls')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
def var_main(*arg):
    try:
        f_rt = (var_Any)
    except:
        f_rt = (var_Any)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 50
    try:
        var_print  (app.str['']("Holis")  ,  var_lib.hola)

    except Exception as e:
        app.error(e, 'ErrorExecute', 50)

    app.foins()
    app.index = 80
    try:
        var_lib.error  ()

    except Exception as e:
        app.error(e, 'ErrorExecute', 80)

    app.variables.pop()

    pass
app.foins()
app.index = 99
try:
    var_main  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 99)

app.variables.pop()
