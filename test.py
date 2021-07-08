

class l():
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
print(1|2)