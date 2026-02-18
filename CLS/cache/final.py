app.variables.append([locals(), globals()])
sta_values = stasta.get('tae', {})
constant   = stasta.get('const', [])
stasta = {}
def t_tmp0(*arg):
    try:        f_rt = (var_String)
    except:        f_rt = (var_String)
    try:
        sta_var = var_Any
    except:
        try:        sta_var = var_Any
        except:
            app.error('the Any class not found', errores.ErrorName)
    try:
        var_xd = arg[0]
    except:
        var_xd = (None)
    app.dim(var_xd, sta_var)
    app.variables.append([locals(), globals()])
    sta_values = {}
    constant = {}
    app.foins()
    app.index = 644
    try:
        var_print  (app.str['']("Saludos a")  ,  var_xd  ,  app.str['']("desde una funcion anonima"))

    except Exception as e:
        app.error(e, 'ErrorExecute', 644)

    app.foins()
    app.index = 709
    try:
        return app.dim((var_xd  +  var_xd  +  var_xd), f_rt)

    except Exception as e:
        app.error(e, 'ErrorExecute', 709)

    app.variables.pop()

    pass
app.foins()
app.index = 1
try:
    def var_ns():
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

        var_private = private()
        private = var_private
        app.variables.append([locals(), globals()])
        sta_values = {}
        constant = {}
        app.foins()
        app.index = 20
        try:
            app.index = 20
            try:
                if 'var_value' in constant: app.constE('var_value')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_value = (app.str['']("NS: test.scls"))
                except:
                    var_value = None
                sta_values['var_value'] = sta_var
                me.value = app.dim(var_value, sta_var)
                
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 20)


        except Exception as e:
            app.error(e, 'ErrorExecute', 20)

        app.variables.pop()

        return NS(me())
    var_ns = (var_ns)()

except Exception as e:
    app.error(e, 'ErrorExecute', 1)

app.foins()
app.index = 52
try:
    def var_useModules():
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

        var_private = private()
        private = var_private
        app.variables.append([locals(), globals()])
        sta_values = {}
        constant = {}
        app.foins()
        def var_hi(*arg):
            try:
                f_rt = (var_Any)
            except:
                f_rt = (var_Any)
            app.variables.append([locals(), globals()])
            sta_values = {}
            constant = {}
            app.foins()
            app.index = 100
            try:
                var_print  (app.str['']("Hola!!, soy un modulo!"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 100)

            app.variables.pop()

            pass
        me.hi = var_hi
        app.foins()
        app.index = 144
        try:
            def var_subModule():
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

                var_private = private()
                private = var_private
                app.variables.append([locals(), globals()])
                sta_values = {}
                constant = {}
                app.foins()
                def var_hi(*arg):
                    try:
                        f_rt = (var_Any)
                    except:
                        f_rt = (var_Any)
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 199
                    try:
                        var_print  (app.str['']("Hola!!, soy un submodulo!"))

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 199)

                    app.variables.pop()

                    pass
                me.hi = var_hi
                app.variables.pop()

                return me()
            me.subModule = (var_subModule)()

        except Exception as e:
            app.error(e, 'ErrorExecute', 144)

        app.variables.pop()

        return me()
    var_useModules = (var_useModules)()

except Exception as e:
    app.error(e, 'ErrorExecute', 52)

app.foins()
app.index = 256
try:
    var_print  (app.str['']("opciones(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15): "))

except Exception as e:
    app.error(e, 'ErrorExecute', 256)

app.foins()
app.index = 328
try:
    app.index = 328
    try:
        if 'var_nota' in constant: app.constE('var_nota')
        try:
            sta_var = var_Any
        except:
            try:
                sta_var = var_Any
            except:
                app.error('the Any class not found', 'ErrorName', app.index)
        try:
            var_nota = (var_input  (app.str['']("opcion: ")))
        except:
            var_nota = None
        sta_values['var_nota'] = sta_var
        var_nota = app.dim(var_nota, sta_var)
        

    except Exception as e:
        app.error(e, 'ErrorExecute', 328)


except Exception as e:
    app.error(e, 'ErrorExecute', 328)

