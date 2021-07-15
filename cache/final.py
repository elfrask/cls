class tmp_class_std_main():
    def __dict__():pass
def tmp_var_std_main(obj):
    exportar = {}
    me = obj
    def out(*arg): return me(*arg)
    def p_call(o):
        def eo(*arg):
            o(me, *arg)
        return eo
    def var_std_main(self, *arg):
        try:
            f_rt = (var_std_Any)
        except:
            f_rt = (var_std_Any)
        var_std_me = self
        app.index = 45
        try:
            var_std_me.v = (app.values['int'](120))

        except Exception as e:
            app.error(e, 'ErrorExecute', 45)


        pass
    me.__init__ = var_std_main
    var_std_main = p_call(var_std_main)
    def var_std_get(self, *arg):
        try:
            f_rt = (var_std_Any)
        except:
            f_rt = (var_std_Any)
        var_std_me = self
        app.index = 90
        try:
            return app.dim(((var_std_me.v)), f_rt)

        except Exception as e:
            app.error(e, 'ErrorExecute', 90)


        pass
    me.get = var_std_get
    var_std_get = p_call(var_std_get)

    out.__export__ = exportar
    me.__export__ = exportar
    return out
var_std_main = tmp_var_std_main(tmp_class_std_main)
var_std_main.__clase__ = tmp_class_std_main
def var_std_lol(*arg):
    try:
        f_rt = (var_std_Any)
    except:
        f_rt = (var_std_Any)
    try:
        var_std_i = arg[0]
    except:
        var_std_i = (None)
    app.dim(var_std_i, var_std_main)
    app.index = 162
    try:
        var_std_print  (var_std_i.get  ())

    except Exception as e:
        app.error(e, 'ErrorExecute', 162)


    pass
app.index = 181
try:
    var_std_lol  (var_std_main  ())

except Exception as e:
    app.error(e, 'ErrorExecute', 181)

