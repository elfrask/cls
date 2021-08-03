app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 1
try:
    var_std_catch  (app.str['']("me dio la gana de fallar")  ,  app.str['']("FALLO PVT0"))

except Exception as e:
    app.error(e, 'ErrorExecute', 1)

app.variables.pop()
