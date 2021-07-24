app.variables.append(locals())
sta_values = {}
app.foins()
app.index = 0
try:
    var_std_test = app.getlib('lib/prueba.scls')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 34
try:
    var_std_p = app.getlib('prueba.scls').play

except Exception as e:
    app.error(e, 'ErrorExecute', 34)

app.foins()
app.index = 73
try:
    var_std_test.play  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 73)

app.variables.pop()
