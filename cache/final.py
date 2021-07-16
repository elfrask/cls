class tmp_class_std_test():
    def __dict__():pass
def tmp_var_std_test(obj):
    exportar = {}
    me = obj
    def out(*arg): return me(*arg)
    def p_call(o):
        def eo(*arg):
            o(me, *arg)
        return eo
    def var_std_main(self, *arg):
        try:
            f_rt = (var_std_void)
        except:
            f_rt = (var_std_void)
        var_std_me = self
        try:
            var_std_o = arg[0]
        except:
            var_std_o = (app.values['int'](10))
        app.dim(var_std_o, var_std_int)
        app.index = 49
        try:
            var_std_me.v = (var_std_o)

        except Exception as e:
            app.error(e, 'ErrorExecute', 49)


        pass
    me.__init__ = var_std_main
    var_std_main = p_call(var_std_main)
    def var_std__add(self, *arg):
        try:
            f_rt = (var_std_test)
        except:
            f_rt = (var_std_test)
        var_std_me = self
        try:
            var_std_obj = arg[0]
        except:
            var_std_obj = (None)
        app.dim(var_std_obj, var_std_Any)
        app.index = 92
        try:
            return app.dim(((var_std_test  (var_std_obj.v  +  var_std_me.v))), f_rt)

        except Exception as e:
            app.error(e, 'ErrorExecute', 92)


        pass
    me.__add__ = var_std__add
    var_std__add = p_call(var_std__add)

    out.__export__ = exportar
    me.__export__ = exportar
    return out
var_std_test = tmp_var_std_test(tmp_class_std_test)
var_std_test.__clase__ = tmp_class_std_test
def var_std_lol(*arg):
    try:
        f_rt = (var_std_void)
    except:
        f_rt = (var_std_void)
    try:
        var_std_i = arg[0]
    except:
        var_std_i = (None)
    app.dim(var_std_i, var_std_test)
    app.index = 172
    try:
        var_std_print  ((var_std_i  +  var_std_i  +  var_std_i  +  var_std_i  +  var_std_i  +  var_std_i  +  var_std_i  +  var_std_i) .v)

    except Exception as e:
        app.error(e, 'ErrorExecute', 172)


    pass
app.index = 203
try:
    var_std_lol  (var_std_test  (app.values['int'](20)))

except Exception as e:
    app.error(e, 'ErrorExecute', 203)

