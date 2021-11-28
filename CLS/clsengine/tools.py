
from clsengine.gen_char import *
from clsengine.chars import *
#from clsengine.ObjectCls import * 

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
    
