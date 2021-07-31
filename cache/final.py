app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 0
try:
    var_std_fs = app.getlib('fs')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 20
try:
    var_std_print  (var_std_fs.open)

except Exception as e:
    app.error(e, 'ErrorExecute', 20)

app.variables.pop()
