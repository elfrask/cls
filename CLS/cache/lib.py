app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 0
try:
    var_py = app.getlib('pypkg')

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.foins()
app.index = 24
try:
    if 'var_req' in constant: app.constE('var_req')
    var_req = app.dim((var_py.require  (app.str['']("req.py"))), sta_values.get('var_req', var_Any))

except Exception as e:
    app.error(e, 'ErrorExecute', 24)

app.foins()
app.index = 54
try:
    var_export.request = (var_req.request)

except Exception as e:
    app.error(e, 'ErrorExecute', 54)

app.foins()
app.index = 84
try:
    var_export.arequest = (var_req.arequest)

except Exception as e:
    app.error(e, 'ErrorExecute', 84)

app.foins()
app.index = 116
try:
    var_export.codes = (var_req.codes)

except Exception as e:
    app.error(e, 'ErrorExecute', 116)

app.foins()
app.index = 142
try:
    var_export.session = (var_req.Session)

except Exception as e:
    app.error(e, 'ErrorExecute', 142)

app.foins()
app.index = 172
try:
    var_export.ver = (app.fist( [app.values['int'](0)  ,  app.values['int'](1)  ,  app.values['int'](0)] ))

except Exception as e:
    app.error(e, 'ErrorExecute', 172)

app.foins()
app.index = 196
try:
    var_export.name = (app.str['']("[RequestLibrary (request)]"))

except Exception as e:
    app.error(e, 'ErrorExecute', 196)

app.variables.pop()
