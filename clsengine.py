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
    "visible":["export", "static", "private", "public"],
    "nombre":[
        "func", "function", "class", "module", "with", "for", "if", "while", "define",
        "from", "import", "global", "try"
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

class gen_value:
    def lista(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "complet":list]
        ):
        out=[]
        if len(data)>0: out = data[0]
        return {
            "tipo":"[]",
            "data":out,
            "i":i,
            "complet":data
        }
    def argu(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "complet":list]
        ):
        out=[]
        if len(data)>0: out = data[0]
        return {
            "tipo":"()",
            "data":out,
            "i":i,
            "complet":data
        }
    def code(data:list=[], i:int=0)->(
        dict["tipo":str, "i":int, "data":list, "one":list]
        ):
        out=[]
        if len(data)>0: out = data[0]
        return {
            "tipo":"code",
            "data":data,
            "i":i,
            "one":out
        }
    

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
                                if not (xx["char"] in ["=", ":", "<", ">"]):
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
        for i in code:
            visible="public"
            asyncrono= False
            
            if len(i)>1:
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

                    if i[0]["name"] in ["func", "function"]:
                        rt = self.tydef
                        arg = []
                        name = ""
                        codigo = []
                        funciones = []
                        #o = self
                        def fallo(addi:str = ""):
                            if addi !="":
                                addi=" "+ str(addi)
                            self.error("error at declare a function" + addi, errores.ErrorSyntax, i[0]["i"])
                            pass

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
                            "async":asyncrono
                        }
                        salida.append([f_a])

                        pass
                    else:
                        salida.append([self.estructuration_one(i, func)])

                    #nombre = i[0]["name"]

                    pass
                else:
                    salida.append([self.estructuration_one(i, func)])
                pass
            else:
                salida.append([self.estructuration_one(i, func)])
            pass

        return salida
    def estructuration_one(self, code:list, func:list=[]) -> list:
        salida =[]
        e = 0

        def sub_group(data:list) -> list:
            salida_out=  []
            out = []

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

#se usara un grupo de funciones al final para remplazar a las funciones anonimas