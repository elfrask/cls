"""class l():
    def __init__(self, o=0):
        self.o = o
    def __dict__(self): 
        pass
    def __getitem__(self, s):

        salida = None

        try:
            salida = eval("self."+str(s))
        except:pass

        return salida
    def __privado(self):
        print("funcion privada")
        pass
    pass

    a=l()   
    a.hola = "hola como estas?"
    print(a["hola"])
    print(1|2)"""
"""from copy import copy as c


    #exec("print(locals())", {}, {})
    globales = {}
    gl = c(globals())
    for i in gl:
        globales[i] = None
        pass

    def printf(*op):
        print("Echo: " + " ".join(op))
        pass

    exec(
        open("./py/test.py", "r").read(),
        globales,
        {
            "print":printf
        }
    )"""
"""class classcls:
        def __dict__():
            pass
        def __init__(self, obj:any) -> None:
            self.obj:any = obj
            pass
        def __call__(self, *args: any) -> any:
            class instance(self.obj):
                def __dic__(self):
                    pass
                def __getitem__(self, z):
                    salida = None
                    try:
                        salida = eval("self."+str(z))
                        pass
                    except:
                        pass

                    return salida
                def __setitem__(self, z, x):
                    try:
                        exec(f"self.{z} = tmp_i_m", {"tmp_i_m":x})
                        pass
                    except:
                        pass
                    pass
                pass

            return instance(*args)
        pass
    def toclass(obj:any) -> any:

        class instance(obj):
            real:any = obj
            def __dic__(self):
                pass
            def __getitem__(self, z):
                salida = None
                try:
                    salida = eval("self."+str(z))
                    pass
                except:
                    try:
                        salida = self.real(self)[z]
                    except:
                        pass
                    pass

                return salida
            def __setitem__(self, z, x):
                try:
                    exec(f"tmp_self.{z} = tmp_i_m", {"tmp_i_m":x, "tmp_self":self})
                    pass
                except:
                    pass
                pass
            pass
        
        return instance

    obj = toclass(str)
    obje = obj("Holas")
    obje["hola"] = "Hola mundo! :3" 
    print(obje["hola"])
    print(obje.hola)
    print(obje[0])"""

def main():
    d=2
    
    pass
main()
print(main.__code__.co_code)