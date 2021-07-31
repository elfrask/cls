app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 0
try:
    var_std_py = app.getlib('pypkg')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 23
try:
    if 'var_std_fs' in constant: app.constE('var_std_fs')
    var_std_fs = app.dim((var_std_py.require  (app.str['']("fs.py"))), sta_values.get('var_std_fs', var_std_Any))

except Exception as e:
    app.error(e, 'ErrorExecute', 23)

app.foins()
app.index = 50
try:
    var_std_export.open = (var_std_fs.open)

except Exception as e:
    app.error(e, 'ErrorExecute', 50)

app.foins()
app.index = 73
try:
    var_std_export.dir = (var_std_fs.dir)

except Exception as e:
    app.error(e, 'ErrorExecute', 73)

app.foins()
app.index = 118
try:
    var_std_export.exist = (var_std_fs.exist)

except Exception as e:
    app.error(e, 'ErrorExecute', 118)

app.variables.pop()
