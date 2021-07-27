app.variables.append(locals())
sta_values = stasta.get('tae', {})
stasta = {}
app.foins()
def var_std_main(me):
    private = (MD())
    var_std_private = private
    def t_get_atr(self, v): 
        return None
    me.__getattr__ = t_get_atr
    private.__getattr__ = t_get_atr
    def t_set_atr(self, a, v): 
        print('atr:', a)
        self.__dict__[a] = app.dim(v, sta_values.get('var_std_'+a, var_std_Any))
    me.__setattr__ = t_set_atr
    private.__setattr__ = t_set_atr

    app.variables.append(locals())
    sta_values = {}
    app.foins()
    app.index = 18
    try:
        try:
            sta_var = var_std_int
        except:
            try:
                sta_var = var_std_int
            except:
                app.error('the int class not found', 'ErrorName', app.index)
        try:
            var_std_i = (app.values['int'](10))
        except:
            var_std_i = None
        sta_values['var_std_i'] = sta_var
        me.i = app.dim(var_std_i, sta_var)
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 18)

    app.variables.pop()

    return me
var_std_main = (var_std_main)(MD())
app.foins()
app.index = 38
try:
    var_std_print  (var_std_main.i)

except Exception as e:
    app.error(e, 'ErrorExecute', 38)

app.foins()
app.index = 53
try:
    var_std_main.i = (app.str['']("1.2"))

except Exception as e:
    app.error(e, 'ErrorExecute', 53)

app.variables.pop()
