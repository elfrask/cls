import xml.dom.minidom as xml
import threading


class Any():
    def __init__(self) -> None:
        pass
    pass

class ObjectCls:
    def val(v=None):

        
        if v.__class__ in co_co: return co_co[v.__class__](v)
        

        return v
    class AnyObject:pass
    class void:pass
    class Array(list):
        def forEach(self, callback):
            iterador = 0
            for i in self:
                callback(i, iterador)
                iterador+=1
                pass
            pass
        def map(self, callback):
            salida = []
            iterador=0

            for i in self:
                salida+=[callback(i, iterador)]
                iterador+=1

                pass
            return ObjectCls.val(salida)
        def filter(self, callback):
            salida = []
            iterador=0

            for i in self:
                sal=callback(i, iterador)
                if sal != None:
                    salida.append(sal)
                    pass
                iterador+=1

                pass
            return ObjectCls.val(salida)
        def default(clase):
            return clase([])
        pass
    class Module():
        def __dict__():
            pass
        pass
    class NotModule():
        def __getattr__(self, v):
            return None
        pass
    class clase():
        def __init__(self, obj) -> None:
            self.obj = obj
            pass
        def __call__(self, *args: Any, **kwds: Any) -> Any:
            return self.obj(*args, **kwds)
        pass
    class char():
        def __init__(self, value="", limit = -1) -> None:
            self.limit= limit
            self.set(value)
            pass
        def __repr__(self) -> str:
            return f"c'{self.__str__()}'"
        def __str__(self) -> str:
            return f"{self.value}"
        def __list__(self, v) -> list[str]:
            return list(self.value)
        def __len__(self) -> int:
            return len(self.value)
        def set(self, value):
            self.value = value
            if self.limit != -1:
                self.value = value[0:self.limit]
            pass
        pass
    class String(str):
        def default(clase):
            return clase("")
        pass
    class Integer(int):
        def default(clase):
            return clase(0)
        pass
    class Float(float):
        def default(clase):
            return clase(0.0)
        pass
    class hex():
        def __init__(self, v):
            if (isinstance(v, int) or isinstance(v, ObjectCls.Integer)):
                v = hex(int(v))
            if isinstance(v, self.__class__):
                self = v
            self.value = str(v)
            self.lin = 16

            try: int(v, self.lin)
            except: raise Exception(f'Error at convert this value "{v}" to hex')
            pass
        def default(clase):
            return clase("0x0")
        def __int__(self):
            return obj.Integer(int(self.value, self.lin))
        def __float__(self):
            return obj.Float(int(self.value, self.lin))
        def __repr__(self):
            return f'hex({self.value})'
        def __str__(self):
            return self.value
    
    
        
        
        pass
    class bin():
        def __init__(self, v):
            if (isinstance(v, int) or isinstance(v, ObjectCls.Integer)):
                v = bin(int(v))
            if isinstance(v, self.__class__):
                self = v
            self.value = str(v)
            self.lin = 2
            
            try: int(v, self.lin)
            except: raise Exception(f'Error at convert this value "{v}" to bin')
            pass
        def default(clase):
            return clase("0b0")
        def __int__(self):
            return obj.Integer(int(self.value, self.lin))
        def __float__(self):
            return obj.Float(int(self.value, self.lin))
        def __repr__(self):
            return f'hex({self.value})'
        def __str__(self):
            return self.value
    class oct():
        def __init__(self, v):
            if (isinstance(v, int) or isinstance(v, ObjectCls.Integer)):
                v = oct(int(v))
            if isinstance(v, self.__class__):
                self = v
            self.value = str(v)
            self.lin = 8
            
            try: int(v, self.lin)
            except: raise Exception(f'Error at convert this value "{v}" to oct')
            pass
        def default(clase):
            return clase("0c0")
        def __int__(self):
            return obj.Integer(int(self.value, self.lin))
        def __float__(self):
            return obj.Float(int(self.value, self.lin))
        def __repr__(self):
            return f'hex({self.value})'
        def __str__(self):
            return self.value
    class Bytes(bytes):
        def __getitem__(self, a):
            b = list(self)

            return obj.Integer(b[a])
        def default(clase):
            return clase(b'')
    
        pass
    class Boolean():
        def __init__(self, v):
            if (isinstance(v, str) or isinstance(v, ObjectCls.String)):
                if str(v) in ["on", "off", "true", "false", "True", "False"]:
                    self.str = str(v)
                    
                    pass
                else:
                    self.str = str(bool(v))
            else:
                self.str = str(bool(v))
            pass
        def default(clase):
            return clase("True")
        def __str__(self):
            return self.str
        def __repr__(self):
            return self.str
        def __bool__(self):
            return self.str in ["on", "true", "True"]
        def __int__(self):
            return obj.Integer(self.str in ["on", "true", "True"])
        def __float__(self):
            return obj.Float(self.str in ["on", "true", "True"])
        
        

        pass
    class intbit():
        def __init__(self, v, l=8):
            self.l = l
            #print("llego")
            self.set(v)
            pass
        def default(clase):
            
            return clase()
        def set(self, v):
            self.value = int(v)%(int('1'*self.l, 2)+1)
            pass
        def __int__(self):
            return obj.Integer(self, self.value)
        def __str__(self):
            return obj.String(f"intbit[{self.l}]({self.value})")
        pass
    class Namespace():

        def __init__(self, modulo) -> None:
            self.__names__ = modulo
            pass
        def __str__(self) -> str:
            return "[Namespace Module]"
        def __repr__(self) -> str:
            return "[Namespace Module]"
        
        pass
    class cml():
        def __init__(self, data):
            data = (f"<xml> {data} </xml>")
            self.__x = xml.parseString(str(data)).childNodes[0]
            
            pass
        def getById(self, id):
            return tonode(self.__x.getElementById(id))
        def __str__(self):
            return self.__x.toxml() 
        def tojson(self):
            return tonode(self.__x)
        pass
    class Promise():
        
        call_then = None
        call_catch = None

        res_then = None
        res_catch = None

        def __init__(self, call) -> None:
            self.__names__ = call

            if not callable(call):
                pass

            def res(*response):
                self.res_then = response
                if (callable(self.call_then)):
                    self.call_then(*response)
                pass
            def err(*response):
                self.res_catch = response
                if (callable(self.call_catch)):
                    self.call_then(*response)
                pass
            
            threading.Timer(0.0, call, [res, err]).start()

            pass
        def then(self, call):
            self.call_then = call
            if isinstance(self.res_then, tuple):
                call(*self.res_then)
                pass

            return self
        def catch(self, call):
            self.call_catch = call
            if isinstance(self.res_catch, tuple):
                call(*self.res_catch)
                pass
            
            return self
        def __str__(self) -> str:
            return "[Namespace Module]"
        def __repr__(self) -> str:
            return "[Namespace Module]"
        pass

obj = ObjectCls

co_co = {
    str:ObjectCls.String,
    int:ObjectCls.Integer,
    float:ObjectCls.Float,
    list:ObjectCls.Array,
}


derivadas = {
    ObjectCls.String:[
        str, ObjectCls.String
    ],
    ObjectCls.Integer:[
        int, ObjectCls.Integer
    ],
    ObjectCls.Float:[
        float, ObjectCls.Float
    ],
    ObjectCls.Array:[
        list, ObjectCls.Array
    ],
    
}

def confirm(self, v, tipo):
    if v.__class__ in derivadas.get(tipo, []):
        return True
    return False
def tonode(nodo):
    salida = {}
    nodos = []
    teo = dict(nodo.attributes)
    print(nodo)
    try:
        for i in list(teo):
            salida[i] = teo[i].value
            pass
        
        k = list(nodo.childNodes)
        
        
        for t in k:
            print(t)
            try: nodos.append(tonode(t))
            except: nodos.append(None)
            pass

        return {
            "Tag":str(nodo.nodeName),
            "child":nodos,
            "attributes":salida
        }
        

    except:
        
        return {
            "Tag":"Text",
            "data": nodo.data
        }
