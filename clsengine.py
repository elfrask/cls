
from typing import Dict


uid:int = 0

N="\n"
T="\t"
R="\r"
B="\b"
COMILLAS="\""
APOSTROFE="\'"


tokens = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^"],
    "sim":["{", "}", "(", ")", "[", "]"],
    "cond":["==", "<", ">", "!=", "<=", ">="]
}

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

class gen_char:
    def names(name:str, i:int) -> dict["name":str, "i":int, "tipo":str]:

        return {
            "tipo":"name",
            "name":name,
            "i": i
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

        pass
    def set_api(self, api:object) -> None:

        pass
    def desline(self, code:str, name:str="file0") -> list:
        salida:list = []
        linea:list = []
        self.codigo = code
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
                                        gen_char.ope(c+xx["char"], 
                                        iterador-1, 
                                        (c+xx["char"]) in tokens["cond"])
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
        for i in code:
            linea:list = []
            for x in i:
                
                if modo == "normal":
                    if x["tipo"]=="sim":
                        if x["char"]=="(":
                            pass
                        else:
                            char = x["char"]
                            self.error(f"error syntax: '{char}'", x["i"])
                        pass
                    else:
                        linea.append(x)
                    pass

                pass
            pass

        return salida
    def error(self, msg:str, type:str, i:int) -> None:
        lin0 = self.codigo[0:i].count(N)
        lin1 = self.codigo.split(N, lin0+1)
        lin3 = self.codigo.split(N, lin0)
        lin3.pop()
        columna = i-len((" ").join(lin3))
        lin1.pop()

        lin2 = lin1.pop()
        salida = f"-script: {self.origin}" + N + f"-code: {lin0}:{columna} {lin2}" + N

        raise (f"{type}:{msg}"+N+ salida)
    pass

#no estaria mal probar el sistema de errores, despues te toca hacer los parselex