class m:
    def __init__(self, v = 0) -> None:
        self.v = v
        pass
    def __setattr__(self, a, v):
        #print("atributo:", v)
        self.__dict__[a] = v
    def __getattr__(self, a):
        #print("atributo:", a)
        return None
    def melo(self):

        return "Hola Mundo"
    def __type__():
        return "ty"
    pass

k = m(12)
print(type("k") == str)