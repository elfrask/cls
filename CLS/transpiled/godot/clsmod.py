from clsengine.clsengine import *
import sys

class ModCLs(appcls):
    
    def exec(self, listo:str):

        pre_name = sys.argv[3].split(".")
        pre_name.pop()
        rename = ".".join(pre_name) + ".gd"

        with open(rename, "w") as file:
            file.write(listo)
            pass
        
        pass
    def generator(self, c, modo:str="normal", usingmode = "compiled", data = {}) -> list:
        
        salida = []
        code = []
        func = []
        lastcode = []
        #print(usingmode)
        using_namespace = False
        #modo = "normal"

        def p_error(code, i) -> list:
            
            return code
        def print_arg(args) -> list:
            s_out = ""
            it = -1
            for i in args:
                it+=1
                #print(i)
                arg=i["name"]
                sta=i["type"][0]
                col = self.generator_one(i["def"], "func")
                if col == "": col = "None"
                defa=(col)

                s_out += f"var_{arg}"
                if not (sta in ["Any", "any"]): s_out += f": var_{sta}"
                if (i["def"] != []): s_out += f"= ({defa})"
                s_out+=", "

                pass
            
            return s_out[:-2]

        if isinstance(c, dict):
            
            
            code = c["data"]
            func = c["func"]
            pass
        elif isinstance(c, list):
            code = c
            func = []
            pass
        else: return []
        
        for i in func:
            break
            if modo in ["normal", "func", "func-imp", "module", "class"]:

                nombre = i[0]
                fun = i[1]

                asy=""
                if fun["async"]: asy = "async "
                preparo = [
                    f"{asy}def {nombre}(*arg):",
                    "    try:"
                    f"        f_rt = (var_{fun['return']})",
                    "    except:"
                    f"        f_rt = (var_{fun['return']})",
                    print_arg(fun["arg"]) +
                    self.generator(fun["code"], "func"),
                    "    pass"
                ]
                
                salida+=preparo
            pass
        
        for i in code:
            if modo in ["normal", "func", "func-imp", "module", "class"] and False:
                salida += ["app.foins()"]
            
            #Declaracion de Funciones
            if   (i["tipo"] == "func-def")         and (modo in ["normal"]):
                asy=""
                fun = i
                if fun["async"]: asy = "async "
                preparo = [
                    f"func var_{fun['name']}({print_arg(fun['arg'])}):",                    
                    self.generator(fun["code"], "func"),
                    "    pass"
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "$func-def")         and (modo in ["module"]):

                asy=""
                qqq=""
                fun = i
                
                
                if fun["async"]: asy = "async "
                visible = fun["visible"]
                #nombre = self.namespace+"_"+fun['name']
                nombre = fun["name"]

                selfi = "self,"
                sefi2 = f"var_me = self"
                qqq = f"me.{nombre} = var_{nombre}"
                #qqq2 = f"var_{nombre} = p_call(var_{nombre})"
                
                if visible == "private":
                    qqq=f"private.{nombre} = var_{nombre}"
                    #qqq2=""
                    pass
                
                
                
                preparo = [
                    f"{asy}def var_{nombre}(*arg):",
                    f"    try:",
                    f"        f_rt = (var_{fun['return']})",
                    f"    except:",
                    f"        f_rt = (var_{fun['return']})",
                    print_arg(fun["arg"]) +
                    self.generator(fun["code"], "func"),
                    "    pass",
                    qqq,
                    #qqq2
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "$func-def")         and (modo in ["class"]):

                asy=""
                qqq=""
                fun = i
                
                
                if fun["async"]: asy = "async "
                visible = fun["visible"]
                #nombre = self.namespace+"_"+fun['name']
                nombre = fun["name"]

                selfi = "self,"
                sefi2 = f"var_me = self"
                qqq = f"me.{tokens['metodos'].get(nombre, nombre)} = var_{nombre}"
                qqq2 = f"var_{nombre} = p_call(var_{nombre})"
                privado=""
                if visible=="static":
                    selfi = ""
                    qqq = f"out.{nombre} = var_{nombre}"
                    sefi2=""
                    qqq2=""
                    pass
                elif visible == "private":
                    privado="private_"
                    qqq=f"var_{nombre} = p_call({privado}var_{nombre})"
                    qqq2=f"private.{nombre} = var_{nombre}"
                    pass
                elif visible == "export":
                    privado="private_"
                    #qqq=f"private.{nombre} = p_call({privado}var_{nombre})"
                    pass
                
                
                preparo = [
                    f"{asy}def {privado}var_{nombre}({selfi} *arg):",
                    f"    try:",
                    f"        f_rt = (var_{fun['return']})",
                    f"    except:",
                    f"        f_rt = (var_{fun['return']})",
                    f"    {sefi2}",
                    print_arg(fun["arg"]) +
                    self.generator(fun["code"], "func"),
                    "    pass",
                    qqq,
                    qqq2
                ]
                salida+=preparo
                pass
            
            #Ejecucion, y operaciones basicas
            elif (i["tipo"] == "exe")              and (modo in ["func-imp", "func"]):

                le = self.generator_one(i["exe"], modo, "exec")
                salida += p_error(
                    [le],
                    i["i"]
                )
                pass
            elif (i["tipo"] == "if-def")           and (modo in ["func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                for x in i["lista"]: 
                    if x["tipo"] in ["if", "elif"]:
                        cond = self.generator_one(x["cond"], modo)
                        codigo = self.generator(x["code"], modo)
                        if_out += [
                            f"{x['tipo']} ({cond}):",
                            codigo,
                            "    pass"
                        ]
                    else:
                        codigo = self.generator(x["code"], modo)
                        if_out += [
                            f"else:",
                            codigo,
                            "    pass"
                        ]
                        break
                    pass

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "switch-def")       and (modo in ["func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                cond = self.generator_one(i["cond"], modo)
                codigo = self.generator(i["code"], "switch", data = {"modo": modo})
                if_out += [
                    f"match ({cond}):",
                        codigo,
                     "    pass",
                ]

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "case-def")         and (modo in ["switch"]):
                if_out=[]
                #tipo, cond, code

                cond = self.generator_one(i["cond"], data.get("modo", "normal"))
                codigo = self.generator(i["code"], data.get("modo", "normal"))
                if_out += [
                    f"({cond}):",
                     codigo,
                     "    pass",
                ]

                salida += if_out

                pass
            elif (i["tipo"] == "case-d-def")       and (modo in ["switch"]):
                if_out=[]

                #tipo, cond, code
                codigo = self.generator(i["code"], data.get("modo", "normal"))
                if_out += [
                    f"_:",
                     codigo,
                     "    pass",
                ]

                lastcode = if_out

                pass
            elif (i["tipo"] == "while-def")        and (modo in ["func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                cond = self.generator_one(x["cond"], modo)
                codigo = self.generator(x["code"], modo)
                if_out += [
                    f"while ({cond}):",
                    codigo,
                    "    pass"
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "for-def")          and (modo in ["func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                d_for =x["for"] #self.generator_one(x["for"], modo)
                cond = self.generator_one(d_for[1], modo)
                codigo = self.generator(x["code"], modo)
                post_code = self.generator_one(d_for[2], modo, "exec")
                if_out += [
                    f"var_{d_for[0][0]['name']} = ({self.generator_one(d_for[0][2:], modo)})",
                    f"while (true):",
                    f"    if !({cond}): break",
                    codigo,
                    f"    {post_code}",
                     "    pass"
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "for-each-def")     and (modo in ["func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                cond = self.generator_one(x["cond"], modo)
                codigo = self.generator(x["code"], modo)
                if_out += [
                    f"for var_{x['var']} in ({cond}):",
                        codigo,
                    "    pass"
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "$with-def")         and (modo in ["normal", "func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                cond = self.generator_one(x["value"], modo)
                codigo = self.generator(x["code"], modo)
                if_out += [
                    f"def t_tmp_with(t_t):",
                    f"    var_{x['name']} = t_t",
                        codigo,
                    "    pass",
                    f"t_tmp_with({cond})",
                    f"del t_tmp_with"
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "rt-def")           and (modo in ["func"]):
                
                
                out = self.generator_one(i["eval"], modo)
                preparo = [
                    #f"return app.dim(({out}), f_rt)",
                    f"return {out}",
                ]

                salida+=p_error(preparo, i["i"])
                pass
            elif (i["tipo"] == "$try-def")          and (modo in ["normal", "func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                #cond = self.generator_one(x["cond"], modo)
                code_try = self.generator(x["try"], modo)
                code_error = self.generator(x["error"], modo)
                if_out += [
                     "try:",
                          code_try,
                     "    pass",
                     "except Exception as ero:",
                    f"    var_{x['e']} = ero",
                     "    app.cracheos = []",
                          code_error,
                     "    pass",
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            
            #Declaracion de Clases, Estructuras, Espacios de nombres y Modulos

                # Clases

            elif (i["tipo"] == "$class-def")        and (modo in ["normal", "func-imp", "func"]):
                fun = i
                arg= []
                for x in fun["extend"]:
                    #print(x)
                    arg.append("var_"+x["name"])
                    pass
                
                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                     "me.__setattr__ = t_set_atr",
                     "private.__setattr__ = t_set_atr",
                    #"exportar.__setattr__ = t_set_atr",
                    
                ]
                
                preparo = [
                    f"class tmp_class_{fun['name']}({','.join(arg)}):",
                     "    pass",#"    def __dict__():pass",
                    f"def tmp_var_{fun['name']}(obj):",
                     "    exportar = {}",
                    f"    me = obj",
                     "    class private:pass",
                          preparo2,
                    f"    var_private = private",
                    f"    private = var_private",
                     "    def out(*arg): return me(*arg)",
                     "    def p_call(o):",
                     "        def eo(*arg):",
                     "            o(me, *arg)",
                     "        return eo",


                            self.generator(fun["code"], "class"),


                    f"    out.__export__ = exportar",
                    f"    me.__export__ = exportar",
                     "    return out",
                    f"var_{fun['name']} = tmp_var_{fun['name']}(tmp_class_{fun['name']})",
                    f"var_{fun['name']}.__clase__ = tmp_class_{fun['name']}",
                        #self.generator(fun["code"], "class"),
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "$class-def")        and (modo in ["module"]):
                fun = i
                arg= []
                for x in fun["extend"]:
                    #print(x)
                    arg.append("var_"+x["name"])
                    pass
                
                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                     "me.__setattr__ = t_set_atr",
                     "private.__setattr__ = t_set_atr",
                    #"exportar.__setattr__ = t_set_atr",
                    
                ]
                ta = "me"

                if fun["visisble"] == "private":
                    ta = "private"
                
                preparo = [
                    f"class tmp_class_{fun['name']}({','.join(arg)}):",
                     "    pass",#"    def __dict__():pass",
                    f"def tmp_var_{fun['name']}(obj):",
                     "    exportar = {}",
                    f"    me = obj",
                     "    class private:pass",
                        preparo2,
                    f"    var_private = private",
                    f"    private = var_private",
                     "    def out(*arg): return me(*arg)",
                     "    def p_call(o):",
                     "        def eo(*arg):",
                     "            o(me, *arg)",
                     "        return eo",


                            self.generator(fun["code"], "class"),


                    f"    out.__export__ = exportar",
                    f"    me.__export__ = exportar",
                     "    return out",
                    f"{ta}.{fun['name']} = tmp_var_{fun['name']}(tmp_class_{fun['name']})",
                    f"{ta}.{fun['name']}.__clase__ = tmp_class_{fun['name']}",
                        #self.generator(fun["code"], "class"),
                ]
                salida+=preparo
                pass
            
                # Modulos
            
            elif (i["tipo"] == "$module-def")       and (modo in ["normal", "func-imp", "func"]):
                fun = i

                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    #"    print('atr:', a)",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                    "me.__setattr__ = t_set_atr",
                    "private.__setattr__ = t_set_atr",
                    #"exportar.__setattr__ = t_set_atr",
                    
                ]
                
                preparo = [
                    f"def var_{fun['name']}():",
                     "    class private:pass",#"    private = (MD())",
                     "    class me:pass",
                          preparo2,
                    f"    var_private = private()",
                    f"    private = var_private",
                        self.generator(fun["code"], "module"),
                    #f"    var_private = private()",
                     "    return me()",
                    f"var_{fun['name']} = (var_{fun['name']})()"
                ]
                salida+=preparo
                pass 
            elif (i["tipo"] == "$module-def")       and (modo in ["module"]):
                fun = i

                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    #"    print('atr:', a)",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                    "me.__setattr__ = t_set_atr",
                    "private.__setattr__ = t_set_atr",
                    #"exportar.__setattr__ = t_set_atr",
                    
                ]

                ta = "me"

                if fun["visisble"] == "private":
                    ta = "private"
                
                preparo = [
                    f"def var_{fun['name']}():",
                     "    class private:pass",#"    private = (MD())",
                     "    class me:pass",
                          preparo2,
                    f"    var_private = private()",
                    f"    private = var_private",
                        self.generator(fun["code"], "module"),
                    #f"    var_private = private()",
                     "    return me()",
                    f"{ta}.{fun['name']} = (var_{fun['name']})()"
                ]
                salida+=preparo
                pass
            
                # Espacios de nomres

            elif (i["tipo"] == "$namespace-def")    and (modo in ["normal", "func-imp", "func"]):
                fun = i

                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    #"    print('atr:', a)",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                    "me.__setattr__ = t_set_atr",
                    "private.__setattr__ = t_set_atr",
                    #"exportar.__setattr__ = t_set_atr",
                ]
                
                preparo = [
                    f"def var_{fun['name']}():",
                     "    class private:pass",#"    private = (MD())",
                     "    class me:pass",
                          preparo2,
                    f"    var_private = private()",
                    f"    private = var_private",
                        self.generator(fun["code"], "module"),
                    #f"    var_private = private()",
                     "    return NS(me())",
                    f"var_{fun['name']} = (var_{fun['name']})()"
                ]
                salida+=preparo
                pass
            
                # Estructuras
            
            elif (i["tipo"] == "$struct-def")       and (modo in ["normal", "func-imp", "func"]):
                fun = i

                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    #"    print('atr:', a)",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                     "me.__setattr__ = t_set_atr",
                     "private.__setattr__ = t_set_atr",
                     "me.default = lambda x: me()",
                    f"me.__str__ = lambda x: '<Struct {fun['name']}>'",
                     "me._str = me.__str__",
                     "me.__repl__ = me.__str__",

                    #"exportar.__setattr__ = t_set_atr",
                    
                ]
                
                preparo = [
                    f"def var_{fun['name']}():",
                     "    class private:pass",#"    private = (MD())",
                     "    class me:pass",
                          preparo2,
                    f"    var_private = private()",
                    f"    private = var_private",
                          self.generator(fun["code"], "struct"),
                    #f"    var_private = private()",
                     "    return me",
                    f"var_{fun['name']} = (var_{fun['name']})()"
                ]
                salida+=preparo
                pass 
            elif (i["tipo"] == "$struct-def")       and (modo in ["module"]):
                fun = i

                preparo2 = [
                     "def t_get_atr(self, v): ",
                     "    return None",
                    
                     "me.__getattr__ = t_get_atr",
                     "private.__getattr__ = t_get_atr",
                    #"exportar.__getattr__ = t_get_atr",
                    
                     "def t_set_atr(self, a, v): ",
                    #"    print('atr:', a)",
                    f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{self.tydef}))",
                    
                     "me.__setattr__ = t_set_atr",
                     "private.__setattr__ = t_set_atr",
                     "me.default = lambda x: me()",
                    f"me.default = lambda x: '<Struct {fun['name']}>'",
                     "me._str = me.__str__",
                     "me.__repl__ = me.__str__",
                     
                    #"exportar.__setattr__ = t_set_atr",
                    
                ]

                ta = "me"

                if fun["visisble"] == "private":
                    ta = "private"
                
                preparo = [
                    f"def var_{fun['name']}():",
                     "    class private:pass",#"    private = (MD())",
                     "    class me:pass",
                          preparo2,
                    f"    var_private = private()",
                    f"    private = var_private",
                        self.generator(fun["code"], "struct"),
                    #f"    var_private = private()",
                     "    return me",
                    f"{ta}.{fun['name']} = (var_{fun['name']})()"
                ]
                salida+=preparo
                pass
        

            
            #Declaracion de Variables
            elif (i["tipo"] == "var-def")          and (modo in ["normal", "func-imp", "func"]):
                ordenar = []                
                for x in i["dim"]:
                    out_s = ""

                    if (modo == "normal"): out_s = "onready "

                    arg=x["name"]
                    sta=x["type"][0]
                    col = self.generator_one(x["def"], "func")
                    if col == "": col = "None"
                    defa=(col)

                    #print(f"'{sta}'")

                    out_s += f"var var_{arg}"
                    if not (sta in ["Any", "any"]): out_s += f": var_{sta}"
                    if (x["def"] != []): out_s += f"= ({defa})"
                    

                    ordenar.append(out_s)

                    pass

                salida += ordenar
                pass
            elif (i["tipo"] == "$var-def")          and (modo in ["class", "module", "struct"]):
                ordenar = []                
                for x in i["dim"]:
                    
                    #print(i)
                    arg=x["name"]
                    sta=x["type"][0]
                    col = self.generator_one(x["def"], "func")
                    if col == "": col = "None"
                    defa=(col)

                    na = "me"
                    qqq = ""
                    qqq2 = ""
                    if i["const"]:qqq2 = f"constant.append('var_{arg}')"


                    if i["visible"] == "private":
                        na = "private"
                        pass
                    elif i["visible"] == "static" and modo == "class":
                        na = "out"
                        pass
                    elif i["visible"] == "export" and modo == "class":
                        qqq = f"exportar['{arg}'] = app.dim(var_{arg}, sta_var)"
                        pass
                    
                    

                    ordenar+=p_error([
                        f"if 'var_{arg}' in constant: app.constE('var_{arg}')",
                        f"try:",
                        f"    sta_var = var_{sta}",
                        f"except:",
                        f"    try:",
                        f"        sta_var = var_{sta}",
                        f"    except:",
                        f"        app.error('the {sta} class not found', '{errores.ErrorName}', app.index)",
                        f"try:",
                        f"    var_{arg} = ({defa})",
                        f"except:",
                        f"    var_{arg} = None",
                        f"sta_values['var_{arg}'] = sta_var",
                        f"{na}.{arg} = app.dim(var_{arg}, sta_var)",
                        qqq,
                        qqq2
                    ], i["i"])
                    pass

                salida += ordenar
                pass
            elif (i["tipo"] == "var-eval")         and (modo in ["func-imp", "func", "class", "module", "struct"]):
                
                
                g_1 = self.generator_one(i["var"], modo)
                g_2 = self.generator_one(i["eval"], modo)
                preparo = p_error([
                    f"{g_1} = ({g_2})"
                ], i["i"])

                salida+=preparo
                pass
            
            #Elementos de importacion de modulos externo e internos
            elif (i["tipo"] == "import-def")       and (modo in ["normal"]):
                
                salida += p_error([
                    f"onready var_{i['as']} = cls_engine.getlib('{i['import']}')"
                ], i["i"])
                pass
            elif (i["tipo"] == "$from-def")         and (modo in ["normal", "func-imp", "func"]):
                
                salida += p_error([
                    f"var_{i['as']} = app.getlib('{i['from']}').{i['import']}"
                ], i["i"])
                pass
            elif (i["tipo"] == "$include-def")      and (modo in ["normal", "func-imp", "func"]):
                
                salida += p_error([
                    f"incluir = app.getlib('{i['include']}')",
                     #"print(type(incluir))",
                     "for x in dir(incluir):",
                     #"    print(getattr(incluir, x))",
                    f"    globals()['var_' + x] = getattr(incluir, x)",
                    f"    locals()['var_' + x] = getattr(incluir, x)",

                ], i["i"])
                pass
            elif (i["tipo"] == "template-def")     and (modo in ["normal", "func-imp", "func"]):
                
                pati = lib.find(i["template"])

                tem = []
                if pati == "None":
                    print(f"template '{i['template']}' not found")
                    pass
                else:
                    _ftp = open(pati, "r")
                    tem = _ftp.read().split("\n")
                    _ftp.close()
                    pass

                salida += p_error(tem, i["i"])
                pass
            elif (i["tipo"] == "template-if-def")     and (modo in ["normal", "func-imp", "func"]):
                #print(i, self.PRO.get(i["var"], None))
                
                if "Godot" == i["value"]:
                    pati = lib.find(i["template"])

                    tem = []
                    if pati == "None":
                        print(f"template '{i['template']}' not found")
                        pass
                    else:
                        _ftp = open(pati, "r")
                        tem = _ftp.read().split("\n")
                        _ftp.close()
                        pass

                    salida += p_error(tem, i["i"])
                    pass

                pass
            


            else:
                self.error([
                    f"the sentence of type {i['tipo']} is unsopport for it compiler"
                ], errores.ErrorUnsopportSyntax, i["i"])
                pass
            pass
        if (using_namespace) and (usingmode == "compiled"):
            #print("que?")
            self.namespace = "std"
        if modo == "switch":
            salida += lastcode
        if isinstance(c, dict):
            #salida +=["app.variables.pop()"]
            code = c["data"]
            func = c["func"]
            pass
        
        #del c, using_namespace, usingmode

        return salida
        salida = ""
        last = {"tipo":"none"}
        iskey= False
        
        ite =-1
        for i in line:
            #print(i)
            ite+=1
            def fallo(msg:str="", x:int=False):
                if x == False:
                    x = i["i"]
                if msg=="": msg = "Error Syntax"
                #print("Fallo", x)
                self.error(msg, errores.ErrorSyntax, x)
            if True:

                if i["tipo"] in ["()", "[]", "code"]:
                    i["fist"] = False
                    if last["tipo"] in ["none", "ope", "sim"]:
                        i["fist"] = True
                        #print("llego:", salida)
                        pass
                    pass


                if i["tipo"] == last["tipo"]:
                    if i["tipo"] == "name":
                        if (i["name"] in nombre_reservados["codi"]) or (last["name"] in nombre_reservados["codi"]):
                            #last = i
                            pass
                        elif i["name"][0]==".":
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i["tipo"] in ["[]", "()"]:
                        if last["tipo"] in ["name", "()", "[]"]:
                            last = i
                            pass
                        elif last["tipo"] == "value":
                            if last["type"] == "str":
                                last=i
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i["tipo"] in ["ope"]:
                        if (last["char"] in [":"]) and (not i["tipo"] in [":"]):
                            pass
                        else:
                            fallo()
                        pass
                    else:
                        fallo()

                    pass
                else:
                    if i["tipo"] in ["name", "value"]:
                        if last["tipo"] in ["[]", "()", "code", "name", "value"]:
                            if i["tipo"] == "name":
                                if i["name"][0]==".":
                                    pass
                                else:
                                    fallo()
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            pass
                        pass
                    elif i["tipo"] in ["code"]:
                        if last["tipo"] in ["name", "[]", "value", "()"]:
                            fallo()
                            pass
                        else:
                            pass
                        pass
                    else:
                        pass
                    pass
                if key:
                    if last["tipo"] in ["sim", "none"]:
                        if last.get("char", ",") == ",":
                            iskey = True
                            #print("error 1", i)

                            pass
                        pass
                last = i
                pass
            
            if modo in ["func", "func-imp", "class", "module", "normal"]:

                if i["tipo"]=="name":
                    if iskey:
                        #print("error 2", i)

                        t = {"tipo":"value", "value":f"'{i['name']}'", "type":"str", "i":i["i"], "byte":""}
                        salida+= f" {self.print_value(t)} "
                        pass
                    elif i["notmod"]:
                        salida+= f" {i['name']} "
                        pass
                    elif i["name"] in nombre_reservados["codi"]:
                        salida+= f" {i['name']} "
                        pass                  
                    elif (i["name"] in nombre_reservados["bucle"]):
                        if modi == "exec":
                            if salida == "":
                                salida+= f" {i['name']} "
                            else:
                                fallo(f"Error Syntax with the token '{i['name']}'")
                            pass
                        else:
                            fallo(f"the token '{i['name']}' is invalid in this case")
                        pass
                    elif i["name"][0]==".":
                        salida+= f"{i['name']} "
                        pass
                    elif i["name"][0]=="0":
                        
                        salida+= f" app.fint('{i['name']}', '{i['name'][1]}') "
                        pass
                    elif compara([{"tipo":"ope", "char":"::"}], line[ite+1:ite+2]):
                        salida+= f" var_{i['name']}.__names__"
                        pass
                    elif compara([{"tipo":"ope", "char":"::"}], line[ite-1:ite]):
                        salida+= f".{i['name']}"
                        pass
                    else:
                        salida+= f" var_{i['name']} "
                        pass
                    pass
                elif i["tipo"] == "sim":
                    salida += f" {i['char']} "
                    pass
                elif i["tipo"] == "ope":
                    if i["char"] in ["+", "-", "*", "/", "**", "%", ":"]:
                        salida += f" {i['char']} "
                    elif i["char"] in tokens["convert"]["condi"]:
                        salida += f" {tokens['convert']['condi'][i['char']]} "
                        pass
                    elif i["char"] in tokens["convert"]["expre-"+str(modi)]:
                        salida += f" {tokens['convert']['expre-'+str(modi)][i['char']]} "
                        pass
                    elif i["char"] in tokens["cond"]:
                        salida += f" {i['char']} "
                    elif (i["char"] in ["="]) and (modi=="exec"):
                        salida += f" = "
                    
                    
                    pass
                elif i["tipo"] == "()":
                    be = f" ({self.generator_one(i['data'], modo)}) "
                    if i["fist"]:
                        be = " app.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "[]":
                    be = f" [{self.generator_one(i['data'], modo)}] "
                    if i["fist"]:
                        be = " app.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "code":
                    be = f" {'{'+self.generator_one(i['one'], modo, 'eval', True)+'}'} "
                    if i["fist"]:
                        #print("code")

                        be = " app.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "value":
                    salida+= f" {self.print_value(i)} "
                    pass
                elif i["tipo"] == "cml":
                    salida+= f" app.cml({repr(i['data'])}) "
                    pass
                elif i["tipo"] == "if-exp":
                    be_if = f" ({self.generator_one(i['then']['data'], modo)}) "
                    be_else = f" ({self.generator_one(i['else']['data'], modo)}) "
                    be_cond = f" ({self.generator_one(i['if']['data'], modo)}) "

                    salida+= f" ({be_if} if {be_cond} else {be_else}) "
                    pass
                
                pass
            iskey = False

            
            pass
        while salida[:1]==" ":
            salida = salida[1:]
            pass
        while salida[-1:]==" ":
            salida = salida[:-1]
            pass
        
            
        return salida
    def generator_one(self, line:list, modo:str="normal", modi:str ="eval", key:bool=False) -> str:
        salida = ""
        last = {"tipo":"none"}
        iskey= False
        
        ite =-1
        for i in line:
            #print(i)
            ite+=1
            def fallo(msg:str="", x:int=False):
                if x == False:
                    x = i["i"]
                if msg=="": msg = "Error Syntax"
                #print("Fallo", x)
                self.error(msg, errores.ErrorSyntax, x)
            if True:

                if i["tipo"] in ["()", "[]", "code"]:
                    i["fist"] = False
                    if last["tipo"] in ["none", "ope", "sim"]:
                        i["fist"] = True
                        #print("llego:", salida)
                        pass
                    pass


                if i["tipo"] == last["tipo"]:
                    if i["tipo"] == "name":
                        if (i["name"] in nombre_reservados["codi"]) or (last["name"] in nombre_reservados["codi"]):
                            #last = i
                            pass
                        elif i["name"][0]==".":
                            pass
                        else:
                            print(i)
                            fallo()
                            pass
                        pass
                    elif i["tipo"] in ["[]", "()"]:
                        if last["tipo"] in ["name", "()", "[]"]:
                            last = i
                            pass
                        elif last["tipo"] == "value":
                            if last["type"] == "str":
                                last=i
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i["tipo"] in ["ope"]:
                        if (last["char"] in [":"]) and (not i["tipo"] in [":"]):
                            pass
                        else:
                            fallo()
                        pass
                    else:
                        fallo()

                    pass
                else:
                    if i["tipo"] in ["name", "value"]:
                        if last["tipo"] in ["[]", "()", "code", "name", "value"]:
                            if i["tipo"] == "name":
                                if i["name"][0]==".":
                                    pass
                                else:
                                    fallo()
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            pass
                        pass
                    elif i["tipo"] in ["code"]:
                        if last["tipo"] in ["name", "[]", "value", "()"]:
                            fallo()
                            pass
                        else:
                            pass
                        pass
                    else:
                        pass
                    pass
                if key:
                    if last["tipo"] in ["sim", "none"]:
                        if last.get("char", ",") == ",":
                            iskey = True
                            #print("error 1", i)

                            pass
                        pass
                last = i
                pass
            
            if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:

                if i["tipo"]=="name":
                    if iskey:
                        #print("error 2", i)

                        t = {"tipo":"value", "value":f"'{i['name']}'", "type":"str", "i":i["i"], "byte":""}
                        salida+= f" {self.print_value(t)} "
                        pass
                    elif i["notmod"]:
                        salida+= f" {i['name']} "
                        pass
                    elif i["name"] in nombre_reservados["codi"]:
                        salida+= f" {i['name']} "
                        pass                  
                    elif (i["name"] in nombre_reservados["bucle"]):
                        if modi == "exec":
                            if salida == "":
                                salida+= f" {i['name']} "
                            else:
                                fallo(f"Error Syntax with the token '{i['name']}'")
                            pass
                        else:
                            fallo(f"the token '{i['name']}' is invalid in this case")
                        pass
                    elif i["name"][0]==".":
                        salida+= f"{i['name']} "
                        pass
                    elif i["name"][0]=="0":
                        
                        salida+= f" app.fint('{i['name']}', '{i['name'][1]}') "
                        pass
                    elif compara([{"tipo":"ope", "char":"::"}], line[ite+1:ite+2]):
                        salida+= f" var_{i['name']}.__names__"
                        pass
                    elif compara([{"tipo":"ope", "char":"::"}], line[ite-1:ite]):
                        salida+= f".{i['name']}"
                        pass
                    else:
                        salida+= f" var_{i['name']} "
                        pass
                    pass
                elif i["tipo"] == "sim":
                    salida += f" {i['char']} "
                    pass
                elif i["tipo"] == "ope":
                    if i["char"] in ["+", "-", "*", "/", "**", "%", ":"]:
                        salida += f" {i['char']} "
                    elif i["char"] in tokens["convert"]["condi"]:
                        salida += f" {tokens['convert']['condi'][i['char']]} "
                        pass
                    elif i["char"] in tokens["convert"]["expre-"+str(modi)]:
                        salida += f" {tokens['convert']['expre-'+str(modi)][i['char']]} "
                        pass
                    elif i["char"] in tokens["cond"]:
                        salida += f" {i['char']} "
                    elif (i["char"] in ["="]) and (modi=="exec"):
                        salida += f" = "
                    
                    
                    pass
                elif i["tipo"] == "()":
                    be = f" ({self.generator_one(i['data'], modo)}) "
                    if i["fist"]:
                        be = " cls_engine.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "[]":
                    be = f" [{self.generator_one(i['data'], modo)}] "
                    if i["fist"]:
                        be = " cls_engine.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "code":
                    be = f" {'{'+self.generator_one(i['one'], modo, 'eval', True)+'}'} "
                    if i["fist"]:
                        #print("code")

                        be = " cls_engine.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "value":
                    salida+= f" {self.print_value(i)} "
                    pass
                elif i["tipo"] == "cml":
                    salida+= f" cls_engine.cml({repr(i['data'])}) "
                    pass
                elif i["tipo"] == "$if-exp":
                    be_if = f" ({self.generator_one(i['then']['data'], modo)}) "
                    be_else = f" ({self.generator_one(i['else']['data'], modo)}) "
                    be_cond = f" ({self.generator_one(i['if']['data'], modo)}) "

                    salida+= f" ({be_if} if {be_cond} else {be_else}) "
                    pass
                
                pass
            iskey = False

            
            pass
        while salida[:1]==" ":
            salida = salida[1:]
            pass
        while salida[-1:]==" ":
            salida = salida[:-1]
            pass
        
            
        return salida
    def print_value(self, v) -> str:
        salida = f"cls_engine.parse_values('{v['type']}', {v['value']})"

        if v["type"] == "str":
            salida = f"cls_engine.strf('{v['byte']}', {v['value']})"

        return salida
    
    
    
    pass