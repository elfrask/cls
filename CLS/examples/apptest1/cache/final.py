app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 1
try:
    var_std_print  (app.str['']("Hola mundo"))

except Exception as e:
    app.error(e, 'ErrorExecute', 1)

app.variables.pop()
