from copy import copy as c
import os
import sys
import types

from clsengine.tools import *
from clsengine.gen_char import *
from clsengine.chars import *
from clsengine.ObjectCls import *


uid:int = 0


class Any():
    def __init__(self) -> None:
        pass
    pass
after_code=""
after_code_old="""

mem_out = c(globals())
ti = {}
for i in mem_out:
    if i[0:${l}] == "${n}":
        ti[i] = mem_out[i]
    pass
ex["mem"] = ti
"""

before_code = "ex['mem'] = locals()"

tokens = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^", ":"],
    "sim":["{", "}", "(", ")", "[", "]", ","],
    "cond":["==", "<", ">", "!=", "<=", ">=", "!"],
    "convert":{
        "condi":{"&":"and", "|":"or", "!":"not", "?":"in", "^":"**"},
        "expre-eval":{"++":"+1", "--":"-1"},
        "expre-exec":{"++":"+=1", "--":"-=1"},
    },
    "metodos":{"main":"__init__", "_call":"__call__", "_getitem":"__getitem__", "_setitem":"__setitem__",
        "_add":"__add__", "_sub":"__sub__", "_div":"__div__", "_delitem":"__delitem__", "_mul":"__mul__",
        "_mod":"__mod__", "_or":"__or__", "_and":"__and__", "_xor":"__xor__", "_or":"__or__", "_len":"__len__",
        "_repr":"__repr__", "_str":"__str__", "_int":"__int__", "_float":"__float__", "_array":"__list__", 
        "_dict":"__dict__"
    },
    "to_c":{"String":"str", "Array":"list", "Int":"int", "Float":"float", "Dict":"dict"}
}

nombre_reservados = {
    "visible":["export", "static", "private", "public", "global"], 
    "nombre":[
            "func", "function", "class", "module", "with", "for", "if", "while", "define",
            "from", "import", "global", "try", "def", "fub", "method", "include", "using", "var",
            "template", "switch", "struct", "case", "return", "setrule"
        ],
    "codi":["or", "in", "and", "is"],
    "bucle":["break", "continue"]
}


class errores:
    ErrorSyntax:str="ErrorSyntax"
    ErrorSemant:str="ErrorSemant"
    ErrorAritmetic:str="ErrorAritmetic"
    ErrorTyping:str="ErrorTyping"
    ErrorName:str = "ErrorName"
    ErrorUnsopportSyntax = "ErrorUnsopportSyntax"
    pass
        


derivados = True
key_true = True

lib_path = []


class gen_value:
    def lista(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "complet":list, "fist":bool]
        ):
        out=[]
        if len(data)>0: out = data[0]
        salida = {
            "tipo":"[]",
            "data":out,
            "i":i,
            "fist":False
            #"complet":data
        }
        if derivados: 
            salida["complet"]=data
        return salida
    def argu(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "complet":list, "fist":bool]
        ):
        out=[]
        if len(data)>0: out = data[0]
        salida = {
            "tipo":"()",
            "data":out,
            "i":i,
            "fist":False
            #"complet":data
        }
        if derivados:
            salida["complet"] = data
        return salida
    def code(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "one":list, "fist":bool]
        ):
        out=[]
        if len(data)>0: out = data[0]
        salida = {
            "tipo":"code",
            "data":data,
            "i":i,
            "fist":False
            #"one":out
        }
        if key_true:
            salida["one"] = out
        return salida
    
class lista(list):
    def __dict__(): pass
    pass
def get_at(obj:any, cadena:str, defa=None):
    salida = None
    try:
        salida =eval("tmp_o." + cadena, {"tmp_o":obj})
        pass
    except:
        salida = defa
    return salida

obj = ObjectCls




class form:
    class char:
        __clase__ = ObjectCls.char
        def __init__(self, o):
            self.o = o
            pass
        def __dict__(self):
            pass
        def default(self, clase):
            return clase("", -1)
        def __getitem__(self, e:int=-1):
            def gen(a:str=""):
                return self.o(a, e)
            return gen
        def __call__(self, a:str="", e:int=-1):
            return self.o(a, e)
    class intbit:
        __clase__ = ObjectCls.intbit
        def __init__(self, o):
            self.o = o
            pass
        def __dict__(self):
            pass
        def default(self, clase): 
            return clase(0, 8)
        def __getitem__(self, e:int=8):
            def gen(a:int=0):
                return self.o(a, e)
            return gen
        def __call__(self, a:int=0, e:int=8):
            return self.o(a, e)
    

class lib:
    def load(fs):
        
        for i in lib_path:
            if os.path.isfile(i+"/"+fs):
                return open(i+"/"+fs,"r").read()
            pass

        return None
    def find(fs):
        
        for i in lib_path:
            if os.path.isfile(i+"/"+fs):
                return i+"/"+fs
            pass

        return "None"
    def exist(fs):
        
        for i in lib_path:
            if os.path.isfile(i+"/"+fs):
                return True
            pass

        return False
    
    pass

class PyImports:
    def require(file):
        dato = lib.load(file)
        export = ObjectCls.Module()

        exec(dato, {
            "export":export, 
            "module":ObjectCls.Module,
            "Module":ObjectCls.Module,
            "Api":obj
        })

        return export
    def load(dato):
        
        export = ObjectCls.Module()

        exec(dato, {
            "export":export, 
            "module":ObjectCls.Module,
            "Module":ObjectCls.Module,
            "Api":obj

        })

        return export  
    pass

pre_exit = lambda x=0: exit(x)

class process:
    argv = sys.argv
    PyImports = PyImports
    exit = pre_exit
    class io:
        stdin = sys.stdin
        stdout = sys.stdout
        pass
    pass


Api_cls = {
    "__name__":"main",
    "__cwd__":os.getcwd(),
    "process":process,
    "str":ObjectCls.String,
    "String":ObjectCls.String,
    "bytes":ObjectCls.Bytes,
    "Bytes":ObjectCls.Bytes,
    "Any":ObjectCls.AnyObject,
    "any":ObjectCls.AnyObject,
    "int":ObjectCls.Integer,
    "float":ObjectCls.Float,
    "void":ObjectCls.void,
    "Array":ObjectCls.Array,
    "array":ObjectCls.Array,
    "module":ObjectCls.Module,
    "Module":ObjectCls.Module,
    "function":que_tipo.__class__,
    "Function":que_tipo.__class__,
    "Boolean":ObjectCls.Boolean,
    "bool":ObjectCls.Boolean,
    "Object":dict,
    "object":dict,
    "obj":dict,
    "char":form.char(ObjectCls.char),
    "intbit":form.intbit(ObjectCls.intbit),
    "ErrorNames":errores,
    "namespace":ObjectCls.Namespace,
    "Namespace":ObjectCls.Namespace,
    "Promise":ObjectCls.Promise,
    "promise":ObjectCls.Promise,
    "print":print,
    "len":len,
    "input":input,
    "PyApi":PyImports,
    "range":lambda *x: ObjectCls.Array(range(*x)),

    "hex":obj.hex,
    "bin":obj.bin,
    "oct":obj.oct,
    "Hex":obj.hex,
    "Bin":obj.bin,
    "Oct":obj.oct,
    "ord":lambda x: (obj.Integer(ord(x))),

    "true":ObjectCls.Boolean("true"),
    "false":ObjectCls.Boolean("false"),            
    "True":ObjectCls.Boolean("True"),
    "False":ObjectCls.Boolean("Frue"),
    "on":ObjectCls.Boolean("on"),
    "off":ObjectCls.Boolean("off"),
}

#procesador = 1/int(sys.argv[2])

