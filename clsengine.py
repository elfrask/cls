from copy import Error, copy as c
import typing

uid:int = 0

N="\n"
T="\t"
R="\r"
B="\b"
COMILLAS="\""
APOSTROFE="\'"

class Any():
    def __init__(self) -> None:
        pass
    pass

after_code="""

mem_out = c(locals())
ti = {}
for i in mem_out:
    if i[0:${l}] == "${n}":
        ti[i] = mem_out[i]
    pass
ex["mem"] = ti
"""

tokens = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^", ":"],
    "sim":["{", "}", "(", ")", "[", "]", ","],
    "cond":["==", "<", ">", "!=", "<=", ">=", "!"],
    "convert":{
        "condi":{"&":"and", "|":"or", "!":"not"},
        "expre-eval":{"++":"+1", "--":"-1"},
        "expre-exec":{"++":"+=1", "--":"-=1"},
    },
    "metodos":{"main":"__init__", "_call":"__call__", "_getitem":"__getitem__", "_setitem":"__setitem__",
        "_add":"__add__", "_sub":"__sub__", "_div":"__div__", "_delitem":"__delitem__", "_mul":"__mul__",
        "_mod":"__mod__", "_or":"__or__", "_and":"__and__", "_xor":"__xor__", "_or":"__or__", "_len":"__len__"
    },
    "to_c":{"String":"str", "Array":"list", "Int":"int", "Float":"float", "Dict":"dict"}
}

nombre_reservados = {
    "visible":["export", "static", "private", "public", "global"],
    "nombre":[
        "func", "function", "class", "module", "with", "for", "if", "while", "define",
        "from", "import", "global", "try", "def", "fub", "method", "include"
        ],
    "codi":["or", "in", "and", "is"],
    "bucle":["break", "continue"]
}
class errores:
    ErrorSyntax:str="ErrorSyntax"
    ErrorSemant:str="ErrorSemant"
    ErrorAritmetic:str="ErrorAritmetic"
    ErrorTyping:str="ErrorTyping"
    pass

def que_tipo(obj:any)->str:
    return str(type(obj)).split("'")[1]
def tipo_valor(v:str) -> list[str, str]:
    tipo = "name"
    try:
        int(v)
        tipo="int"
        pass
    except:
        try:
            float(v)
            tipo="float"
            pass
        except:
            tipo = "name"
            pass
    return [v, tipo]
def assign_valor(v:str, t:str, i:int) -> dict:
    salida = 0
    if t=="int":
        salida = gen_char.val(v, "int", i-len(v))
    elif t=="float":
        salida = gen_char.val(v, "float", i-len(v))
    else:
        salida = gen_char.names(v, i-len(v))
    
    return salida
def find(token:dict, lista:dict) -> list[bool, int]:
    #salida = False
    iterador = 0
    for i in lista:
        if token["tipo"] == i["tipo"]:
            l = []
            for k in token:
                o = (token[k] == i[k])
                l.append(o)
                pass

            if not False in l:
                return [True, iterador]
            pass
        iterador+=1
        pass
    return [False, 0]
def trim_string(v:str = "") -> str:
    salida = v
    while salida[:1]==" ":
        salida = salida[1:]
        pass
    while salida[-1:]==" ":
        salida = salida[:-1]
        pass
    return salida
def list_to_dict(lista:list=[])->dict:
    salida = {}
    iterador = -1
    for i in lista: 
        iterador+=1
        salida[iterador] = i
    return salida
def repeat(char:str=" ", veces:int=1) -> str:
    salida=""

    for i in range(0,veces):
        salida=salida+char

    return salida
def get(obj:any, index:any, default:any=0)->list:
    salida =0
    try:
        salida=obj[index]
    except:
        salida = default
    
    return salida
