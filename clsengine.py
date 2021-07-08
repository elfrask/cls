from copy import copy as c
import typing

uid:int = 0

N="\n"
T="\t"
R="\r"
B="\b"
COMILLAS="\""
APOSTROFE="\'"


tokens = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^", ":"],
    "sim":["{", "}", "(", ")", "[", "]", ","],
    "cond":["==", "<", ">", "!=", "<=", ">=", "!"]
}

nombre_reservados = {
    "visible":["export", "static", "private", "public", "global"],
    "nombre":[
        "func", "function", "class", "module", "with", "for", "if", "while", "define",
        "from", "import", "global", "try", "def", "fub", "method",
        ]
}
class errores:
    ErrorSyntax:str="ErrorSyntax"
    ErrorSemant:str="ErrorSemant"
    ErrorAritmetic:str="ErrorAritmetic"
    pass

def que_tipo(obj:any)->str:
    return str(type(obj)).split("'")[1]
def tipo_valor(v:str) -> list[str, str]:
    tipo = "name"
    try:
        float(v)
        tipo="float"
        pass
    except:
        try:
            int(v)
            tipo="int"
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
    
    if len(data)<len(obj):
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
    def names(name:str, i:int, mod:bool=False) -> dict["name":str, "i":int, "tipo":str, "notmod":bool]:

        return {
            "tipo":"name",
            "name":name,
            "i": i,
            "notmod":mod
        }
    def val(value:str, type:str, i:int, lim:str="'", byte:str="") -> (
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
    def ope(char:str, i:int, ope:bool =False) -> (
        dict["char":str, "i":int, "ope":bool, "tipo":str]
        ):

        return {
            "char":char,
            "i":i,
            "ope":ope,
            "tipo":"ope"
        }
    def sim(char:str, i:int) -> (
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
        self.namespace = "std"

        pass
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
                                if (not (xx["char"] in ["=", ":", "<", ">"])) or (c in ["<", ">"]):
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

                    #print(i)
                    dim = find({"tipo":"ope", "char":"="}, i)
                    is_dim = dim[0]
                    t_dim = dim[1]
                    #print("dim exacto", i[t_dim])

                    if i[0]["name"] in ["func", "function", "def", "fub", "method"]:
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
                            "code":codigo,
                            "func-a":funciones,
                            "tipo":"func-def",
                            "visible":visible,
                            "async":asyncrono,
                            "i":i[0]["i"]
                        }
                        salida.append([f_a])

                        pass
                    elif i[0]["name"] == "if":
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
                    elif i[0]["name"] == "while":
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
                    elif i[0]["name"] == "for":
                        fallo = generar_error("error to build at for", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", {"tipo":"name", "name":"each"}, "()", "code"], i):
                                for_a = {
                                    "tipo":"for-each-def",
                                    "code":self.estructuration(i[3]["data"], func),
                                    "cond":sub_group(i[2])
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
                                        "(the structure invalid) for (i = 0; i < len(Array()); i+=1) { ... }",
                                        i[1]["i"]    
                                    )
                                    pass
                                for_a = {
                                    "tipo":"for-def",
                                    "code":self.estructuration(i[2]["data"], func),
                                    "cond":sub_group(i[1]),
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
                    elif i[0]["name"] == "class":
                        fallo = generar_error("error at try create a class", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "()", "code"], i):
                                funciones_clases = []
                                codigo = self.estructuration(i[3]["data"], funciones_clases)
                                class_a = {
                                    "tipo":"class-def",
                                    "name":i[1]["name"],
                                    "extend":i[2]["data"],
                                    "code":codigo,
                                    "func-a":funciones_clases,
                                    "i":i[0]["i"]
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
                    elif i[0]["name"] == "module":
                        fallo = generar_error("error at try create a module", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "code"], i):
                                funciones_clases = []
                                codigo = self.estructuration(i[2]["data"], funciones_clases)
                                class_a = {
                                    "tipo":"module-def",
                                    "name":i[1]["name"],
                                    "code":codigo,
                                    "func-a":funciones_clases,
                                    "i":i[0]["i"]
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
                    elif i[0]["name"] == "with":
                        fallo = generar_error("error at try structurated with sub values", i[0]["i"])
                        if len(i)==4:
                            if compara(["name", "name", "()", "code"], i):
                                funciones_clases = []
                                codigo = self.estructuration(i[3]["data"], funciones_clases)
                                ll = sub_group(i[2])
                                if ll["data"]==[]:
                                    fallo()
                                with_a = {
                                    "tipo":"with-def",
                                    "name":i[1]["name"],
                                    "value":ll["data"],
                                    "code":codigo,
                                    "func-a":funciones_clases,
                                    "i":i[0]["i"]
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
                    elif i[0]["name"] == "from":
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
                    elif i[0]["name"] == "import":
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
                    elif i[0]["name"] == "define":
                        generar_error("include is not sopport in this version of cls", i[0]["i"])()
                        pass
                    elif i[0]["name"] == "try":
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
                    elif i[0]["name"] == "return":
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
                    elif i[0]["name"] == "var":
                        pass
                    elif i[0]["name"] == "if":
                        pass
                    elif is_dim:
                        #print("es un dim")
                        pass
                    else:
                        salida.append(self.estructuration_one(i, func))

                    #nombre = i[0]["name"]

                    pass
                else:
                    salida.append(self.estructuration_one(i, func))
                pass
            else:
                salida.append(self.estructuration_one(i, func))
            pass

        
        return salida
    def estructuration_one(self, code:list, func:list=[]) -> list:
        salida =[]
        e = 0

        def sub_group(data:list) -> list:
            salida_out=  []
            out = []
            #print("llego")

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
        
        

        while(len(code)>e):
            #print(e)
            i = code[e]

            if i["tipo"] == "()":
                is_func:bool = False
                asyncrono:bool = False
                arg:list = []
                codigo:list = []
                devolver:str =self.tydef
                if len(code[e:e+3])>= 3:
                    if compara(["()", {"tipo":"ope", "char":"->"}, "code"], code[e:]):
                        arg=code[e]
                        codigo = code[e+2]
                        e+=2
                        is_func=True
                        pass
                    else:
                        salida.append(i)
                        continue
                    pass
                if len(code[e:e+4])>= 4:
                    if compara(["()", {"tipo":"ope", "char":"->"}, "name", "code"], code[e:]):
                        arg=code[e]
                        codigo = code[e+3]
                        devolver = code[e+2]
                        e+=3
                        is_func=True

                        pass
                    else:
                        salida.append(i)
                        continue
                    pass
                
                if is_func:
                    if len(salida)>0:
                        if salida[0]["tipo"]=="name":
                            if salida[0]["name"] in ["sync", "async"]:
                                asyncrono = salida[0]["name"] == "async"
                                salida.pop()
                                pass
                            pass
                        pass
                    funciones = []
                    f_A = {
                        "tipo":"func-a",
                        "arg":self.argparse(arg["data"]),
                        "code":self.estructuration(codigo["data"], funciones),
                        "return":devolver,
                        "func-a":funciones,
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
            elif i["tipo"]=="[]":
                salida.append(
                    sub_group(i)
                )
                pass
            else:
                salida.append(i)
                pass

            e+=1; 
            
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
        cursor = "      " + repeat(" ", columna) + "^"
        raise ValueError(before+N+f" error: {type}:{msg}"+N+ salida + cursor)
    pass

#crea el dim despues de hacer la pagina de tu padre