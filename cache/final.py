app.variables.append(locals())
sta_values = {}
app.foins()
app.index = 0
try:
    var_std_fs = app.getlib('fs')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 20
try:
    var_std_file = app.dim((var_std_fs.open  (app.str['']("test.scls"))), sta_values.get('var_std_file', var_std_Any))

except Exception as e:
    app.error(e, 'ErrorExecute', 20)

app.foins()
app.index = 50
try:
    var_std_print  (var_std_file.read  ())

except Exception as e:
    app.error(e, 'ErrorExecute', 50)

app.variables.pop()
