app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
stasta = {}
app.foins()
app.index = 1
try:
    var_std_print  (app.values['int'](12)  app.values['int'](23))

except Exception as e:
    app.error(e, 'ErrorExecute', 1)

app.variables.pop()
