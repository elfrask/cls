app.variables.append(locals())
sta_values = {}
app.foins()
def var_std_play(*arg):
    try:
        f_rt = (var_std_void)
    except:
        f_rt = (var_std_void)
    app.variables.append(locals())
    sta_values = {}
    app.foins()
    app.index = 19
    try:
        var_std_print  (app.str['']("Testeando..."))

    except Exception as e:
        app.error(e, 'ErrorExecute', 19)

    app.foins()
    app.index = 46
    try:
        var_std_print  (app.str['']("Testeo completo!"))

    except Exception as e:
        app.error(e, 'ErrorExecute', 46)

    app.variables.pop()

    pass
app.foins()
app.index = 77
try:
    var_std_export.play = (var_std_play)

except Exception as e:
    app.error(e, 'ErrorExecute', 77)

app.variables.pop()
