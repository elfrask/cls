app.variables.append(locals())
sta_values = stasta.get('tae', {})
stasta = {}
app.foins()
app.index = 0
try:
    var_std_os = app.getlib('os')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 20
try:
    var_std_print  (var_std_os.name  ())

except Exception as e:
    app.error(e, 'ErrorExecute', 20)

app.variables.pop()