app.foins()
app.index = 362
try:
    switch_val = (var_nota)
    if False:
        pass
    elif (app.str['']("1")) == (switch_val):
        app.foins()
        def var_holabb(*arg):
            try:
                f_rt = (var_str)
            except:
                f_rt = (var_str)
            app.variables.append([locals(), globals()])
            sta_values = {}
            constant = {}
            app.foins()
            app.index = 442
            try:
                var_print  (app.str['']("Este es la funcion 'Hola BB'"))

            except Exception as e:
                app.error(e, 'ErrorExecute', 442)

            app.foins()
            app.index = 493
            try:
                return app.dim((app.str['']("retornado")), f_rt)

            except Exception as e:
                app.error(e, 'ErrorExecute', 493)

            app.variables.pop()

            pass
        app.foins()
        app.index = 532
        try:
            var_print  (app.str['']("la funcion retorno:")  ,  var_holabb  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 532)


        pass
    elif (app.str['']("2")) == (switch_val):
        app.foins()
        app.index = 606
        try:
            app.index = 606
            try:
                if 'var_FA' in constant: app.constE('var_FA')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_FA = (t_tmp0)
                except:
                    var_FA = None
                sta_values['var_FA'] = sta_var
                var_FA = app.dim(var_FA, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 606)


        except Exception as e:
            app.error(e, 'ErrorExecute', 606)

        app.foins()
        app.index = 750
        try:
            var_FA  (app.str['']("frask"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 750)


        pass
    elif (app.str['']("3")) == (switch_val):
        app.foins()
        app.index = 795
        try:
            if (var_nota  ==  app.str['']("3")):
                app.foins()
                app.index = 826
                try:
                    var_print  (app.str['']("este si es el caso 3"))

                except Exception as e:
                    app.error(e, 'ErrorExecute', 826)


                pass

        except Exception as e:
            app.error(e, 'ErrorExecute', 795)

        app.foins()
        app.index = 876
        try:
            if (var_nota  ==  app.str['']("2")):

                pass
            elif (var_true):
                app.foins()
                app.index = 931
                try:
                    var_print  (app.str['']("y este no es el caso 2"))

                except Exception as e:
                    app.error(e, 'ErrorExecute', 931)


                pass

        except Exception as e:
            app.error(e, 'ErrorExecute', 876)


        pass
    elif (app.str['']("4")) == (switch_val):
        app.foins()
        app.index = 1009
        try:
            while (var_true):
                app.foins()
                app.index = 1036
                try:
                    app.index = 1036
                    try:
                        if 'var_pass' in constant: app.constE('var_pass')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_pass = (var_input  (app.str['']("contraseña: ")))
                        except:
                            var_pass = None
                        sta_values['var_pass'] = sta_var
                        var_pass = app.dim(var_pass, sta_var)
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1036)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 1036)

                app.foins()
                app.index = 1084
                try:
                    if (var_pass  ==  app.str['']("done")):
                        app.foins()
                        app.index = 1122
                        try:
                            break

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 1122)


                        pass

                except Exception as e:
                    app.error(e, 'ErrorExecute', 1084)


                pass

        except Exception as e:
            app.error(e, 'ErrorExecute', 1009)


        pass
    elif (app.str['']("5")) == (switch_val):
        app.foins()
        app.index = 1186
        try:
            app.index = 1186
            try:
                if 'var_lista' in constant: app.constE('var_lista')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_lista = (app.fist( [app.values['int'](1)  ,  app.values['int'](2)  ,  app.values['int'](3)  ,  app.values['int'](4)  ,  app.values['int'](5)] ))
                except:
                    var_lista = None
                sta_values['var_lista'] = sta_var
                var_lista = app.dim(var_lista, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 1186)


        except Exception as e:
            app.error(e, 'ErrorExecute', 1186)

        app.foins()
        app.index = 1220
        try:
            var_index = (app.values['int'](0))
            while (True):
                if not (var_index  <  var_len  (var_lista)): break
                app.foins()
                app.index = 1280
                try:
                    app.index = 1280
                    try:
                        if 'var_element' in constant: app.constE('var_element')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_element = (var_lista  [var_index])
                        except:
                            var_element = None
                        sta_values['var_element'] = sta_var
                        var_element = app.dim(var_element, sta_var)
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1280)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 1280)

                app.foins()
                app.index = 1321
                try:
                    var_print  (var_element)

                except Exception as e:
                    app.error(e, 'ErrorExecute', 1321)


                var_index  +=1
                pass

        except Exception as e:
            app.error(e, 'ErrorExecute', 1220)


        pass
    elif (app.str['']("6")) == (switch_val):
        app.foins()
        app.index = 1381
        try:
            app.index = 1381
            try:
                if 'var_lista' in constant: app.constE('var_lista')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_lista = (app.fist( [app.values['int'](1)  ,  app.values['int'](2)  ,  app.values['int'](3)  ,  app.values['int'](4)  ,  app.values['int'](5)] ))
                except:
                    var_lista = None
                sta_values['var_lista'] = sta_var
                var_lista = app.dim(var_lista, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 1381)


        except Exception as e:
            app.error(e, 'ErrorExecute', 1381)

        app.foins()
        app.index = 1414
        try:
            var_print  (app.str['']("con for each"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 1414)

        app.foins()
        app.index = 1446
        try:
            for var_x in (var_lista):
                app.foins()
                app.index = 1479
                try:
                    var_print  (var_x)

                except Exception as e:
                    app.error(e, 'ErrorExecute', 1479)


                pass

        except Exception as e:
            app.error(e, 'ErrorExecute', 1446)


        pass
    elif (app.str['']("7")) == (switch_val):
        app.foins()
        app.index = 1533
        try:
            var_print  (app.str['']("sentencia con with"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 1533)

        app.foins()
        app.index = 1571
        try:
            def t_tmp_with(t_t):
                var_x = t_t
                app.foins()
                app.index = 1603
                try:
                    var_print  (app.str['']("¡Hola!")  ,  var_x)

                except Exception as e:
                    app.error(e, 'ErrorExecute', 1603)


                pass
            t_tmp_with(app.str['']("Carlos"))
            del t_tmp_with

        except Exception as e:
            app.error(e, 'ErrorExecute', 1571)


        pass
    elif (app.str['']("8")) == (switch_val):
        app.foins()
        app.index = 1668
        try:
            try:
                app.foins()
                app.index = 1686
                try:
                    var_xd

                except Exception as e:
                    app.error(e, 'ErrorExecute', 1686)


                pass
            except Exception as ero:
                var_e = ero
                app.cracheos = []
                app.foins()
                app.index = 1721
                try:
                    var_print  (app.str['']("Hubo un error: ")  ,  var_e)

                except Exception as e:
                    app.error(e, 'ErrorExecute', 1721)


                pass

        except Exception as e:
            app.error(e, 'ErrorExecute', 1668)


        pass
    elif (app.str['']("9")) == (switch_val):
        app.foins()
        app.index = 1793
        try:
            class tmp_class_Persona(*app.code2class()):
                pass
            def tmp_var_Persona(obj):
                exportar = {}
                me = obj
                class private:pass
                def t_get_atr(self, v): 
                    return None
                me.__getattr__ = t_get_atr
                private.__getattr__ = t_get_atr
                def t_set_atr(self, a, v): 
                    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_Any))
                me.__setattr__ = t_set_atr
                private.__setattr__ = t_set_atr

                var_private = private
                private = var_private
                def out(*arg): return me(*arg)
                def p_call(o):
                    def eo(*arg):
                        o(me, *arg)
                    return eo
                app.foins()
                app.index = 1831
                try:
                    app.index = 1831
                    try:
                        if 'var_x' in constant: app.constE('var_x')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_x = (app.values['int'](0))
                        except:
                            var_x = None
                        sta_values['var_x'] = sta_var
                        private.x = app.dim(var_x, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1831)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 1831)

                app.foins()
                app.index = 1862
                try:
                    app.index = 1862
                    try:
                        if 'var_y' in constant: app.constE('var_y')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_y = (app.values['int'](0))
                        except:
                            var_y = None
                        sta_values['var_y'] = sta_var
                        private.y = app.dim(var_y, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1862)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 1862)

                app.foins()
                app.index = 1885
                try:
                    app.index = 1885
                    try:
                        if 'var_name' in constant: app.constE('var_name')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_name = (app.str[''](""))
                        except:
                            var_name = None
                        sta_values['var_name'] = sta_var
                        me.name = app.dim(var_name, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1885)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 1885)

                app.foins()
                def var_main(self, *arg):
                    try:
                        f_rt = (var_Any)
                    except:
                        f_rt = (var_Any)
                    var_me = self
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_name = arg[0]
                    except:
                        var_name = (None)
                    app.dim(var_name, sta_var)
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_x = arg[1]
                    except:
                        var_x = (None)
                    app.dim(var_x, sta_var)
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_y = arg[2]
                    except:
                        var_y = (None)
                    app.dim(var_y, sta_var)
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 1956
                    try:
                        app.index = 1956
                        try:
                            var_private.x = (var_x)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 1956)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1956)

                    app.foins()
                    app.index = 1987
                    try:
                        app.index = 1987
                        try:
                            var_private.y = (var_y)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 1987)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 1987)

                    app.foins()
                    app.index = 2018
                    try:
                        app.index = 2018
                        try:
                            var_me.name = (var_name)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 2018)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2018)

                    app.variables.pop()

                    pass
                me.__init__ = var_main
                var_main = p_call(var_main)
                app.foins()
                def var_getname(self, *arg):
                    try:
                        f_rt = (var_String)
                    except:
                        f_rt = (var_String)
                    var_me = self
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 2096
                    try:
                        return app.dim((var_me.name), f_rt)

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2096)

                    app.variables.pop()

                    pass
                me.getname = var_getname
                var_getname = p_call(var_getname)
                app.foins()
                def var_get_position(self, *arg):
                    try:
                        f_rt = (var_Array)
                    except:
                        f_rt = (var_Array)
                    var_me = self
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 2178
                    try:
                        return app.dim((app.fist( [var_private.x  ,  var_private.y] )), f_rt)

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2178)

                    app.variables.pop()

                    pass
                me.get_position = var_get_position
                var_get_position = p_call(var_get_position)

                out.__export__ = exportar
                me.__export__ = exportar
                return out
            var_Persona = tmp_var_Persona(tmp_class_Persona)
            var_Persona.__clase__ = tmp_class_Persona

        except Exception as e:
            app.error(e, 'ErrorExecute', 1793)

        app.foins()
        app.index = 2243
        try:
            app.index = 2243
            try:
                if 'var_Carlos' in constant: app.constE('var_Carlos')
                try:
                    sta_var = var_Persona
                except:
                    try:
                        sta_var = var_Persona
                    except:
                        app.error('the Persona class not found', 'ErrorName', app.index)
                try:
                    var_Carlos = (var_Persona  (app.str['']("Carlos")  ,  app.values['int'](10)  ,  app.values['int'](10)))
                except:
                    var_Carlos = None
                sta_values['var_Carlos'] = sta_var
                var_Carlos = app.dim(var_Carlos, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 2243)


        except Exception as e:
            app.error(e, 'ErrorExecute', 2243)

        app.foins()
        app.index = 2309
        try:
            var_print  (app.str['']("Carlos es una instancia de Persona"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 2309)

        app.foins()
        app.index = 2362
        try:
            var_print  (app.str['']("Nombre:")  ,  var_Carlos.getname  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 2362)

        app.foins()
        app.index = 2406
        try:
            var_print  (app.str['']("Pocision:")  ,  var_Carlos.get_position  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 2406)

        app.foins()
        app.index = 2458
        try:
            class tmp_class_Chambeador(*app.code2class(var_Persona)):
                pass
            def tmp_var_Chambeador(obj):
                exportar = {}
                me = obj
                class private:pass
                def t_get_atr(self, v): 
                    return None
                me.__getattr__ = t_get_atr
                private.__getattr__ = t_get_atr
                def t_set_atr(self, a, v): 
                    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_Any))
                me.__setattr__ = t_set_atr
                private.__setattr__ = t_set_atr

                var_private = private
                private = var_private
                def out(*arg): return me(*arg)
                def p_call(o):
                    def eo(*arg):
                        o(me, *arg)
                    return eo
                app.foins()
                app.index = 2506
                try:
                    app.index = 2506
                    try:
                        if 'var_x' in constant: app.constE('var_x')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_x = (app.values['int'](0))
                        except:
                            var_x = None
                        sta_values['var_x'] = sta_var
                        private.x = app.dim(var_x, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2506)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 2506)

                app.foins()
                app.index = 2537
                try:
                    app.index = 2537
                    try:
                        if 'var_y' in constant: app.constE('var_y')
                        try:
                            sta_var = var_Any
                        except:
                            try:
                                sta_var = var_Any
                            except:
                                app.error('the Any class not found', 'ErrorName', app.index)
                        try:
                            var_y = (app.values['int'](0))
                        except:
                            var_y = None
                        sta_values['var_y'] = sta_var
                        private.y = app.dim(var_y, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2537)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 2537)

                app.foins()
                app.index = 2560
                try:
                    app.index = 2560
                    try:
                        if 'var_chamba' in constant: app.constE('var_chamba')
                        try:
                            sta_var = var_String
                        except:
                            try:
                                sta_var = var_String
                            except:
                                app.error('the String class not found', 'ErrorName', app.index)
                        try:
                            var_chamba = (app.str[''](""))
                        except:
                            var_chamba = None
                        sta_values['var_chamba'] = sta_var
                        me.chamba = app.dim(var_chamba, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2560)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 2560)

                app.foins()
                def var_main(self, *arg):
                    try:
                        f_rt = (var_Any)
                    except:
                        f_rt = (var_Any)
                    var_me = self
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_name = arg[0]
                    except:
                        var_name = (None)
                    app.dim(var_name, sta_var)
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_x = arg[1]
                    except:
                        var_x = (None)
                    app.dim(var_x, sta_var)
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_y = arg[2]
                    except:
                        var_y = (None)
                    app.dim(var_y, sta_var)
                    try:
                        sta_var = var_Any
                    except:
                        try:        sta_var = var_Any
                        except:
                            app.error('the Any class not found', errores.ErrorName)
                    try:
                        var_chamba = arg[3]
                    except:
                        var_chamba = (None)
                    app.dim(var_chamba, sta_var)
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 2650
                    try:
                        app.index = 2650
                        try:
                            var_private.x = (var_x)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 2650)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2650)

                    app.foins()
                    app.index = 2681
                    try:
                        app.index = 2681
                        try:
                            var_private.y = (var_y)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 2681)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2681)

                    app.foins()
                    app.index = 2712
                    try:
                        app.index = 2712
                        try:
                            var_me.name = (var_name)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 2712)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2712)

                    app.foins()
                    app.index = 2744
                    try:
                        app.index = 2744
                        try:
                            var_me.chamba = (var_chamba)

                        except Exception as e:
                            app.error(e, 'ErrorExecute', 2744)


                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2744)

                    app.variables.pop()

                    pass
                me.__init__ = var_main
                var_main = p_call(var_main)
                app.foins()
                def var_get_chamba(self, *arg):
                    try:
                        f_rt = (var_String)
                    except:
                        f_rt = (var_String)
                    var_me = self
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 2829
                    try:
                        return app.dim((var_me.chamba), f_rt)

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2829)

                    app.variables.pop()

                    pass
                me.get_chamba = var_get_chamba
                var_get_chamba = p_call(var_get_chamba)
                app.foins()
                def var_play( *arg):
                    try:
                        f_rt = (var_Any)
                    except:
                        f_rt = (var_Any)
                    
                    app.variables.append([locals(), globals()])
                    sta_values = {}
                    constant = {}
                    app.foins()
                    app.index = 2915
                    try:
                        var_print  (app.str['']("ha chambear"))

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 2915)

                    app.variables.pop()

                    pass
                out.play = var_play
                

                out.__export__ = exportar
                me.__export__ = exportar
                return out
            var_Chambeador = tmp_var_Chambeador(tmp_class_Chambeador)
            var_Chambeador.__clase__ = tmp_class_Chambeador

        except Exception as e:
            app.error(e, 'ErrorExecute', 2458)

        app.foins()
        app.index = 2972
        try:
            app.index = 2972
            try:
                if 'var_Pablo' in constant: app.constE('var_Pablo')
                try:
                    sta_var = var_Chambeador
                except:
                    try:
                        sta_var = var_Chambeador
                    except:
                        app.error('the Chambeador class not found', 'ErrorName', app.index)
                try:
                    var_Pablo = (var_Chambeador  (app.str['']("Pablo")  ,  app.values['int'](10)  ,  app.values['int'](10)  ,  app.str['']("prostitucion")))
                except:
                    var_Pablo = None
                sta_values['var_Pablo'] = sta_var
                var_Pablo = app.dim(var_Pablo, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 2972)


        except Exception as e:
            app.error(e, 'ErrorExecute', 2972)

        app.foins()
        app.index = 3058
        try:
            var_print  (app.str['']("Pablo es una instancia de Chambeador"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 3058)

        app.foins()
        app.index = 3113
        try:
            var_print  (app.str['']("Nombre:")  ,  var_Pablo.getname  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 3113)

        app.foins()
        app.index = 3156
        try:
            var_print  (app.str['']("Pocision:")  ,  var_Pablo.get_position  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 3156)

        app.foins()
        app.index = 3206
        try:
            var_print  (app.str['']("Chamba:")  ,  var_Pablo.get_chamba  ())

        except Exception as e:
            app.error(e, 'ErrorExecute', 3206)

        app.foins()
        app.index = 3253
        try:
            var_Chambeador.play  ()

        except Exception as e:
            app.error(e, 'ErrorExecute', 3253)


        pass
    elif (app.str['']("10")) == (switch_val):
        app.foins()
        app.index = 3306
        try:
            var_useModules.hi  ()

        except Exception as e:
            app.error(e, 'ErrorExecute', 3306)

        app.foins()
        app.index = 3332
        try:
            var_useModules.subModule.hi  ()

        except Exception as e:
            app.error(e, 'ErrorExecute', 3332)


        pass
    elif (app.str['']("11")) == (switch_val):
        app.foins()
        app.index = 3393
        try:
            var_print  (app.str['']("nombre de espacio:")  ,  var_ns.__names__.value)

        except Exception as e:
            app.error(e, 'ErrorExecute', 3393)


        pass
    elif (app.str['']("12")) == (switch_val):
        app.foins()
        app.index = 3467
        try:
            def var_Vec2():
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
                me.__str__ = lambda x: '<Struct Vec2>'
                me._str = me.__str__
                me.__repl__ = me.__str__

                var_private = private()
                private = var_private
                app.variables.append([locals(), globals()])
                sta_values = {}
                constant = {}
                app.index = 3493
                try:
                    app.index = 3493
                    try:
                        if 'var_X' in constant: app.constE('var_X')
                        try:
                            sta_var = var_int
                        except:
                            try:
                                sta_var = var_int
                            except:
                                app.error('the int class not found', 'ErrorName', app.index)
                        try:
                            var_X = (app.values['int'](0))
                        except:
                            var_X = None
                        sta_values['var_X'] = sta_var
                        me.X = app.dim(var_X, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 3493)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 3493)

                app.index = 3516
                try:
                    app.index = 3516
                    try:
                        if 'var_Y' in constant: app.constE('var_Y')
                        try:
                            sta_var = var_int
                        except:
                            try:
                                sta_var = var_int
                            except:
                                app.error('the int class not found', 'ErrorName', app.index)
                        try:
                            var_Y = (app.values['int'](0))
                        except:
                            var_Y = None
                        sta_values['var_Y'] = sta_var
                        me.Y = app.dim(var_Y, sta_var)
                        
                        

                    except Exception as e:
                        app.error(e, 'ErrorExecute', 3516)


                except Exception as e:
                    app.error(e, 'ErrorExecute', 3516)

                app.variables.pop()

                return me
            var_Vec2 = (var_Vec2)()

        except Exception as e:
            app.error(e, 'ErrorExecute', 3467)

        app.foins()
        app.index = 3548
        try:
            app.index = 3548
            try:
                if 'var_Vector' in constant: app.constE('var_Vector')
                try:
                    sta_var = var_Vec2
                except:
                    try:
                        sta_var = var_Vec2
                    except:
                        app.error('the Vec2 class not found', 'ErrorName', app.index)
                try:
                    var_Vector = (None)
                except:
                    var_Vector = None
                sta_values['var_Vector'] = sta_var
                var_Vector = app.dim(var_Vector, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 3548)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3548)

        app.foins()
        app.index = 3570
        try:
            app.index = 3570
            try:
                var_Vector.X = (app.values['int'](10))

            except Exception as e:
                app.error(e, 'ErrorExecute', 3570)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3570)

        app.foins()
        app.index = 3593
        try:
            app.index = 3593
            try:
                var_Vector.Y = (app.values['int'](20)  +  app.values['int'](10))

            except Exception as e:
                app.error(e, 'ErrorExecute', 3593)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3593)

        app.foins()
        app.index = 3622
        try:
            var_print  (app.str['']("vector:")  ,  var_Vector.X  ,  var_Vector.Y)

        except Exception as e:
            app.error(e, 'ErrorExecute', 3622)


        pass
    elif (app.str['']("13")) == (switch_val):
        app.foins()
        app.index = 3694
        try:
            app.index = 3694
            try:
                if 'var_arreglo' in constant: app.constE('var_arreglo')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_arreglo = (app.fist( [app.values['int'](1)  ,  app.values['int'](2)  ,  app.values['int'](3)  ,  app.fist( [app.values['int'](1)] )] ))
                except:
                    var_arreglo = None
                sta_values['var_arreglo'] = sta_var
                var_arreglo = app.dim(var_arreglo, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 3694)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3694)

        app.foins()
        app.index = 3731
        try:
            app.index = 3731
            try:
                var_arreglo  [app.values['int'](3)]  [app.values['int'](0)] = (app.values['int'](4))

            except Exception as e:
                app.error(e, 'ErrorExecute', 3731)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3731)

        app.foins()
        app.index = 3759
        try:
            var_print  (var_arreglo)

        except Exception as e:
            app.error(e, 'ErrorExecute', 3759)


        pass
    elif (app.str['']("14")) == (switch_val):
        app.foins()
        app.index = 3810
        try:
            app.index = 3810
            try:
                if 'var_objeto' in constant: app.constE('var_objeto')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_objeto = (app.fist( {app.str['']('nombre')  :  app.str['']("Carlos")  ,  app.str['']('edad')  :  app.values['int'](19)  ,  app.str['']('virgen')  :  var_true} ))
                except:
                    var_objeto = None
                sta_values['var_objeto'] = sta_var
                var_objeto = app.dim(var_objeto, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 3810)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3810)

        app.foins()
        app.index = 3922
        try:
            var_print  (var_objeto)

        except Exception as e:
            app.error(e, 'ErrorExecute', 3922)


        pass
    elif (app.str['']("15")) == (switch_val):
        app.foins()
        app.index = 3974
        try:
            app.index = 3974
            try:
                if 'var_ask' in constant: app.constE('var_ask')
                try:
                    sta_var = var_Any
                except:
                    try:
                        sta_var = var_Any
                    except:
                        app.error('the Any class not found', 'ErrorName', app.index)
                try:
                    var_ask = (var_input  (app.str['']("es verdadero? (s, n): ")))
                except:
                    var_ask = None
                sta_values['var_ask'] = sta_var
                var_ask = app.dim(var_ask, sta_var)
                

            except Exception as e:
                app.error(e, 'ErrorExecute', 3974)


        except Exception as e:
            app.error(e, 'ErrorExecute', 3974)

        app.foins()
        app.index = 4025
        try:
            var_print  (( (app.str['']("es verdadero"))  if  (var_ask  ==  app.str['']("s"))  else  (app.str['']("no es verdadero")) ))

        except Exception as e:
            app.error(e, 'ErrorExecute', 4025)


        pass
    else:
        app.foins()
        app.index = 4130
        try:
            var_print  (app.str['']("Caso no disponible"))

        except Exception as e:
            app.error(e, 'ErrorExecute', 4130)


        pass

except Exception as e:
    app.error(e, 'ErrorExecute', 362)

app.variables.pop()