def compara(data:any, obj:any)->bool:
    yei = True
    
    if len(data)>len(obj):
        #print("ehh")
        return False
    

    for i in range(0, len(data)):
        
        if isinstance(data[i], str):
            if not data[i]==obj[i]["tipo"]: 
                yei = False
                pass
            pass
        elif isinstance(data[i], dict):
            if data[i]["tipo"]==obj[i]["tipo"]:
                for x in data[i]:
                    if data[i][x]!=obj[i][x]: 
                        yei = False
                        pass
                    else:
                        pass
                    pass
                pass
            else:
                yei = False
            
            pass
        else:
            yei = False
            pass
        pass
    #print(yei)
    return yei
def tablines(data:str, i:int) -> str:

    salida = data.replace(N, N+ repeat(T, i))
    salida = repeat(T, i)+salida

    return salida

class gen_char:
    def names(name:str, i:int, mod:bool=False) -> (#name
        dict["name":str, "i":int, "tipo":str, "notmod":bool]
        ):

        return {
            "tipo":"name",
            "name":name,
            "i": i,
            "notmod":mod
        }
    def val(value:str, type:str, i:int, lim:str="'", byte:str="") -> (#value
        dict["value":str, "tipo":str, "i":int, "l":str, "type":str, "byte":str]
        ):

        return {
            "value":value,
            "tipo":"value",
            "i":i,
            "l":lim,
            "type":type,
            "byte":byte
        }
    def ope(char:str, i:int, ope:bool =False) -> (#ope
        dict["char":str, "i":int, "ope":bool, "tipo":str]
        ):

        return {
            "char":char,
            "i":i,
            "ope":ope,
            "tipo":"ope"
        }
    def sim(char:str, i:int) -> (#sim
        dict["char":str, "i":int, "tipo":str]
        ):

        return {
            "char":char,
            "i":i,
            "tipo":"sim"
        }

derivados = True
key_true = True


