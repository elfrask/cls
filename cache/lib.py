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
app.index = 94
try:
    var_std_export.Async = (var_std_fs.Async)

except Exception as e:
    app.error(e, 'ErrorExecute', 94)

app.foins()
app.index = 119
try:
    var_std_export.exist = (var_std_fs.exist)

except Exception as e:
    app.error(e, 'ErrorExecute', 119)

app.foins()
app.index = 144
try:
    var_std_export.ver = (app.fist( [app.values['int'](1)  ,  app.values['int'](0)  ,  app.values['int'](0)] ))

except Exception as e:
    app.error(e, 'ErrorExecute', 144)

app.foins()
app.index = 166
try:
    var_std_export.name = (app.str['']("FileSystem (fs)"))

except Exception as e:
    app.error(e, 'ErrorExecute', 166)

app.foins()
app.index = 199
try:
    var_std_export.unlink = (var_std_fs.delete)

except Exception as e:
    app.error(e, 'ErrorExecute', 199)

app.foins()
app.index = 226
try:
    var_std_export.mkdir = (var_std_fs.mkdir)

except Exception as e:
    app.error(e, 'ErrorExecute', 226)

app.foins()
app.index = 251
try:
    var_std_export.mkfile = (var_std_fs.mkfile)

except Exception as e:
    app.error(e, 'ErrorExecute', 251)

app.foins()
app.index = 278
try:
    var_std_export.copy = (var_std_fs.copy)

except Exception as e:
    app.error(e, 'ErrorExecute', 278)

app.foins()
app.index = 301
try:
    var_std_export.move = (var_std_fs.move)

except Exception as e:
    app.error(e, 'ErrorExecute', 301)

app.variables.pop()
