app.variables.append(locals())
sta_values = {}
app.foins()
app.index = 0
try:
    var_std_fs = app.getlib('fs')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 22
try:
    var_std_print  (var_std_on)

except Exception as e:
    app.error(e, 'ErrorExecute', 22)

app.variables.pop()