class gen_value:
    def lista(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "complet":list]
        ):
        out=[]
        if len(data)>0: out = data[0]
        salida = {
            "tipo":"[]",
            "data":out,
            "i":i,
            #"complet":data
        }
        if derivados: 
            salida["complet"]=data
        return salida
    def argu(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "complet":list]
        ):
        out=[]
        if len(data)>0: out = data[0]
        salida = {
            "tipo":"()",
            "data":out,
            "i":i,
            #"complet":data
        }
        if derivados:
            salida["complet"] = data
        return salida
    def code(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "one":list]
        ):
        out=[]
        if len(data)>0: out = data[0]
        salida = {
            "tipo":"code",
            "data":data,
            "i":i,
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
class ObjectCls:
    class AnyObject:pass
    class void:pass
    class Array(list):pass
    class Module():
        def __dict__():
            pass
        pass
    class clase():
        def __init__(self, obj) -> None:
            self.obj = obj
            pass
        def __call__(self, *args: Any, **kwds: Any) -> Any:
            return self.obj(*args, **kwds)
        pass

class clsapi:
    def input(msg:str=""):
        return
    pass

class appcls():
    def __init__(self, id:int = uid) -> None:
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
        self.tydef = "Any"
        self.namespace = "v"
        self.index=0
        self.memory = {}
        self.values={
            "str":str,
            "int":int,
            "float":float
        }
        self.api = {
            "print":print,
            "str":str,
            "String":str,
            "Any":ObjectCls.AnyObject,
            "any":ObjectCls.AnyObject,
            "int":int,
            "float":float,
            "void":ObjectCls.void,
            "Array":ObjectCls.Array,
            "array":ObjectCls.Array,
            "module":ObjectCls.Module,
            "Module":ObjectCls.Module,
            "function":que_tipo.__class__,
            "Function":que_tipo.__class__,
            "Boolean":bool,
            "bool":bool,
            "len":len,

            "true":True,
            "false":False,            
            "True":True,
            "False":False,
            "on":True,
            "off":False,
        }
        
        pass
    def exception(self, msg:str=""): # in exec
        raise Exception(msg)
    def catch(self, msg:str="", e:str="ErrorCatch"): # in exec
        self.error(msg, e, self.index)
    def set_api(self, api:object) -> None:

        pass
    def desline(self, code:str, name:str="file0") -> list:
        salida:list = []
        linea:list = []
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
                                if (not (xx["char"] in ["=", ":", "<", ">"])) or (c in ["<", ">", "="]):
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
                            cadena+c,
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

            pass
        if cadena!="":
            val = tipo_valor(cadena)
            out = assign_valor(val[0], val[1], iterador+1)
            cadena=""
            linea.append(out)
            pass
        if linea!=[]:
            salida.append(linea)


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

        def structure(nombre:str, defe:list=[], ty:list=[])->dict["name":str, "def":list, "ty":list]:
            if ty==[]:ty=[self.tydef]
            return ({
                "name":nombre,
                "def":self.estructuration_one(defe, func),
                "type":ty
            })
        
        nombre = "";
        defa = []
        typado = []
        #print(data)
        for i in data:
            #print(modo)
            if modo=="name":
                if i["tipo"]=="name":
                    nombre = i["name"]
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
                elif compara([{"tipo":"ope", "char":":"}], [i]): 
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
        #funcional:bool = func == [] 

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
                if i[0]["tipo"]=="name":
                    if i[0]["tipo"] == "name":
                        if i[0]["name"] in nombre_reservados["visible"]:
                            visible = i[0]["name"]
                            i=i[1:]
                            
                            pass
                    if i[0]["tipo"] == "name":
                        if i[0]["name"] in ["async", "sync"]:
                            asyncrono = i[0]["name"]=="async"
                            i=i[1:]
                            pass
                    if i[0]["tipo"] == "name":
                        if not (i[0]["name"] in nombre_reservados["nombre"]):
                            if len(i)==4:
                                if compara(["name", "name", "()", "code"], i):
                                    i = [
                                        gen_char.names("function", i[0]["i"]),
                                        gen_char.names(i[1]["name"], i[0]["i"]),
                                        i[2],
                                        gen_char.ope("->", i[3]["i"]),
                                        gen_char.names(i[0]["name"], i[3]["i"]),
                                        i[3]
                                    ]
                                    pass
                                pass
                            pass
                        pass
                    if i[0]["tipo"] == "name":
                        if i[0]["name"] == "loop":
                            if len(i) == 2:
                                if compara(["name", "code"]):
                                    i = [gen_char.names("while", i[0]["i"]),
                                        gen_value.argu([
                                                [
                                                    gen_char("True", i[0]["i"])
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

                    dim = find({"tipo":"ope", "char":"="}, i)
                    is_dim = dim[0]
                    i_dim = dim[1]

                    if i[0]["name"] in ["func", "function", "def", "fub", "method"]:#func-def
                        rt = self.tydef
                        arg = []
                        name = ""
                        codigo = []
                        funciones = []
                        #o = self
                        fallo = generar_error("error at try create a function", i[0]["i"])

                        if (len(i)==4): 
                            if compara(["name", "name", "()", "code"], i):
                                name = i[1]["name"]
                                arg = self.argparse(i[2]["data"])
                                codigo = self.estructuration(i[3]["data"], funciones)
                                pass
                            else:
                                fallo()
                                pass
                            pass
                        elif (len(i)==6):
                            if compara(["name", "name", "()", {"tipo":"ope","char":"->"}, "name", "code"], i):
                                name = i[1]["name"]
                                arg = self.argparse(i[2]["data"])
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
                            fallo("(no se amite '.')")
                        
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
                    elif i[0]["name"] == "class":#class-def
                        fallo = generar_error("error at try create a class", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "()", "code"], i):
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
                    elif i[0]["name"] == "with":#with-def
                        fallo = generar_error("error at try structurated with sub values", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "()", "code"], i):
                                funciones_clases = []
                                codigo = self.estructuration(i[3]["data"], func)
                                ll = sub_group(i[2])
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
                        if len(i)==6:
                            if compara(["name", "value", {"tipo":"name", "name":"as"}, "name"], i):
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
                    elif i[0]["name"] == "include":#inculde-def
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
                        if len(i)>1:
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
                    elif is_dim:#var-eval
                        #print("es un dim")
                        if not len(i[i_dim+1:])>0:
                            generar_error("Error Syntax",i[i_dim]["i"])()
                        define_var = {
                            "tipo":"var-eval",
                            "var":self.estructuration_one(i[0:i_dim], func),
                            "eval":self.estructuration_one(i[i_dim+1:], func),
                            "visible":visible,
                            "onename":(is_dim==1) and ("." in i[0]["name"]), 
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

                if len(n)==2:
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
                if len(m)==3:
                    m=salida[-3:]
                    
                    if compara(["()", {"tipo":"ope", "char":"->"}, "name"], m):
                        devolver = salida.pop()["name"]
                        salida.pop()
                        arg=salida.pop()
                        codigo = i
                        #e+=3
                        is_func=True

                        pass
                    else:
                        salida.append(i)
                        continue
                    pass
                
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
                pass
            else:
                salida.append(i)
                pass

            #e=1+e; 
            #print("llego")
            
            pass

        return salida
    def exec(self, code:str="") -> any:

        values = {
            "app":self,
            "MD":ObjectCls.Module,
            "ex":{"mem":{}},
            "c":c,
            str(self.namespace+"_"):"main"
        }
        len_name = len(self.namespace+"_")
        before = ""
        after = after_code.replace("${l}", str(len_name)).replace("${n}", (self.namespace+"_"))
        #print(after)
        for i in self.api:
            values[self.namespace + "_" + i] = self.api[i]
            pass
        
        exec(before + code +after, values, self.memory)

        self.memory = values["ex"]["mem"]

        return None
    def dim(self, v, tipo) -> any:
        #print(v, tipo)
        if tipo == ObjectCls.AnyObject:
            return v
        elif tipo == ObjectCls.void:
            return v
        if isinstance(v, tipo):
            return v
        else:   
            self.catch(f"error the object '{que_tipo(v)}' can't set in a var of type '{tipo.__name__}'", errores.ErrorTyping)
        
        return v
    def generator(self, c, modo:str="normal") -> list:
        
        salida = []
        code = []
        func = []
        #modo = "normal"

        def p_error(code, i) -> list:
            
            return [
                "app.index = " + str(i),
                "try:",
                code,
                "except Exception as e:",
                [f"app.error(e, 'ErrorExecute', {i})"]
            ]
        def print_arg(args) ->list:
            s_out = []
            it = -1
            for i in args:
                it+=1
                #print(i)
                arg=i["name"]
                sta=i["type"][0]
                col = self.generator_one(i["def"], "func")
                if col == "": col = "None"
                defa=(col)

                s_out+=[
                    f"try:",
                    f"    {self.namespace}_{arg} = arg[{it}]",
                    f"except:",
                    f"    {self.namespace}_{arg} = ({defa})",
                    f"app.dim({self.namespace}_{arg}, {self.namespace}_{sta})"
                ]
                pass
            
            return s_out

        if isinstance(c, dict):
            code = c["data"]
            func = c["func"]
            pass
        elif isinstance(c, list):
            code = c
            func = []
            pass
        else:
            #print("nada...")
            return []
        #print(isinstance(c, list))
        for i in func:
            nombre = i[0]
            fun = i[1]

            asy=""
            if fun["async"]: asy = "async "
            preparo = [
                f"{asy}def {nombre}(*arg):",
                f"    f_rt = ({self.namespace}_{fun['return']})",
                print_arg(fun["arg"]) +
                self.generator(fun["code"], "func"),
                "    pass"
            ]
            salida+=preparo
            pass
        #print(modo)
        for i in code:
            if   (i["tipo"] == "func-def")     and (modo in ["normal", "func-imp", "func"]):
                asy=""
                fun = i
                if fun["async"]: asy = "async "
                preparo = [
                    f"{asy}def {self.namespace}_{fun['name']}(*arg):",
                    f"    f_rt = ({self.namespace}_{fun['return']})",
                    print_arg(fun["arg"]) +
                    self.generator(fun["code"], "func"),
                    "    pass"
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "func-def")     and (modo in ["module"]):
                asy=""
                fun = i
                if fun["async"]: asy = "async "
                preparo = [
                    f"{asy}def {self.namespace}_{fun['name']}(*arg):",
                    f"    f_rt = ({self.namespace}_{fun['return']})",
                    print_arg(fun["arg"]) +
                    self.generator(fun["code"], "func"),
                    "    pass",
                    f"me.{fun['name']} = {self.namespace}_{fun['name']}"
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "func-def")     and (modo in ["class"]):

                asy=""
                qqq=""
                fun = i
                if fun["name"][0:2] == "__": 
                    if fun["name"][-2:] == "__": 
                        continue
                if fun["async"]: asy = "async "
                rename= False
                #nombre = self.namespace+"_"+fun['name']
                nombre = fun["name"]
                if fun["name"] in tokens["metodos"]:
                    nombre = tokens["metodos"][fun["name"]]
                    pass
                if (fun["name"][0:2] == "to") and (fun["name"][2:] in tokens["metodos"]):
                    nombre = "__"+tokens["metodos"][fun["name"][2:]] + "__"
                    pass
                
                
                if rename: 
                    qqq= self.namespace+"_"+fun['name'] + " = " + nombre
                preparo = [
                    f"{asy}def {nombre}(self, *arg):",
                    f"    f_rt = ({self.namespace}_{fun['return']})",
                    f"    {self.namespace}_me = self",
                    print_arg(fun["arg"]) +
                    self.generator(fun["code"], "func"),
                    "    pass",
                    qqq
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "exe")          and (modo in ["normal", "func-imp", "func"]):
                le = self.generator_one(i["exe"], modo, "exec")
                salida += p_error(
                    [le],
                    i["i"]
                )
                pass
            elif (i["tipo"] == "if-def")       and (modo in ["normal", "func-imp", "func"]):
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
            elif (i["tipo"] == "while-def")    and (modo in ["normal", "func-imp", "func"]):
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
            elif (i["tipo"] == "for-def")    and (modo in ["normal", "func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                d_for =x["for"] #self.generator_one(x["for"], modo)
                cond = self.generator_one(d_for[1], modo)
                codigo = self.generator(x["code"], modo)
                post_code = self.generator_one(d_for[2], modo, "exec")
                if_out += [
                    f"{self.namespace}_{d_for[0][0]['name']} = ({self.generator_one(d_for[0][2:], modo)})",
                    f"while (True):",
                    f"    if not ({cond}): break",
                    codigo,
                    f"    {post_code}",
                     "    pass"
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "for-each-def") and (modo in ["normal", "func-imp", "func"]):
                if_out=[]
                #tipo, cond, code
                x=i
                cond = self.generator_one(x["cond"], modo)
                codigo = self.generator(x["code"], modo)
                if_out += [
                    f"for {self.namespace}_{x['var']} in ({cond}):",
                        codigo,
                    "    pass"
                ]
                

                salida += p_error(if_out, i["i"])

                pass
            elif (i["tipo"] == "class-def")    and (modo in ["normal", "func-imp", "func", "class"]):
                fun = i
                arg= []
                for x in fun["extend"]:
                    arg.append(self.namespace+"_"+x["arg"]) 
                preparo = [
                    f"class {self.namespace}_{fun['name']}({','.join(arg)}):",
                        self.generator(fun["code"], "class"),
                     "    pass"
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "module-def")   and (modo in ["normal", "func-imp", "func", "class"]):
                fun = i
                
                preparo = [
                    f"def {self.namespace}_{fun['name']}(me):",
                        self.generator(fun["code"], "module"),
                     "    return me",
                    f"{self.namespace}_{fun['name']} = ({self.namespace}_{fun['name']})(MD())"
                ]
                salida+=preparo
                pass
            elif (i["tipo"] == "var-eval")     and (modo in ["normal", "func-imp", "func", "class", "module"]):
                
                if i["onename"]:
                    if modo in ["func", "func-imp", "normal"]:
                        g_1 = self.generator_one(i["var"], modo)
                        g_2 = self.generator_one(i["eval"], modo)
                        preparo = p_error([
                            f"{g_1} = ({g_2})"
                        ], i["i"])
                        pass
                    elif modo in ["class"]:

                        g_1 = i["var"][0]["name"]
                        g_2 = self.generator_one(i["eval"], modo)
                        preparo = p_error([
                            f"{g_1} = ({g_2})"
                        ], i["i"])
                        pass
                    elif modo in ["module"]:

                        g_1 = i["var"][0]["name"]
                        g_2 = self.generator_one(i["eval"], modo)
                        preparo = p_error([
                            f"me.{g_1} = ({g_2})",
                            f"{self.namespace}_{g_1} = (me.{g_1})",
                        ], i["i"])
                        pass
                    
                    
                    pass
                else:
                    g_1 = self.generator_one(i["var"], modo)
                    g_2 = self.generator_one(i["eval"], modo)
                    preparo = p_error([
                        f"{g_1} = ({g_2})"
                    ], i["i"])
                    pass

                salida+=preparo
                pass
            
            
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
    def print_value(self, v):
        salida = f"app.values['{v['type']}']({v['value']})"
        return salida
    def generator_one(self, line:list, modo:str="normal", modi:str ="eval", key:bool=False) -> str:
        salida = ""
        last = {"tipo":"none"}
        iskey= False
        

        for i in line:
            #print(i)
            def fallo(msg:str="", x:int=False):
                if x == False:
                    x = i["i"]
                if msg=="": msg = "Error Syntax"
                #print("Fallo", x)
                self.error(msg, errores.ErrorSyntax, x)
            if True:
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
                            pass
                        pass
                last = i
                pass
            
            if modo in ["func", "func-imp", "class", "module", "normal"]:

                if i["tipo"]=="name":
                    if iskey:
                        t = {"tipo":"value", "value":f"'{i['name']}'", "type":"str", "i":i["i"]}
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
                    else:
                        salida+= f" {self.namespace}_{i['name']} "
                        pass
                    pass
                elif i["tipo"] == "sim":
                    salida += f" {i['char']} "
                    pass
                elif i["tipo"] == "ope":
                    if i["char"] in ["+", "-", "*", "/", "**", "%"]:
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
                    salida+= f" ({self.generator_one(i['data'], modo)}) "
                    pass
                elif i["tipo"] == "[]":
                    salida+= f" [{self.generator_one(i['data'], modo)}] "
                    pass
                elif i["tipo"] == "code":
                    salida+= f" {'{'+self.generator_one(i['one'], modo, 'eval', True)+'}'}] "
                    pass
                elif i["tipo"] == "value":
                    salida+= f" {self.print_value(i)} "
                    pass
                
                pass
            
            pass
        while salida[:1]==" ":
            salida = salida[1:]
            pass
        while salida[-1:]==" ":
            salida = salida[:-1]
            pass
        
            
        return salida
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

        salida = f"   script: '{self.origin}' line {lin0+1} column {columna}" + N + f"      {lin2}" + N
        cursor = "     " + repeat(" ", columna) + "^"
        raise Exception(before+N+f" error: {type}: {msg}"+N+ salida + cursor)
    pass

#debes de crear los modulos y construir el resto de funcionalidades, PyCLS casi terminado