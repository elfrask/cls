app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
app.foins()
app.index = 68
try:
    
    
    
    def var_test():
        print("Hola mundo")

except Exception as e:
    app.error(e, 'ErrorExecute', 68)

app.foins()
app.index = 111
try:
    var_test  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 111)

app.foins()
def var_main(*arg):
    try:
        f_rt = (var_void)
    except:
        f_rt = (var_void)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 163
    try:
        if 'var_opcion' in constant: app.constE('var_opcion')
        try:
            sta_var = var_String
        except:
            try:
                sta_var = var_String
            except:
                app.error('the String class not found', 'ErrorName', app.index)
        try:
            var_opcion = (var_input  (app.str['']("opcion 1, 2, 3: ")))
        except:
            var_opcion = None
        sta_values['var_opcion'] = sta_var
        var_opcion = app.dim(var_opcion, sta_var)
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 163)

    app.foins()
    app.index = 212
    try:
        switch_val = (var_opcion)
        if False:
            pass
        elif (app.str['']("1")) == (switch_val):
            app.foins()
            app.index = 338
            try:
                var_print  (app.str['']("Opcion 1"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 338)


            pass
        elif (app.str['']("2")) == (switch_val):
            app.foins()
            app.index = 401
            try:
                var_print  (app.str['']("Opcion 2"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 401)


            pass
        elif (app.str['']("3")) == (switch_val):
            app.foins()
            app.index = 464
            try:
                var_print  (app.str['']("Opcion 3"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 464)


            pass
        else:
            app.foins()
            app.index = 265
            try:
                var_print  (app.str['']("Opcion por defecto"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 265)


            pass

    except Exception as e:
        app.error(e, 'ErrorExecute', 212)

    app.variables.pop()

    pass
app.foins()
app.index = 505
try:
    var_main  ()

except Exception as e:
    app.error(e, 'ErrorExecute', 505)

app.foins()
def var_Persona():
    class private:pass
    class me:pass
    def t_get_atr(self, v): 
        return None
    me.__getattr__ = t_get_atr
    private.__getattr__ = t_get_atr
    def t_set_atr(self, a, v): 
        self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_Any))
    me.__setattr__ = t_set_atr
    private.__setattr__ = t_set_atr
    me.default = lambda x: me()
    me.__str__ = lambda x: '<Struct Persona>'
    me._str = me.__str__
    me.__repl__ = me.__str__

    var_private = private()
    private = var_private
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.index = 645
    try:
        if 'var_x' in constant: app.constE('var_x')
        try:
            sta_var = var_int
        except:
            try:
                sta_var = var_int
            except:
                app.error('the int class not found', 'ErrorName', app.index)
        try:
            var_x = (app.values['int'](0))
        except:
            var_x = None
        sta_values['var_x'] = sta_var
        me.x = app.dim(var_x, sta_var)
        
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 645)

    app.index = 660
    try:
        if 'var_nombre' in constant: app.constE('var_nombre')
        try:
            sta_var = var_char
        except:
            try:
                sta_var = var_char
            except:
                app.error('the char class not found', 'ErrorName', app.index)
        try:
            var_nombre = (var_char  [app.values['int'](100)]  (app.str['']("texto")))
        except:
            var_nombre = None
        sta_values['var_nombre'] = sta_var
        me.nombre = app.dim(var_nombre, sta_var)
        
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 660)

    app.index = 698
    try:
        if 'var_peso' in constant: app.constE('var_peso')
        try:
            sta_var = var_float
        except:
            try:
                sta_var = var_float
            except:
                app.error('the float class not found', 'ErrorName', app.index)
        try:
            var_peso = (app.values['float'](1.0))
        except:
            var_peso = None
        sta_values['var_peso'] = sta_var
        me.peso = app.dim(var_peso, sta_var)
        
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 698)

    app.variables.pop()

    return me
var_Persona = (var_Persona)()
app.foins()
app.index = 720
try:
    if 'var_xx' in constant: app.constE('var_xx')
    try:
        sta_var = var_Persona
    except:
        try:
            sta_var = var_Persona
        except:
            app.error('the Persona class not found', 'ErrorName', app.index)
    try:
        var_xx = (None)
    except:
        var_xx = None
    sta_values['var_xx'] = sta_var
    var_xx = app.dim(var_xx, sta_var)
    

except Exception as e:
    app.error(e, 'ErrorExecute', 720)

app.foins()
app.index = 733
try:
    var_print  (var_xx.peso)

except Exception as e:
    app.error(e, 'ErrorExecute', 733)

app.variables.pop()