class appclsBase():
    def __init__(self, id:int = uid, mode="default", root=True) -> None:
        global uid
        uid += 1
        self.codigo:str = "" 
        self.origin:str = f"<Script:{id}>" 
        self.var:dict = {} 
        self.mode:str = "cls" 
        self.app:dict = {} 
        self.archivo:list = [] 
        self.submode:list = [] 
        self.funca:bool=True 
        self.tydef:str = "Any" 
        self.namespace:str = "std" 
        self.index:int = 0
        self.memory = {} 
        self.values={
            "str":ObjectCls.String,
            "int":ObjectCls.Integer,
            "float":ObjectCls.Float
        } 
        self.variables = [] 
        self.str = {
            "":ObjectCls.String,
            "b":lambda x:(ObjectCls.Bytes(x, "utf8")),
            "f":self.str_format,
            "c":lambda x: Api_cls["char"](x, len(x))
        } 
        self.parent = None
        self.api = c(Api_cls) 
        self.cracheos = [] 
        self.libs = {
            "pypkg":PyImports
        } 
        self.formato_int = {
            "x":obj.hex,
            "b":obj.bin,
            "o":obj.oct,
        }
        self.PRO = {
            "mode":mode
        }
        self.api["catch"] = self.catch
        self.root = root

        def print_debug(*arg):
            lin0 = self.codigo[0:self.index].count(N)
            print(f"{self.origin} - {lin0+1} :", *arg)
            pass

        self.api["print_debug"] = print_debug
        self.api["_file"] = "<File>"
        self.api["MODE_ENGINE"] = mode
        self.GeneratorEngine = EngineOutput()
        
        pass
    def getlib(self, file):
        
        def load_lib(file):
            app = appcls(uid, "default", False)

            r_file = file

            file = lib.find(file)

            if file == "None":
                raise Exception(f'the module "{r_file}" not found')

            aplicacion = open(file, "r").read()
            crude:list =app.desline(aplicacion, file)
            parseado:list = app.parselex(crude)
            f:list = []
            estructurado:list = app.estructuration(parseado, f)
            app.parent = self
            generado:dict = app.generator({
                        "data":estructurado,
                        "func":f
            }, "normal")
            listo:str = app.jump(generado, 0)

            open("./cache/lib.json", "w").write(
                str(
                    {
                        "data":estructurado,
                        "func":f
                    }
                )
            )
            open("./cache/lib.py", "w").write(
                listo
            )
            class library:
                
                pass
            app.api["export"] = library()
            #app.api["Module"] = ObjectCls.Module
            #app.api["module"] = ObjectCls.Module

            app.exec(listo)

            self.libs[r_file] = app.memory["var_export"]

            return app.memory["var_export"]
        salida = self.libs.get(file, False)

        if False == salida:
            salida = load_lib(file)
        return salida
    def str_format(self, cadena):
        
        lis = self.variables[-1]
        
        salida = ""

        formatear = cadena.split("{")

        tue =False
        sal = []
        for i in formatear:
            if tue:
                sal += i.split("}", 1)
                pass
            else:
                sal += [i]
                pass
            tue = not tue
            pass
        #print(sal)
        for i in lis:
            locals()[i] = lis[i] 
            pass

        salida = []
        tae = False
        for i in sal:
            if tae:
                if trim_string(i) == "":
                    continue
                salida.append(
                    str(
                        eval(
                            self.generator_one(
                                self.desline(i, "none", False)[0],
                                "normal",
                                "eval"
                            )
                        )
                    )
                )

                pass
            else:
                salida.append(i)
                pass
            tae = not tae
            pass

        

        return "".join(salida)
    def exception(self, msg:str=""): # in exec
        raise Exception(msg)
    def catch(self, msg:str="", e:str="ErrorCatch"): # in exec
        self.error(msg, e, self.index)
    def set_api(self, api:object) -> None:

        pass
    def desline(self, code:str, name:str="file0", enter =True) -> list:
        salida:list = []
        linea:list = []
        if enter:    
            self.codigo = code
            self.origin = name
        code = code.replace(T, " ")
        #code = code.replace(N, " ")
        code = code.replace(R, " ")

        cadena:str = ""
        iterador:int = -1
        modo:str = "normal"
        term:str = ""
        byte:str =""
        tagi =0

        for c in code:
            iterador+=1
            if modo=="normal":
                if not (c==" "):
                    if c=="#":
                        if cadena!="":
                            val = tipo_valor(cadena)
                            out = assign_valor(val[0], val[1], iterador)
                            cadena=""
                            linea.append(out)
                            pass
                        modo="coment"
                        pass
                    elif c in tokens["ope"]:
                        if cadena!="":
                            val = tipo_valor(cadena)
                            out = assign_valor(val[0], val[1], iterador)
                            cadena=""
                            linea.append(out)
                            pass
                        if len(linea)>0:
                            xx = linea[len(linea)-1]
                            if xx["tipo"]=="ope":
                                if (not (xx["char"] in ["=", ":", "<", ">"])) or (c in ["<", ">", "=", ":"]):
                                    linea.pop()
                                    linea.append(
                                        gen_char.ope(xx["char"]+c, 
                                        iterador-1, 
                                        (xx["char"]+c) in tokens["cond"])
                                    )
                                    if linea[len(linea)-1]["char"] == "//":
                                        modo="coment"
                                    continue
                                pass
                            pass
                        pass
                        linea.append(
                            gen_char.ope(c, iterador, c in tokens["cond"])
                        )
                        pass
                    elif c in tokens["sim"]:
                        if cadena!="":
                            val = tipo_valor(cadena)
                            out = assign_valor(val[0], val[1], iterador)
                            cadena=""
                            linea.append(out)
                            pass
                        linea.append(
                            gen_char.sim(c, iterador)
                        )
                        
                        if compara([{"tipo":"ope", "char":"<"}, {"tipo":"ope", "char":">"}], linea[-2:]):
                            modo = "cml"
                            linea.pop()
                            tagi = linea.pop()["i"]
                            cadena = ""
                            pass
                        pass
                    elif c in ['"', "'"]:
                        byte=cadena
                        term=c
                        modo="str"
                        cadena = term
                        pass
                    elif c==N:
                        if cadena!="":
                            val = tipo_valor(cadena)
                            out = assign_valor(val[0], val[1], iterador)
                            cadena=""
                            linea.append(out)
                            pass
                        pass
                    elif c==";":
                        if cadena!="":
                            val = tipo_valor(cadena)
                            out = assign_valor(val[0], val[1], iterador)
                            cadena=""
                            linea.append(out)
                            pass
                        salida.append(linea)
                        if linea == []: salida.pop()
                        linea = []
                        pass
                    else:
                        cadena=cadena+c
                        pass
                    pass
                else:
                    if cadena!="":
                        val = tipo_valor(cadena)
                        out = assign_valor(val[0], val[1], iterador)
                        cadena=""
                        linea.append(out)
                        pass
                    
                    pass
                pass
            elif modo=="coment":
                if c==N:
                    modo="normal"
                pass
            elif modo == "str":
                if c == term:
                    linea.append(
                        gen_char.val(
                            (cadena+c).replace(N, invertido+"n"),
                            "str",
                            iterador-len(cadena),
                            term,
                            byte
                        )
                    )
                    cadena=""
                    modo="normal"
                    pass
                else:
                    cadena = cadena+c
                pass
            elif modo == "cml":
                
                if (c == ">") & (cadena[-2:] == "</"):
                    linea.append(
                        gen_char.cml(
                            cadena[0:-1],
                            tagi,
                        )
                    )
                    cadena=""
                    modo="normal"
                    pass
                else:
                    cadena = cadena+c
                pass
            
            pass
        if cadena!="":
            val = tipo_valor(cadena)
            out = assign_valor(val[0], val[1], iterador+1)
            cadena=""
            linea.append(out)
            pass
        if linea!=[]:
            salida.append(linea)
        #print(salida)


        return salida
    def parselex(self, code:list) -> list:
        modo = "normal"
        salida:list = []
        lol:list = []
        sub=0
        iterador:int=0
        linea:list = []
        cadena:list = []
        for i in code:

            for x in i:
                
                if modo == "normal":
                    if x["tipo"]=="sim":
                        if x["char"]=="(":
                            modo="()"
                            lol=[]
                            cadena=[]
                            sub=0
                            iterador=x["i"]
                            pass
                        elif x["char"]=="[":
                            modo="[]"
                            lol=[]
                            cadena=[]
                            sub=0
                            iterador=x["i"]
                            pass
                        elif x["char"]=="{":
                            modo="code"
                            lol=[]
                            cadena=[]
                            sub=0
                            iterador=x["i"]
                            pass
                        else:
                            char = x["char"]

                            if char in ["}", ")", "]"]:  
                                self.error(f"error syntax: '{char}'", "ErrorSyntax", x["i"])
                            else:
                                linea.append(x)
                        pass
                    else:
                        linea.append(x)
                    pass
                elif modo == "()":
                    if x["tipo"]=="sim":
                        #x:gen_char.sim
                        if x["char"]==")":
                            sub-=1
                            if sub==-1:
                                if cadena!=[]:
                                    lol.append(cadena)
                                    cadena=[]
                                    pass
                                lel = self.parselex(lol)
                                #print(lel)
                                linea.append(gen_value.argu(lel, iterador))
                                modo="normal"
                                pass
                            else:
                                cadena.append(x)

                            pass
                        elif x["char"]=="(":
                            sub+=1
                            cadena.append(x)

                            pass
                        else:
                            cadena.append(x)
                        pass
                    else:
                        cadena.append(x)
                    pass
                elif modo == "[]":
                    if x["tipo"]=="sim":
                        #x:gen_char.sim
                        if x["char"]=="]":
                            sub-=1
                            if sub==-1:
                                if cadena!=[]:
                                    lol.append(cadena)
                                    cadena=[]
                                    pass
                                lel = self.parselex(lol)
                                #print(lel)
                                linea.append(gen_value.lista(lel, iterador))
                                modo="normal"
                                pass
                            else:
                                cadena.append(x)

                            pass
                        elif x["char"]=="[":
                            sub+=1
                            cadena.append(x)

                            pass
                        else:
                            cadena.append(x)
                        pass
                    else:
                        cadena.append(x)
                    pass
                elif modo == "code":
                    if x["tipo"]=="sim":
                        #x:gen_char.sim
                        if x["char"]=="}":
                            sub-=1
                            if sub==-1:
                                if cadena!=[]:
                                    lol.append(cadena)
                                    cadena=[]
                                    pass
                                lel = self.parselex(lol)
                                #print(lel)
                                linea.append(gen_value.code(lel, iterador))
                                modo="normal"
                                pass
                            else:
                                cadena.append(x)

                            pass
                        elif x["char"]=="{":
                            sub+=1
                            cadena.append(x)

                            pass
                        else:
                            cadena.append(x)
                        pass
                    else:
                        cadena.append(x)
                    pass
                pass
            if modo =="normal":
                if linea!=[]:
                    salida.append(linea)
                    linea=[]
                    pass
                pass
            else:
                if cadena!=[]:
                    lol.append(cadena)
                    cadena=[]
                    pass
            

            pass
        
        if modo!= "normal":
            self.error(f"Error syntax ", errores.ErrorSyntax, iterador)
        if linea!=[]:
            salida.append(linea)
            linea=[]
            pass
        

        return salida
    def argparse(self, data:list, func:list=[]) -> list:
        salida = []
        modo = "name"
        
        def structure(nombre:str, defe:list=[], ty:list=[])->dict["name":str, "ty":list]:
            if ty==[]:ty=[self.tydef]
            return ({
                "name":nombre,
                "def":self.estructuration_one(defe, func),
                "type":ty
            })
        
        nombre = ""
        defa = []
        typado = []
        #print(data)
        for i in data:
            #print(modo)
            if modo=="name":
                if i["tipo"]=="name":
                    nombre = i["name"]
                    if (str(nombre).count(".")!=0):
                        self.error("Error Syntax, token '.' is invalid", errores.ErrorSyntax, i["i"])#generar_error("error of syntax in var", i["i"])
                    modo="post"
                    pass
                pass
            elif modo=="post":
                #print(compara([{"tipo":"sim", "char":","}], [i]))
                if compara([{"tipo":"sim", "char":","}], [i]):
                    
                    salida.append(structure(nombre, defa, typado))
                    nombre = ""
                    defa=[]
                    typado = []
                    modo = "name"

                    pass
                elif compara([{"tipo":"ope", "char":"="}], [i]):
                    #print("llego")
                    if (defa==[]):
                        modo="def"
                    pass
                elif compara([{"tipo":"ope", "char":":"}], [i]) or compara([{"tipo":"ope", "char":"->"}], [i]): 
                    #print("llego")

                    if (typado==[]):
                        modo="typer"
                    pass
                
                pass
            elif modo=="typer":
                if i["tipo"]=="name":
                    typado = [i["name"]]
                    modo="post"
                    pass
                pass
            elif modo=="def":
                if compara([{"tipo":"sim", "char":","}], [i]):
                    
                    salida.append(structure(nombre, defa, typado))
                    nombre = ""
                    defa=[]
                    typado = []
                    modo = "name"

                    pass
                else:
                    defa.append(i)
                pass
            
            pass
        
        if nombre!="":
            salida.append(structure(nombre, defa, typado))
            
            pass
        

        return salida
    def foins(self):
       
        pass
    def estructuration(self, code:list, func:list=[]) -> list:
        
        modo:str = "normal"
        salida:list = []
        def sub_group(data:list) -> list:
            salida_out=  []
            out = []
            #print(data)

            for i in data["complet"]:
                salida_out.append(
                    self.estructuration_one(i, func)
                )
                pass

            if len(salida_out)>0:
                out=salida_out[0]
            
            return {
                "tipo":data["tipo"],
                "complet":salida_out,
                "data":out,
                "i":data["i"]
            }

        def generar_error(msg, i):
            def fallo(addi:str = "", index=i):
                if addi !="":
                    addi=" "+ str(addi)
                self.error(msg + addi, errores.ErrorSyntax, index)
                pass
            return fallo
        def gen_exe(c):
            return {
                "tipo":"exe",
                "exe":c,
                "i":c[0]["i"]
            }
        for i in code:
            visible="public"
            asyncrono= False
            
            
            if len(i)>1:
                #print("salida - ", i)
                if i[0]["tipo"] == "name":
                    if i[0]["tipo"] == "name": # es una declaracion de visibilidad?
                        if i[0]["name"] in nombre_reservados["visible"]:
                            visible = i[0]["name"]
                            i=i[1:]
                            
                            pass
                    if i[0]["tipo"] == "name": # es una declaracion de hilos?
                        if i[0]["name"] in ["async", "sync"]:
                            asyncrono = i[0]["name"]=="async"
                            i=i[1:]
                            pass
                    if i[0]["tipo"] == "name": # es una declaracion tipo C
                        if not (i[0]["name"] in nombre_reservados["nombre"]):
                            
                            if (len(i) == 4) and (compara(["name", "name", "()", "code"], i)):
                                i = [
                                    gen_char.names("function", i[0]["i"]),
                                    gen_char.names(i[1]["name"], i[0]["i"]),
                                    i[2],
                                    gen_char.ope("->", i[3]["i"]),
                                    gen_char.names(i[0]["name"], i[3]["i"]),
                                    i[3]
                                ]
                                pass
                            else:
                                if len(i) == 2:
                                    if compara(["name", "name"], i):
                                        i = [
                                            gen_char.names("var", i[0]["i"]),
                                            i[1],
                                            gen_char.ope(":", i[0]["i"], False),
                                            i[0]
                                            
                                        ]
                                        pass
                                    pass
                                elif len(i) > 3:
                                    if compara(["name", "name", {"tipo":"ope"}], i):
                                        i = [
                                            gen_char.names("var", i[0]["i"]),
                                            i[1],
                                            gen_char.ope(":", i[0]["i"], False),
                                            i[0],
                                            gen_char.ope("=", i[0]["i"], False),
                                            *i[3:]
                                            
                                        ]
                                        pass
                                    pass
                                else:
                                    pass
                                pass
                            pass
                        pass
                    if i[0]["tipo"] == "name": # es un loop
                        if i[0]["name"] == "loop":
                            if len(i) == 2:
                                if compara(["name", "code"], i):
                                    i = [gen_char.names("while", i[0]["i"]),
                                        gen_value.argu([
                                                [
                                                    gen_char.names("True", i[0]["i"])
                                                ]
                                            ], 
                                                i[0]["i"]
                                            ),
                                        i[1]
                                    ]
                                    pass
                                pass
                            pass
                        pass
                    #dim = [0, 0]
                    dim = find({"tipo":"ope", "char":"="}, i)
                    is_dim = dim[0]
                    i_dim = dim[1]
                    #print(i)

                    if i[0]["name"] in ["func", "function", "def", "fub", "method"]:#func-def
                        rt = self.tydef
                        arg = []
                        name = ""
                        codigo = []
                        funciones = []
                        #o = self
                        fallo = generar_error("error at try create a function", i[0]["i"])

                        #print(i)
                        if (len(i)==4): 
                            if compara(["name", "name", "()", "code"], i):
                                name = i[1]["name"]
                                arg = self.argparse(i[2]["data"], func)
                                codigo = self.estructuration(i[3]["data"], funciones)
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        elif (len(i)==6):
                            if compara(["name", "name", "()", {"tipo":"ope","char":"->"}, "name", "code"], i):
                                name = i[1]["name"]
                                arg = self.argparse(i[2]["data"], func)
                                codigo = self.estructuration(i[5]["data"], funciones)
                                rt = i[4]["name"]
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        else:
                            fallo()
                            pass

                        if name.count(".")>0:
                            fallo("(error token in name function '.')")
                        
                        f_a = {
                            "name":name,
                            "arg":arg,
                            "return":rt,
                            "code":{
                                "data":codigo,
                                "func":funciones
                            },
                            "tipo":"func-def",
                            "visible":visible,
                            "async":asyncrono,
                            "i":i[0]["i"]
                        }
                        salida.append([f_a])

                        pass
                    elif i[0]["name"] == "if":#if-def
                        fallo = generar_error("error at try create a if, elif or else", i[0]["i"])

                        est:list = []
                        o=0
                        while (len(i[o:o+3])>1):
                            t=i[o:o+3]
                            if len(t)==3:
                                if compara([{"tipo":"name", "name":"if"}, "()", "code"], t):
                                    ll = sub_group(t[1])["data"]
                                    if len(ll)==0:
                                        fallo(index=t[1]["i"])
                                        pass
                                    est.append(
                                        {
                                            "tipo":"if",
                                            "cond":ll,
                                            "code":self.estructuration(t[2]["data"], func)
                                        }
                                    )
                                    if not o==0:
                                        fallo("if?", t[1]["i"])
                                        pass
                                    pass
                                elif compara([{"tipo":"name", "name":"elif"}, "()", "code"], t):
                                    ll = sub_group(t[1])["data"]
                                    if len(ll)==0:
                                        fallo(index=t[1]["i"])
                                        pass
                                    est.append(
                                        {
                                            "tipo":"elif",
                                            "cond":ll,
                                            "code":self.estructuration(t[2]["data"], func)
                                        }
                                    )
                                    pass
                                elif compara([{"tipo":"name", "name":"elseif"}, "()", "code"], t):
                                    ll = sub_group(t[1])["data"]
                                    if len(ll)==0:
                                        fallo(index=t[1]["i"])
                                        pass
                                    est.append(
                                        {
                                            "tipo":"elif",
                                            "cond":ll,
                                            "code":self.estructuration(t[2]["data"], func)
                                        }
                                    )
                                    pass
                                elif compara([{"tipo":"name", "name":"else"}, {"tipo":"name", "name":"if"}, "()"], t):
                                    o+=1
                                    ll = sub_group(t[2])["data"]
                                    if len(ll)==0:
                                        fallo(index=t[2]["i"])
                                        pass
                                    if len(i[o+3:])==0:
                                        fallo(index=t[2]["i"])
                                        pass
                                    ccode = i[o+2]
                                    if ccode["tipo"]!="code":
                                        #print(ccode)
                                        fallo(index=ccode["i"])
                                    est.append(
                                        {
                                            "tipo":"elif",
                                            "cond":ll,
                                            "code":self.estructuration(ccode["data"], func)
                                        }
                                    )
                                    pass
                                else:
                                    fallo(index=t[0]["i"])
                                    pass
                                
                                pass
                            elif len(t)==2:
                                if compara([{"tipo":"name", "name":"else"}, "code"], t):
                                    est.append(
                                        {
                                            "tipo":"else",
                                            "code":self.estructuration(t[1]["data"], func)
                                        }
                                    )
                                    break
                                else:
                                    fallo(index=t[0]["i"])
                                    pass
                                pass
                            else:
                                fallo()


                            o+=3
                            pass
                        
                        salida.append([{
                            "tipo":"if-def",
                            "lista":est,
                            "i":i[0]["i"]
                        }])

                        pass
                    elif i[0]["name"] == "while":#while-def
                        fallo = generar_error("error to build at while", i[0]["i"])
                        if len(i)==3:
                            if compara(["name", "()", "code"], i):
                                ll = sub_group(i[1])["data"]
                                if len(ll)==0:
                                    fallo(index=i[1]["i"])
                                while_a = {
                                    "tipo":"while-def",
                                    "code":self.estructuration(i[2]["data"], func),
                                    "cond":ll,
                                    "i":i[0]["i"]
                                }
                                salida.append([while_a])
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "for":#for-def, for-each-def
                        fallo = generar_error("error to build at for", i[0]["i"])
                        if len(i)==5:
                            if compara(["name", {"tipo":"name", "name":"each"}, "name", "()", "code"], i):
                                for_a = {
                                    "tipo":"for-each-def",
                                    "code":self.estructuration(i[4]["data"], func),
                                    "cond":sub_group(i[3])["data"],
                                    "i":i[0]["i"],
                                    "var":i[2]["name"]
                                }
                                salida.append([for_a])
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        elif len(i)==3:
                            if compara(["name", "()", "code"], i):
                                if not len(i[1]["complet"])>2:
                                    fallo(
                                        "(the structure invalid) for (i = 0; i < len(Array()); i++) { ... }",
                                        i[1]["i"]    
                                    )
                                    pass
                                ttt = i[1]["complet"]
                                ttc = ["name", {"tipo":"ope", "char":"="}]
                                #print(ttt[0])
                                #print(ttc)
                                ttq = compara(ttc, ttt[0])
                                #print(ttq)
                                if not ttq:
                                    fallo(
                                        "(the structure invalid) for (i = 0; i < len(Array()); i++) { ... }",
                                        i[1]["i"]    
                                    )
                                    pass
                                for_a = {
                                    "tipo":"for-def",
                                    "code":self.estructuration(i[2]["data"], func),
                                    "for":sub_group(i[1])["complet"],
                                    "i":i[0]["i"]
                                }
                                salida.append([for_a])
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "switch":#switch-def
                        fallo = generar_error("error to build at switch", i[0]["i"])
                        
                        if len(i) != 3:
                            fallo()
                            pass
                        
                        if compara(["name", "()", "code"], i):
                            sw_a = {
                                "tipo":"switch-def",
                                "code":self.estructuration(i[2]["data"], func),
                                "cond":sub_group(i[1])["data"],
                                "i":i[0]["i"],
                            }
                            salida.append([sw_a])
                            pass
                        else:
                            fallo()
                            pass
                        pass
                        
                        pass
                    elif i[0]["name"] == "case":#case-def
                        fallo = generar_error("error to build at case", i[0]["i"])

                        if len(i) != 3:
                            fallo()
                            pass
                        
                        if compara(["name", "()", "code"], i):
                            case_a = {
                                "tipo":"case-def",
                                "code":self.estructuration(i[2]["data"], func),
                                "cond":sub_group(i[1])["data"],
                                "i":i[0]["i"],
                            }
                            salida.append([case_a])
                            pass
                        elif compara(["name", {"tipo":"name", "name":"default"}, "code"], i):
                            sw_a = {
                                "tipo":"case-d-def",
                                "code":self.estructuration(i[2]["data"], func),
                                "i":i[0]["i"],
                            }
                            salida.append([sw_a])
                            pass
                        else:
                            fallo()
                            pass
                        pass
                        
                        pass
                    elif i[0]["name"] == "class":#class-def
                        fallo = generar_error("error at try create a class", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "()", "code"], i):
                                if i[1]["name"].count("."): 
                                    fallo("(error token in name class '.')")
                                

                                funciones_clases = []
                                codigo = self.estructuration(i[3]["data"], func)
                                class_a = {
                                    "tipo":"class-def",
                                    "name":i[1]["name"],
                                    "extend":self.argparse(i[2]["data"]),
                                    "code":codigo,
                                    #"func-a":funciones_clases,
                                    "i":i[0]["i"],
                                    "visible":visible
                                }
                                salida.append([class_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "module":#module-def
                        fallo = generar_error("error at try create a module", i[0]["i"])
                        if len(i)==3:
                            if compara(["name", "name", "code"], i):
                                if i[1]["name"].count("."): 
                                    fallo("(error token in name module '.')")

                                funciones_clases = []
                                codigo = self.estructuration(i[2]["data"], funciones_clases)
                                class_a = {
                                    "tipo":"module-def",
                                    "name":i[1]["name"],
                                    "code":{
                                        "data":codigo,
                                        "func":funciones_clases
                                    },
                                    "i":i[0]["i"],
                                    "visible":visible
                                }
                                salida.append([class_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            #print("ehh")
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "struct":#struct-def
                        fallo = generar_error("error at try create a struct", i[0]["i"])
                        if len(i)==3:
                            if compara(["name", "name", "code"], i):
                                if i[1]["name"].count("."): 
                                    fallo("(error token in name struct '.')")

                                funciones_clases = []
                                codigo = self.estructuration(i[2]["data"], funciones_clases)
                                class_a = {
                                    "tipo":"struct-def",
                                    "name":i[1]["name"],
                                    "code":{
                                        "data":codigo,
                                        "func":funciones_clases
                                    },
                                    "i":i[0]["i"],
                                    "visible":visible
                                }
                                salida.append([class_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            #print("ehh")
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "namespace":#namespace-def
                        fallo = generar_error("error at try create a namespace", i[0]["i"])
                        if len(i)==3:
                            if compara(["name", "name", "code"], i):
                                if i[1]["name"].count("."): 
                                    fallo("(error token in name for namespace '.')")

                                funciones_clases = []
                                codigo = self.estructuration(i[2]["data"], funciones_clases)
                                class_a = {
                                    "tipo":"namespace-def",
                                    "name":i[1]["name"],
                                    "code":{
                                        "data":codigo,
                                        "func":funciones_clases
                                    },
                                    "i":i[0]["i"],
                                    "visible":visible
                                }
                                salida.append([class_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            #print("ehh")
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "with":#with-def
                        fallo = generar_error("error at try structurated with sub values", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "()", "code"], i):
                                funciones_clases = []
                                codigo = self.estructuration(i[3]["data"], func)
                                ll = sub_group(i[2])

                                if i[1]["name"].count("."): 
                                    fallo("(error token in name of var for with '.')")


                                if ll["data"]==[]:
                                    fallo()
                                with_a = {
                                    "tipo":"with-def",
                                    "name":i[1]["name"],
                                    "value":ll["data"],
                                    "code":codigo,
                                    #"func-a":funciones_clases,
                                    "i":i[0]["i"],
                                    "async":asyncrono
                                }
                                salida.append([with_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "from":#from-def
                        fallo = generar_error("error of syntax in from - import", i[0]["i"])
                        if len(i)==6:
                            if compara(["name", "value", {"tipo":"name", "name":"import"}, "name", {"tipo":"name", "name":"as"}, "name"], i):
                                
                                if i[5]["name"].count("."): 
                                    fallo("(error token in name of var for module to import '.')")
                                
                                from_a = {
                                    "tipo":"from-def",
                                    "from":eval(i[1]["value"]),
                                    "import":i[3]["name"],
                                    "as":i[5]["name"],
                                    "i":i[0]["i"]
                                }
                                salida.append([from_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "import":#import-def
                        fallo = generar_error("error of syntax in import", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "value", {"tipo":"name", "name":"as"}, "name"], i):
                                if i[3]["name"].count("."): 
                                    fallo("(error token in name of var for module to import '.')")

                                from_a = {
                                    "tipo":"import-def",
                                    "import":eval(i[1]["value"]),
                                    "as":i[3]["name"],
                                    "i":i[0]["i"]
                                }
                                salida.append([from_a])
                                pass
                            else:
                                fallo()
                            pass
                        else:
                            fallo()
                            pass
                        pass
                    elif i[0]["name"] == "include":#include-def
                        #generar_error("include is not sopport in this version of cls", i[0]["i"])()
                        if i[1]["tipo"] != "value":
                            generar_error("you must write the name of package in a String (include '[lib/pkg].scls')", i[0]["i"])()
                        
                        include_A = {
                            "tipo":"include-def",
                            "i":i[0]["i"],
                            "include":eval(i[1]["value"])
                        }
                        salida.append([include_A])
                        pass
                    elif i[0]["name"] == "template":#template-def
                        #generar_error("include is not sopport in this version of cls", i[0]["i"])()
                        fallo = generar_error("error to build at template", i[0]["i"])

                        if len(i) == 2:

                            if compara(["name", "value"], i):

                                include_A = {
                                    "tipo":"template-def",
                                    "i":i[0]["i"],
                                    "template":eval(i[1]["value"])
                                }
                                pass
                            else:
                                fallo()

                            pass
                        elif len(i) == 5:

                            if compara(["name", "value", {"tipo":"name", "name":"if"}, "name", "name"], i):

                                include_A = {
                                    "tipo":"template-if-def",
                                    "i":i[0]["i"],
                                    "template":eval(i[1]["value"]),
                                    "var": i[3]["name"],
                                    "value": i[4]["name"],
                                }
                                pass
                            else:
                                fallo()

                            pass
                        else:
                            fallo("syntax error")
                        

                        
                        salida.append([include_A])
                        pass
                    elif i[0]["name"] == "try":#try-def
                        if len(i)==2:
                            agregar = [
                                gen_char.names("error", i[0]["i"]),
                                gen_char.names("e", i[0]["i"]),
                                gen_value.code([], i[0]["i"])
                            ]
                            i = i + agregar
                            pass
                        if len(i)==5:
                            
                            fallo = generar_error("error at try structurated a try", i[0]["i"])

                            if compara(["name", "code", "name", "name", "code"], i):
                                if i[2]["name"] in ["error", "catch", "except", "failed", "fail"]:

                                    if i[3]["name"].count("."): 
                                        fallo("(error token in name of var for insert error '.')")

                                    try_a = {
                                        "tipo":"try-def",
                                        "try":self.estructuration(i[1]["data"], func),
                                        "error":self.estructuration(i[4]["data"], func),
                                        "e":i[3]["name"],
                                        "i":i[0]["i"]
                                    }

                                    salida.append([try_a])
                                    pass
                                else:
                                    fallo()
                                pass
                            else:
                                fallo()
                                pass                                


                            pass
                        pass
                    elif i[0]["name"] == "return":#rt-def
                        if len(i)>0:
                            return_a = {
                                "tipo":"rt-def",
                                "eval":self.estructuration_one(i[1:], func),
                                "i":i[0]["i"]
                            }
                            salida.append([return_a])
                            pass
                        else:
                            generar_error("error of syntax in return", i[0]["i"])
                            pass
                        pass
                    elif i[0]["name"] in ["var", "const"]:#var-def
                        #print(i)
                        if len(i)>1:
                            capture = i[1:]
                            if i[1]["tipo"] in ["code"]: 
                                capture = i[1]["one"]
                            return_a = {
                                "tipo": "var-def",
                                "dim": self.argparse(capture, func),
                                "i": i[0]["i"],
                                "visible": visible,
                                "const": i[0]["name"] == "const"
                            }
                            salida.append([return_a])
                            pass
                        else:
                            generar_error("error of syntax in var", i[0]["i"])
                            pass
                        pass
                    elif i[0]["name"] == "using":#using-comp
                        if len(i)>1:
                            #print("using")
                            return_a = {
                                "tipo":"using-comp",
                                "using":self.estructuration_one(i[1:], func),
                                "i":i[0]["i"]
                            }
                            salida.append([return_a])
                            pass
                        else:
                            generar_error("error of syntax at use 'using'", i[0]["i"])
                            pass
                        pass
                    elif is_dim:#var-eval
                        #print("es un dim")
                        if not len(i[i_dim+1:])>0:
                            generar_error("Error Syntax",i[i_dim]["i"])()
                        define_var = {
                            "tipo":"var-eval",
                            "var":self.estructuration_one(i[0:i_dim], func),
                            "eval":self.estructuration_one(i[i_dim+1:], func),
                            "visible":visible,
                            "onename":(i_dim==1) and not ("." in i[0]["name"]), 
                            "i":i[0]["i"]
                        }

                        salida.append([define_var])
                        pass
                    elif i[0]["name"] == "#modulo":
                        pass
                    else:#exe
                        salida.append(gen_exe(self.estructuration_one(i, func)))
                    if isinstance(salida[-1], list):
                        a=salida.pop()
                        salida.append(a[0])
                    pass
                else:#exe
                    salida.append(gen_exe(self.estructuration_one(i, func)))
                pass
            else:#exe
                salida.append(gen_exe(self.estructuration_one(i, func)))
            pass
        
        
        return salida
    def estructuration_one(self, code:list, func:list=[]) -> list:
        salida =[]

        #print("llego")
        def sub_group(data:list) -> list:
            salida_out=  []
            out = []
            if data["tipo"] in ["()", "[]"]:
                for i in data["complet"]:
                    salida_out.append(
                        self.estructuration_one(i, func)
                    )
                    pass

                if len(salida_out)>0:
                    out=salida_out[0]
                
                return {
                    "tipo":data["tipo"],
                    "complet":salida_out,
                    "data":out,
                    "i":data["i"]
                }
            elif data["tipo"] in ["code"]:
                
                for i in data["data"]:
                    salida_out.append(
                        self.estructuration_one(i, func)
                    )
                    pass

                if len(salida_out)>0:
                    out=salida_out[0]
                
                return {
                    "tipo":data["tipo"],
                    "data":salida_out,
                    "one":out,
                    "i":data["i"]
                }

                pass
        
        #e = 0
        #print("llamada")
        for i in code:#while(len(code)>e):
            #print("code", i)
            

            if i["tipo"] == "code":
                is_func:bool = False
                asyncrono:bool = False
                arg:list = []
                codigo:list = []
                devolver:str =self.tydef
                #print(salida[-2:])
                m=salida[-3:]
                n=salida[-2:]
                #print(len(m))
                if len(m)==3:
                    m=salida[-3:]
                    if compara(["()", {"tipo":"ope", "char":"->"}, "name"], m):
                        #print("llego")
                        devolver = salida.pop()["name"]
                        salida.pop()
                        arg=salida.pop()
                        codigo = i
                        #e+=3
                        is_func=True

                        pass
                    else:
                        pass
                    pass
                
                if len(n)==2 and (not is_func):
                    #print("funcion")
                    #print(n)
                    
                    if compara(["()", {"tipo":"ope", "char":"->"}], n):
                        codigo = i
                        salida.pop()
                        arg= salida.pop()
                        #e+=2
                        is_func=True
                        #print("funcion")
                        pass
                    else:
                        salida.append(i)
                        continue
                    pass
                #print(is_func)
                
                if is_func:
                    if len(salida)>0:
                        if salida[-1]["tipo"]=="name":
                            if salida[-1]["name"] in ["sync", "async"]:
                                asyncrono = salida[-1]["name"] == "async"
                                salida.pop()
                                pass
                            pass
                        pass
                    funciones = []
                    #print("salida")
                    f_A = {
                        "tipo":"func-a",
                        "arg":self.argparse(arg["data"]),
                        "code":{
                            "data":self.estructuration(codigo["data"], funciones),
                            "func":funciones
                        },
                        "return":devolver,
                        #"func-a":funciones,
                        "async":asyncrono,
                        "name":"t_tmp" + str(len(func))
                    }

                    salida.append(gen_char.names("t_tmp"+str(len(func)), i["i"], True))
                    func.append(["t_tmp" + str(len(func)), f_A])
                    #salida["func"]+=1
                    pass
                else:
                    salida.append(
                        sub_group(i)
                    )
                    pass
                pass
            elif i["tipo"] in ["[]", "()"]:
                
                salida.append(
                    sub_group(i)
                )

                

                if compara([
                    {"tipo":"name", "name":"if"},
                    "()",
                    {"tipo":"name", "name":"then"},
                    "()",
                    {"tipo":"name", "name":"else"},
                    "()",
                ], salida[-6:]):
                    #print("pasa")
                    if_expre = {
                        "tipo":"if-exp",
                        "i":salida[-6]["i"],
                        "if":sub_group(salida[-5]),
                        "then":sub_group(salida[-3]),
                        "else":sub_group(salida[-1]),
                    }

                    for i in range(0, 6): 
                        salida.pop()


                    salida.append(if_expre)
                    del if_expre

                    #print(salida)

                    pass
                else:

                    if i["tipo"] == "($)":

                        salida.pop()
                        is_func:bool = False
                        asyncrono:bool = False
                        arg:list = []
                        codigo:list = []
                        devolver:str =self.tydef
                        #print(salida[-2:])
                        m=salida[-3:]
                        n=salida[-2:]
                        #print(len(m))
                        print(i)
                        if len(m)==3:
                            m=salida[-3:]
                            if compara(["()", {"tipo":"ope", "char":"->"}, "name"], m):
                                #print("llego")
                                devolver = salida.pop()["name"]
                                salida.pop()
                                arg=salida.pop()
                                codigo = i
                                #e+=3
                                is_func=True

                                pass
                            else:
                                pass
                            pass
                        
                        if len(n)==2 and (not is_func):
                            #print("funcion")
                            #print(n)
                            
                            if compara(["()", {"tipo":"ope", "char":"->"}], n):
                                codigo = i
                                salida.pop()
                                arg= salida.pop()
                                #e+=2
                                is_func=True
                                #print("funcion")
                                pass
                            else:
                                salida.append(i)
                                continue
                            pass
                        #print(is_func)
                        
                        if is_func:
                            if len(salida)>0:
                                if salida[-1]["tipo"]=="name":
                                    if salida[-1]["name"] in ["sync", "async"]:
                                        asyncrono = salida[-1]["name"] == "async"
                                        salida.pop()
                                        pass
                                    pass
                                pass
                            funciones = []
                            #print("salida")
                            f_A = {
                                "tipo":"func-a-exp",
                                "arg":self.argparse(arg["data"]),
                                "code":{
                                    "data":self.estructuration_one(sub_group(codigo), funciones),
                                    "func":funciones
                                },
                                "return":devolver,
                                #"func-a":funciones,
                                "async":asyncrono,
                                "name":"t_tmp" + str(len(func))
                            }

                            salida.append(gen_char.names("t_tmp"+str(len(func)), i["i"], True))
                            func.append(["t_tmp" + str(len(func)), f_A])
                            #salida["func"]+=1
                            pass
                        else:
                            salida.append(
                                sub_group(i)
                            )
                            pass
                pass
            else:
                salida.append(i)
                pass

            #e=1+e; 
            
            pass

        return salida
    def exec(self, code:str="", root=True) -> any:
        self.api["_file"] = ObjectCls.String(self.origin)

        values = {
            "app":self,
            "MD":ObjectCls.Module,
            "NS":ObjectCls.Namespace,
            "NOMD":ObjectCls.NotModule,
            "ex":{"mem":{}},
            "c":c,
            "stasta":{
                "tae":self.memory.get("stasta", {}).get("tae", {}),
                "const":self.memory.get("stasta", {}).get("const", [])
            },
            str("var_"):"main",
            "errores":errores
        }
        len_name = len("var_")
        before = before_code
        after = after_code.replace("${l}", str(len_name)).replace("${n}", ("var_"))
        for i in self.api:
            values["var_" + i] = self.api[i]
            pass
        for i in self.memory:
            values[i] = self.memory[i]
            pass
        #print(code)
        exec(before +N+ code +after, values)
        try:
            pass
        except Exception as e:
            if True:
                salida = ""
                tipo = ""
                mensage = ""
                for e in self.cracheos:
                    salida += f"  file '{e['file']}' line {e['line']}, column {e['column']}" +N
                    salida += f"    {e['code']}" +N
                    count_tab = e["code"][0:e["column"]-1].count("\t")

                    salida +=  "    " + ("\t"*count_tab) + (" "*(e['column']-count_tab-1)) + "^" +N
                    tipo = e["type"]
                    mensage = e["msg"]
                    pass
                if root:
                    self.cracheos = []
                    print("--CLS Virtual Machine to stop" + N)
                    print(salida)
                    #print(mensage)
                    print(" TypeError: " + tipo)
                    print(" Detailed: " + str(mensage))
                else:
                    #self.parent.cracheos.append(*self.cracheos)

                    pass
            else:
                raise Exception(e)
            pass

        k = c(values["ex"]["mem"])
        
        t_out = {}

        for i in k:
            if i[0:4]=="var_":
                t_out[i] = k[i]
            pass
        #print(t_out)

        self.memory = t_out
        self.memory["stasta"] = {}
        self.memory["stasta"]["tae"] = k.get("sta_values", {})
        self.memory["stasta"]["const"] = k.get("constant", [])
        return None
    def dim(self, v, tipo) -> any:
        #print(v, tipo) 


        if confirm(self, v, tipo):
            return v
        elif tipo == ObjectCls.AnyObject:
            return v
        elif tipo == ObjectCls.void:

            self.catch(f"error the object '{que_tipo(v)}' can't set in this value", errores.ErrorTyping) 
        elif v == None:
            #print("out")
            if hasattr(tipo, "default"):
                #print("si")
                return tipo.default(tipo)
            else:
                #print("no")
                return None
        elif hasattr(tipo, "__clase__"):
            if isinstance(v, tipo.__clase__):
                return v
            else:
                self.catch(f"error the object '{que_tipo(v)}' can't set in a var of type '{tipo.__clase__.__name__}'", errores.ErrorTyping) 
            pass
        elif isinstance(v, tipo):
            return v
        else:
            print(v)
            self.catch(f"error the object '{que_tipo(v)}' can't set in a var of type '{tipo.__name__}'", errores.ErrorTyping)
        
        return v
    def constE(self, n):
        #print("Error")
        raise Exception(f"of value name '{n}' is a constant and can't re-define")
    def generator(self, c, modo:str="normal", usingmode = "compiled", data = {}) -> list:
        
        salida = []
        code = []
        func = []
        lastcode = []
        #print(usingmode)
        using_namespace = False

        GE = self.GeneratorEngine
        #modo = "normal"

        p_error = GE.engine_error
        print_arg = lambda x: GE.print_arg(x, self)

        _pass_arguments = [self, c, modo, usingmode, data]
        _pass_obligue_arguments = [self, c]

        
        
        if isinstance(c, dict):
            
            salida += GE.block_before(*_pass_arguments)
            
            code = c["data"]
            func = c["func"]
            pass
        elif isinstance(c, list):
            code = c
            func = []
            pass
        else: return []
        
        for i in func:
            salida += GE.function_lambda(*_pass_arguments, i)
            
            pass
        
        for i in code:
            salida += GE.sentence_before(*_pass_arguments)
            # Declaracion de Funciones
            

            if   (i["tipo"] == "func-def"):
                
                salida += GE.sentence_function(*_pass_arguments, i)

                pass
            
            #Sentencias, y operaciones basicas
            elif (i["tipo"] == "exe"):

                salida += GE.sentence_simple_exec(*_pass_arguments, i)
                pass
            elif (i["tipo"] == "if-def"):
                
                
                salida += GE.sentence_if(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "switch-def"):
                
                salida += GE.sentence_switch(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "case-def"):
                

                salida += GE.sentence_case(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "case-d-def"):


                salida += GE.sentence_case_default(*_pass_arguments, i, lastcode)


                pass
            elif (i["tipo"] == "while-def"):
                
                

                salida += GE.sentence_while(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "for-def"):
                
                

                salida += GE.sentence_for(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "for-each-def"):
                
                salida += GE.sentence_for_each(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "with-def"):
                
                salida += GE.sentence_with(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "rt-def"):
                
                
                salida += GE.sentence_return(*_pass_arguments, i)
                pass
            elif (i["tipo"] == "try-def"):
                
                salida += GE.sentence_try(*_pass_arguments, i)


                pass
            
            #Declaracion de Clases, Estructuras, Espacios de nombres y Modulos


            elif (i["tipo"] == "class-def"):
                
                salida += GE.sentence_class(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "module-def"):
                
                salida += GE.sentence_module(*_pass_arguments, i)
                
            
                # Espacios de nomres
            elif (i["tipo"] == "namespace-def"):
                
                salida += GE.sentence_namespace(*_pass_arguments, i)

                pass
            
                # Estructuras
            elif (i["tipo"] == "struct-def"):
                

                salida += GE.sentence_struct(*_pass_arguments, i)


                pass 
            
        

            #Elementos de compilacion
            elif (i["tipo"] == "$using-comp")      and (modo in ["normal", "func", "func-imp"]):
                
                #preparo = []
                lel = i["using"]


                #print(lel)

                if compara([{"tipo":"name", "name":"namespace"}, "name"], lel):
                    self.namespace = lel[1]["name"]
                    using_namespace =True
                    pass
                elif compara([{"tipo":"name", "name":"namespace"}], lel):
                    using_namespace =True
                    self.namespace = "std"
                    pass
                


                #salida+=preparo
                pass
            
            #Declaracion de Variables
            elif (i["tipo"] == "var-def"):
                
                salida += GE.sentence_vardefine(*_pass_arguments, i)
                pass
            elif (i["tipo"] == "var-eval"):
                
                # print("eval mode", i)

                salida += GE.sentence_vareval(*_pass_arguments, i)
                pass
            
            #Elementos de importacion de modulos externo e internos
            elif (i["tipo"] == "import-def"):
                
                salida += GE.sentence_import(*_pass_arguments, i)
                pass
            elif (i["tipo"] == "from-def"):
                salida += GE.sentence_from(*_pass_arguments, i)
                
                
                pass
            elif (i["tipo"] == "include-def"):
                
                salida += GE.sentence_include(*_pass_arguments, i)

                pass
            elif (i["tipo"] == "template-def"):
                
                salida += GE.sentence_template(*_pass_arguments, i)
                pass
            elif (i["tipo"] == "template-if-def"):
                #print(i, self.PRO.get(i["var"], None))
                salida += GE.sentence_template_if(*_pass_arguments, i)

                pass
            
            pass
        if (using_namespace) and (usingmode == "compiled"):
            #print("que?")
            self.namespace = "std"
        
        salida += GE.block_after(*_pass_arguments, lastcode)
        if isinstance(c, dict):
            code = c["data"]
            func = c["func"]
            pass
        
        #del c, using_namespace, usingmode

        return salida
    def generator_one(self, line:list, modo:str="normal", modi:str ="eval", key:bool=False) -> str:
        salida = ""
        last = {"tipo":"none"}
        iskey= False

        _pass_arguments = [self, line, modo, modi, key]

        GE = self.GeneratorEngine
        
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
                            # print(i)
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
            
            if True:
                __dict_arguments = {
                    "fallo": fallo,
                    "iskey": iskey,
                    "ite": ite
                }

                if i["tipo"]=="name":
                    salida += GE.expression_name(*_pass_arguments, i, **__dict_arguments)
                    pass
                elif i["tipo"] == "sim":
                    salida += GE.expression_sim(*_pass_arguments, i, **__dict_arguments)
                   
                    pass
                elif i["tipo"] == "ope":
                    salida += GE.expression_ope(*_pass_arguments, i, **__dict_arguments)
                    
                    
                    pass
                elif i["tipo"] == "()":
                    salida += GE.expression_tuple(*_pass_arguments, i, **__dict_arguments)
                    
                    pass
                elif i["tipo"] == "[]":
                    salida += GE.expression_list(*_pass_arguments, i, **__dict_arguments)
                    
                    pass
                elif i["tipo"] == "code":
                    be = f" {'{'+self.generator_one(i['one'], modo, 'eval', True)+'}'} "
                    if i["fist"]:
                        #print("code")

                        be = " app.fist("+be+") "
                    salida+= be
                    pass
                elif i["tipo"] == "value":
                    salida+= GE.expression_value(*_pass_arguments, i, **__dict_arguments)
                    pass
                elif i["tipo"] == "cml":
                    salida+= GE.expression_cml(*_pass_arguments, i, **__dict_arguments)
                    pass
                elif i["tipo"] == "if-exp":
                    salida+= GE.expression_if(*_pass_arguments, i, **__dict_arguments)
                    
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
    def jump(self, code:list=[], i=0) -> str:
        salida = ""
        #print(code)
        for x in code:
            if isinstance(x, str):
                salida+= ("    "*i)+x+N
                pass
            elif isinstance(x, list):
                salida+= self.jump(x, i+1)+N
                pass
            
            pass
        
        return salida
    def print_value(self, v) -> str:
        return self.GeneratorEngine.print_value(self, v)
    def fist(self, v):
        if isinstance(v, list):
            v = ObjectCls.Array(v)
        elif True in [isinstance(v, tuple), isinstance(v, set)]:
            raise Exception('the tokens "," is invalids')
        #print("fist value:", v)
        return v
    def fint(self, v, x):
        salida = 0
        
        salida = self.formato_int.get(x, lambda i:0)(v)

        return salida
    def cml(self, data):
        

        return ObjectCls.cml(data)
    def error(self, msg:(list[str]), type:str, i:int, before:str="") -> None:
        lin0 = self.codigo[0:i].count(N)
        lin1 = self.codigo.split(N, lin0+1)
        lin3 = self.codigo.split(N, lin0)
        lin3.pop()
        columna = i-len((" ").join(lin3))
        #print(f"{lin0} - {len(lin1)}")
        lin2 =lin1.pop()

        

        try:
            lin2 =lin1.pop()
        except:
            pass



        salida = f"script: '{self.origin}' line {lin0+1} column {columna}" + N 
        linea = self.codigo.split(N)[lin0]
        cursor = repeat(" ", columna) + "^"
        #print(self.codigo.split(N))
        gen = {
            "msg":msg,
            "code":linea,
            "cursor":cursor,
            "file":self.origin,
            "out":salida,
            "line":lin0+1,
            "column":columna,
            "type":type
        }
        k = self.cracheos[-1:]

        to = {
            "line":0,
            "column":0,
            "type":"None",
            "code":"-/None/-",
            "file":""
        }

        if len(k)==1:
            #print("si?")
            to = k[0]
        #print(k)
        gee = (
            (gen["line"] == to["line"]),
            (gen["code"] == to["code"]),
            (gen["column"] == to["column"]),
            (gen["file"] == to["file"])
            #(gen["type"] == to["type"]) 
        )
        #print(to)
        
        if False in gee:
            #print("añadir")
            #print(gee)

            self.cracheos.append(gen)
            if not self.root:
                self.parent.cracheos.append(gen)

        #raise Exception(before+N+f" error: {type}: {msg}"+N+ salida + cursor)
        raise Exception(msg)
    def code2class(self, *clases):

        out = []

        for i in clases:
            if isinstance(i, types.FunctionType):
                
                out.append(i.__clase__)
            if isinstance(i, type):
                
                out.append(i)
            
            pass

        return out
    pass


class EngineOutput():
    # infraestructura 
    def engine_error(self, code, i) -> list:
            
        return [
            "app.index = " + str(i),
            "try:",
            code,
            "except Exception as e:",
            [f"app.error(e, 'ErrorExecute', {i})"]
        ]
    def print_arg(self, args, engine) -> list:
            s_out = []
            it = -1
            for i in args:
                it+=1
                #print(i)
                arg=i["name"] 
                sta=i["type"][0] # "Any" default
                col = engine.generator_one(i["def"], "func")
                if col == "": col = "None"
                defa=(col)

                s_out+=[
                    f"try:",
                    f"    sta_var = var_{sta}",
                    f"except:",
                    f"    try:"
                    f"        sta_var = var_{sta}",
                    f"    except:",
                    f"        app.error('the {sta} class not found', errores.ErrorName)",
                    f"try:",
                    f"    var_{arg} = arg[{it}]",
                    f"except:",
                    f"    var_{arg} = ({defa})",
                    #f"print('var',var_{arg})",
                    #f"print('sta',var_{sta})",
                     #"print(globals().get('var_main', 'no'))",
                    f"app.dim(var_{arg}, sta_var)"
                ]
                pass
            
            return s_out
    def print_value(self, engine, v) -> str:
        salida = f"app.values['{v['type']}']({v['value']})"

        if v["type"] == "str":
            salida = f"app.str['{v['byte']}']({v['value']})"

        return salida
    
    # eventos de inicio y fin de bloque
    def block_before(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}) -> list:

        modo_a = [
            "sta_values = {}",
            "constant = {}"
            ]
        if modo == "normal":
            modo_a = [
                "sta_values = stasta.get('tae', {})",
                "constant   = stasta.get('const', [])",
                "stasta = {}",
                #"print(constant)"
            ]

            pass
        return[
            "app.variables.append([locals(), globals()])",
        ] + modo_a
    def block_after(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, lastcode:list =[]) -> list:
        out = []

        if modo == "switch":
            out += lastcode
        
        if isinstance(c, dict):
            out += ["app.variables.pop()"]

        return out
    def sentence_before(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, lastcode:list =[]) -> list:
        out = []

        if modo in ["normal", "func", "func-imp", "module", "class"]:
            out += ["app.foins()"]

        return out
    
    # funciones anonimas
    def function_lambda(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, iterator=[]) -> list:
        out = []
        if modo in ["normal", "func", "func-imp", "module", "class"]:

            nombre = iterator[0]
            fun = iterator[1]

            asy=""
            if fun["async"]: 
                asy = "async "
            preparo = [
                f"{asy}def {nombre}(*arg):",
                "    try:"
                f"        f_rt = (var_{fun['return']})",
                "    except:"
                f"        f_rt = (var_{fun['return']})",
                self.print_arg(fun["arg"], engine) +
                engine.generator(fun["code"], "func"),
                "    pass"
            ]

            out += preparo
            
        return out
    
    # generador de sentencias

    def sentence_function(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:

        # function main() {
        #     print("Hello");
        # }

        out = []

        def print_arg(args):

            return self.print_arg(args, engine)

        if   (modo in ["normal", "func-imp", "func"]):
            asy=""
            fun = sentence
            if fun["async"]: asy = "async "
            preparo = [
                f"{asy}def var_{fun['name']}(*arg):",
                f"    try:",
                f"        f_rt = (var_{fun['return']})",
                f"    except:",
                f"        f_rt = (var_{fun['return']})",
                print_arg(fun["arg"]) +
                engine.generator(fun["code"], "func"),
                "    pass"
            ]
            out+=preparo
            pass
        elif (modo in ["module"]):

            asy=""
            qqq=""
            fun = sentence
            
            
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
                engine.generator(fun["code"], "func"),
                "    pass",
                qqq,
                #qqq2
            ]
            out+=preparo
            pass
        elif (modo in ["class"]):

            asy=""
            qqq=""
            fun = sentence
            
            
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
                engine.generator(fun["code"], "func"),
                "    pass",
                qqq,
                qqq2
            ]
            out+=preparo
            pass
            

        return out
    def sentence_simple_exec(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        # function(20, 10) + 6

        if (modo in ["normal", "func-imp", "func"]):
            le = engine.generator_one(sentence["exe"], modo, "exec")
            return self.engine_error(
                [le],
                sentence["i"]
            )

        return []
    def sentence_if(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        if_out=[]
        if (modo in ["normal", "func-imp", "func"]):
            #tipo, cond, code
            for x in sentence["lista"]: 
                if x["tipo"] in ["if", "elif"]:
                    cond = engine.generator_one(x["cond"], modo)
                    codigo = engine.generator(x["code"], modo)
                    if_out += [
                        f"{x['tipo']} ({cond}):",
                        codigo,
                        "    pass"
                    ]
                else:
                    codigo = engine.generator(x["code"], modo)
                    if_out += [
                        f"else:",
                        codigo,
                        "    pass"
                    ]
                    break
                pass
            pass


        return self.engine_error(if_out, sentence["i"])
    def sentence_switch(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        if_out=[]
        if (modo in ["normal", "func-imp", "func"]):
            #tipo, cond, code
            if_out=[]
                #tipo, cond, code
            cond = engine.generator_one(sentence["cond"], modo)
            codigo = engine.generator(sentence["code"], "switch", data = {"modo": modo})
            if_out += [
                f"switch_val = ({cond})",
                    "if False:",
                    "    pass",
                    *codigo,
            ]


        return self.engine_error(if_out, sentence["i"])
    def sentence_case(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        
        if_out=[]
        #tipo, cond, code
        if (modo in ["switch"]):
            cond = engine.generator_one(sentence["cond"], data.get("modo", "normal"))
            codigo = engine.generator(sentence["code"], data.get("modo", "normal"))
            if_out += [
                f"elif ({cond}) == (switch_val):",
                    codigo,
                "    pass",
            ]
        return if_out
    def sentence_case_default(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}, lastcode:list=[]) -> list:
        if_out=[]

        #tipo, cond, code
        if (modo in ["switch"]):
            codigo = engine.generator(sentence["code"], data.get("modo", "normal"))
            if_out += [
                f"else:",
                    codigo,
                "    pass",
            ]

        for x in if_out:
            lastcode.append(x)
        return []
    def sentence_while(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            x=sentence
            cond = engine.generator_one(x["cond"], modo)
            codigo = engine.generator(x["code"], modo)
            out += [
                f"while ({cond}):",
                codigo,
                "    pass"
            ]

        return self.engine_error(out, sentence["i"])
    def sentence_for(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            x=sentence
            d_for =x["for"] #self.generator_one(x["for"], modo)
            cond = engine.generator_one(d_for[1], modo)
            codigo = engine.generator(x["code"], modo)
            post_code = engine.generator_one(d_for[2], modo, "exec")
        
            out += [
                f"var_{d_for[0][0]['name']} = ({engine.generator_one(d_for[0][2:], modo)})",
                f"while (True):",
                f"    if not ({cond}): break",
                    codigo,
                f"    {post_code}",
                    "    pass"
            ]

        return self.engine_error(out, sentence["i"])
    def sentence_for_each(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            x=sentence
            cond = engine.generator_one(x["cond"], modo)
            codigo = engine.generator(x["code"], modo)
        
            out += [
                f"for var_{x['var']} in ({cond}):",
                    codigo,
                "    pass"
            ]

        return self.engine_error(out, sentence["i"])
    def sentence_with(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            x=sentence
            cond = engine.generator_one(x["value"], modo)
            codigo = engine.generator(x["code"], modo)
            out += [
                f"def t_tmp_with(t_t):",
                f"    var_{x['name']} = t_t",
                    codigo,
                "    pass",
                f"t_tmp_with({cond})",
                f"del t_tmp_with"
            ]

        return self.engine_error(out, sentence["i"])
    def sentence_return(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["func"]):
            outt = engine.generator_one(sentence["eval"], modo)
            preparo = [
                f"return app.dim(({outt}), f_rt)"
            ]
            out += preparo
            

        return self.engine_error(out, sentence["i"])
    def sentence_try(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            x=sentence
            #cond = self.generator_one(x["cond"], modo)
            code_try = engine.generator(x["try"], modo)
            code_error = engine.generator(x["error"], modo)
            out += [
                "try:",
                    code_try,
                "    pass",
                "except Exception as ero:",
               f"    var_{x['e']} = ero",
                "    app.cracheos = []",
                    code_error,
                "    pass",
            ]

        return self.engine_error(out, sentence["i"])
    def sentence_class(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            fun = sentence
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
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
                    "me.__setattr__ = t_set_atr",
                    "private.__setattr__ = t_set_atr",
                #"exportar.__setattr__ = t_set_atr",
                
            ]
            
            preparo = [
                f"class tmp_class_{fun['name']}(*app.code2class({','.join(arg)})):",
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


                        engine.generator(fun["code"], "class"),


                f"    out.__export__ = exportar",
                f"    me.__export__ = exportar",
                 "    return out",
                f"var_{fun['name']} = tmp_var_{fun['name']}(tmp_class_{fun['name']})",
                f"var_{fun['name']}.__clase__ = tmp_class_{fun['name']}",
                    #self.generator(fun["code"], "class"),
            ]
            out+=preparo
        elif (modo in ["module"]):
            fun = sentence
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
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
                    "me.__setattr__ = t_set_atr",
                    "private.__setattr__ = t_set_atr",
                #"exportar.__setattr__ = t_set_atr",
                
            ]
            ta = "me"

            if fun["visible"] == "private":
                ta = "private"
            
            preparo = [
                f"class tmp_class_{fun['name']}(*app.code2class({','.join(arg)})):",
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


                        engine.generator(fun["code"], "class"),


                f"    out.__export__ = exportar",
                f"    me.__export__ = exportar",
                    "    return out",
                f"{ta}.{fun['name']} = tmp_var_{fun['name']}(tmp_class_{fun['name']})",
                f"{ta}.{fun['name']}.__clase__ = tmp_class_{fun['name']}",
                    #self.generator(fun["code"], "class"),
            ]
            out+=preparo

        return self.engine_error(out, sentence["i"])
    def sentence_module(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            fun = sentence

            preparo2 = [
                    "def t_get_atr(self, v): ",
                    "    return None",
                
                    "me.__getattr__ = t_get_atr",
                    "private.__getattr__ = t_get_atr",
                #"exportar.__getattr__ = t_get_atr",
                
                    "def t_set_atr(self, a, v): ",
                #"    print('atr:', a)",
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
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
                    engine.generator(fun["code"], "module"),
                #f"    var_private = private()",
                    "    return me()",
                f"var_{fun['name']} = (var_{fun['name']})()"
            ]

            out += preparo
        elif (modo in ["module"]):
            fun = sentence

            preparo2 = [
                    "def t_get_atr(self, v): ",
                    "    return None",
                
                    "me.__getattr__ = t_get_atr",
                    "private.__getattr__ = t_get_atr",
                #"exportar.__getattr__ = t_get_atr",
                
                    "def t_set_atr(self, a, v): ",
                #"    print('atr:', a)",
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
                "me.__setattr__ = t_set_atr",
                "private.__setattr__ = t_set_atr",
                #"exportar.__setattr__ = t_set_atr",
                
            ]

            ta = "me"

            if fun["visible"] == "private":
                ta = "private"
            
            preparo = [
                f"def var_{fun['name']}():",
                    "    class private:pass",#"    private = (MD())",
                    "    class me:pass",
                        preparo2,
                f"    var_private = private()",
                f"    private = var_private",
                    engine.generator(fun["code"], "module"),
                #f"    var_private = private()",
                    "    return me()",
                f"{ta}.{fun['name']} = (var_{fun['name']})()"
            ]
            out+=preparo

        return self.engine_error(out, sentence["i"])
    def sentence_namespace(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            fun = sentence

            preparo2 = [
                    "def t_get_atr(self, v): ",
                    "    return None",
                
                    "me.__getattr__ = t_get_atr",
                    "private.__getattr__ = t_get_atr",
                #"exportar.__getattr__ = t_get_atr",
                
                    "def t_set_atr(self, a, v): ",
                #"    print('atr:', a)",
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
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
                    engine.generator(fun["code"], "module"),
                #f"    var_private = private()",
                    "    return NS(me())",
                f"var_{fun['name']} = (var_{fun['name']})()"
            ]
            out+=preparo

        return self.engine_error(out, sentence["i"])
    def sentence_struct(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out=[]

        #tipo, cond, code
        if (modo in ["normal", "func-imp", "func"]):
            fun = sentence

            preparo2 = [
                    "def t_get_atr(self, v): ",
                    "    return None",
                
                    "me.__getattr__ = t_get_atr",
                    "private.__getattr__ = t_get_atr",
                #"exportar.__getattr__ = t_get_atr",
                
                    "def t_set_atr(self, a, v): ",
                #"    print('atr:', a)",
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
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
                        engine.generator(fun["code"], "struct"),
                #f"    var_private = private()",
                    "    return me",
                f"var_{fun['name']} = (var_{fun['name']})()"
            ]
            out+=preparo
        elif (modo in ["module"]):
            fun = sentence

            preparo2 = [
                    "def t_get_atr(self, v): ",
                    "    return None",
                
                    "me.__getattr__ = t_get_atr",
                    "private.__getattr__ = t_get_atr",
                #"exportar.__getattr__ = t_get_atr",
                
                    "def t_set_atr(self, a, v): ",
                #"    print('atr:', a)",
                f"    self.__dict__[a] = app.dim(v, sta_values.get('var_'+a, var_{engine.tydef}))",
                
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
                    engine.generator(fun["code"], "struct"),
                #f"    var_private = private()",
                    "    return me",
                f"{ta}.{fun['name']} = (var_{fun['name']})()"
            ]
            out+=preparo

        return self.engine_error(out, sentence["i"])
    def sentence_vardefine(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []
        if modo in ["normal", "func-imp", "func"]:
            ordenar = []
            
            for x in sentence["dim"]:
            
                #print(i)
                arg=x["name"]
                sta=x["type"][0]
                col = engine.generator_one(x["def"], "func")
                if col == "": col = "None"
                defa=(col)

                qqq = ""
                if sentence["const"]:qqq = f"constant.append('var_{arg}')"

                ordenar+=self.engine_error([
                    
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
                    f"var_{arg} = app.dim(var_{arg}, sta_var)",
                    qqq
                ], sentence["i"])
                pass

            out += ordenar
        elif modo in ["class", "module", "struct"]:
            ordenar = []                
            for x in sentence["dim"]:
                
                #print(i)
                arg=x["name"]
                sta=x["type"][0]
                col = engine.generator_one(x["def"], "func")
                if col == "": col = "None"
                defa=(col)

                na = "me"
                qqq = ""
                qqq2 = ""
                if sentence["const"]:qqq2 = f"constant.append('var_{arg}')"


                if sentence["visible"] == "private":
                    na = "private"
                    pass
                elif sentence["visible"] == "static" and modo == "class":
                    na = "out"
                    pass
                elif sentence["visible"] == "export" and modo == "class":
                    qqq = f"exportar['{arg}'] = app.dim(var_{arg}, sta_var)"
                    pass
                
                

                ordenar+= self.engine_error([
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
                ], sentence["i"])
                pass

            out += ordenar

        return self.engine_error(out, sentence["i"])
    def sentence_vareval(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []
        if modo in ["normal", "func-imp", "func", "class", "module", "struct"]:
            if sentence["onename"]:
                t_name = sentence["var"][0]["name"]
                qqq = f"if 'var_{t_name}' in constant: app.constE('var_{t_name}')"
                #print("onename", t_name)
                #print(qqq)
                if modo in ["func", "func-imp", "normal"]:
                    g_1 = engine.generator_one(sentence["var"], modo)
                    g_2 = engine.generator_one(sentence["eval"], modo)
                    preparo = self.engine_error([
                        qqq,
                        f"{g_1} = app.dim(({g_2}), sta_values.get('{g_1}', var_{engine.tydef}))",
                    ], sentence["i"])
                    pass
                elif modo in ["class", "struct"]:

                    g_1 = sentence["var"][0]["name"]
                    g_2 = engine.generator_one(sentence["eval"], modo)
                    preparo = self.engine_error([
                        qqq,
                        f"{g_1} = ({g_2})",
                    ], sentence["i"])
                    pass
                elif modo in ["module"]:

                    g_1 = sentence["var"][0]["name"]
                    g_2 = engine.generator_one(sentence["eval"], modo)
                    preparo = self.engine_error([
                        qqq,
                        f"me.{g_1} = ({g_2})",
                        f"var_{g_1} = (me.{g_1})",
                    ], sentence["i"])
                    pass
                
                
                pass
            else:
                g_1 = engine.generator_one(sentence["var"], modo)
                g_2 = engine.generator_one(sentence["eval"], modo)
                preparo = self.engine_error([
                    f"{g_1} = ({g_2})"
                ], sentence["i"])
                pass

            out+=preparo
            pass
        return self.engine_error(out, sentence["i"])
    def sentence_import(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []
        if modo in ["normal", "func-imp", "func"]:
            
            out+=[
                f"var_{sentence['as']} = app.getlib('{sentence['import']}')"
            ]

            pass
        return self.engine_error(out, sentence["i"])
    def sentence_from(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []
        if modo in ["normal", "func-imp", "func"]:
            
            out += [
                f"var_{sentence['as']} = app.getlib('{sentence['from']}').{sentence['import']}"
            ]

            pass
        return self.engine_error(out, sentence["i"])
    def sentence_include(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []
        if modo in ["normal", "func-imp", "func"]:
            
            out += [
                f"incluir = app.getlib('{sentence['include']}')",
                    #"print(type(incluir))",
                    "for x in dir(incluir):",
                    #"    print(getattr(incluir, x))",
                f"    globals()['var_' + x] = getattr(incluir, x)",
                f"    locals()['var_' + x] = getattr(incluir, x)",
            ]

            pass
        return self.engine_error(out, sentence["i"])
    def sentence_template(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []

        if modo in ["normal", "func-imp", "func"]:

            pati = lib.find(sentence["template"])

            tem = []
            if pati == "None":
                print(f"template '{sentence['template']}' not found")
                pass
            else:
                _ftp = open(pati, "r")
                tem = _ftp.read().split("\n")
                _ftp.close()
                pass

            out += tem



        return self.engine_error(out, sentence["i"])
    def sentence_template_if(self, engine, c, modo:str="normal", usingmode = "compiled", data = {}, sentence={}) -> list:
        out = []

        if modo in ["normal", "func-imp", "func"]:

            if self.PRO.get(sentence["var"], None) == sentence["value"]:
                pati = lib.find(sentence["template"])

                tem = []
                if pati == "None":
                    print(f"template '{sentence['template']}' not found")
                    pass
                else:
                    _ftp = open(pati, "r")
                    tem = _ftp.read().split("\n")
                    _ftp.close()
                    pass

                out += tem
                pass



        return self.engine_error(out, sentence["i"])
    # generador de expresiones

    def expression_name(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            if iskey:
                #print("error 2", i)

                t = {"tipo":"value", "value":f"'{expression['name']}'", "type":"str", "i":expression["i"], "byte":""}
                out+= f" {engine.print_value(t)} "
                pass
            elif expression["notmod"]:
                out+= f" {expression['name']} "
                pass
            elif expression["name"] in nombre_reservados["codi"]:
                out+= f" {expression['name']} "
                pass                  
            elif (expression["name"] in nombre_reservados["bucle"]):
                if modi == "exec":
                    if out == "":
                        out+= f" {expression['name']} "
                    else:
                        fallo(f"Error Syntax with the token '{expression['name']}'")
                    pass
                else:
                    fallo(f"the token '{expression['name']}' is invalid in this case")
                pass
            elif expression["name"][0]==".":
                out+= f"{expression['name']} "
                pass
            elif expression["name"][0]=="0":
                
                out+= f" app.fint('{expression['name']}', '{expression['name'][1]}') "
                pass
            elif compara([{"tipo":"ope", "char":"::"}], line[ite+1:ite+2]):
                out+= f" var_{expression['name']}.__names__"
                pass
            elif compara([{"tipo":"ope", "char":"::"}], line[ite-1:ite]):
                out+= f".{expression['name']}"
                pass
            else:
                out+= f" var_{expression['name']} "
                pass
            pass
        return out
    def expression_sim(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            out += f" {expression['char']} "
            pass
        return out
    def expression_ope(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            if expression["char"] in ["+", "-", "*", "/", "**", "%", ":"]:
                out += f" {expression['char']} "
            elif expression["char"] in tokens["convert"]["condi"]:
                out += f" {tokens['convert']['condi'][expression['char']]} "
                pass
            elif expression["char"] in tokens["convert"]["expre-"+str(modi)]:
                out += f" {tokens['convert']['expre-'+str(modi)][expression['char']]} "
                pass
            elif expression["char"] in tokens["cond"]:
                out += f" {expression['char']} "
            elif (expression["char"] in ["="]) and (modi=="exec"):
                out += f" = "
            
            
        return out
    def expression_tuple(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            be = f" ({engine.generator_one(expression['data'], modo)}) "
            if expression["fist"]:
                be = " app.fist("+be+") "
            out+= be
            pass
        return out
    def expression_list(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            be = f" [{engine.generator_one(expression['data'], modo)}] "
            if expression["fist"]:
                be = " app.fist("+be+") "
            out+= be
            pass
        return out
    def expression_code(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            be = f" {'{'+engine.generator_one(expression['one'], modo, 'eval', True)+'}'} "
            if expression["fist"]:
                be = " app.fist("+be+") "
            out+= be
            pass
        return out
    def expression_value(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            out += f" {engine.print_value(expression)} "
            pass
        return out
    def expression_cml(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            out += f" app.cml({repr(expression['data'])}) "
            pass
        return out
    def expression_if(self, engine, line:list, modo:str="normal", modi:str ="eval", key:bool=False, expression={}, ite:int = 0, fallo:types.FunctionType = (lambda x: []), iskey:bool=False) -> str:
        out = ""
        if modo in ["func", "func-imp", "class", "module", "normal", "struct"]:
            be_if = f" ({engine.generator_one(expression['then']['data'], modo)}) "
            be_else = f" ({engine.generator_one(expression['else']['data'], modo)}) "
            be_cond = f" ({engine.generator_one(expression['if']['data'], modo)}) "

            out += f" ({be_if} if {be_cond} else {be_else}) "
            pass
        return out
    pass








appcls = appclsBase
