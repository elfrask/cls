app.index = 0
try:
    def t_tmp_with(t_t):
        var_std_hola = t_t
        app.index = 31
        try:
            var_std_print  (var_std_hola)

        except Exception as e:
            app.error(e, 'ErrorExecute', 31)


        pass
    t_tmp_with(app.values['str']("Hola mundo"))
    del t_tmp_with

except Exception as e:
    app.error(e, 'ErrorExecute', 0)

app.index = 47
try:
    var_std_print  (var_std_hola)

except Exception as e:
    app.error(e, 'ErrorExecute', 47)

