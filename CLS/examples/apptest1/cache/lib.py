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
app.index = 24
try:
    if 'var_std_os' in constant: app.constE('var_std_os')
    var_std_os = app.dim((var_std_py.require  (app.str['']("os.py"))), sta_values.get('var_std_os', var_std_Any))

except Exception as e:
    app.error(e, 'ErrorExecute', 24)

app.foins()
app.index = 51
try:
    var_std_export.name = (var_std_os.osname)

except Exception as e:
    app.error(e, 'ErrorExecute', 51)

app.variables.pop()
