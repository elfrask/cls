import os
class OpenFile:
    def __init__(self, p) -> None:
        self.path = p
        if not os.path.isfile(p):
            open(p, "w").close()
            pass
        pass
    def read(self):
        file = open(self.path, "r")
        i = file.read()
        file.close()
        return Api.String(i)
    def write(self, d):
        file = open(self.path, "w")
        file.write(str(d))
        file.close()
        pass
    def readb(self):
        file = open(self.path, "rb")
        i = file.read()
        file.close()
        return Api.Bytes(i)
    def writeb(self, d):
        file = open(self.path, "wb")
        file.write(str(d))
        file.close()
        pass
    
    pass
class OpenDir:
    def __init__(self, p) -> None:
        self.path = p
        if not os.path.isfile(p):
            open(p, "w").close()
            pass
        pass
    def read(self):
        file = open(self.path, "r")
        i = file.read()
        file.close()
        return i
    def write(self):
        pass
    pass




export.file = OpenFile
export.file = OpenDir
export.exist = Module()
export.exist.path = (lambda p: os.path.exists(p))
export.exist.file = (lambda p: os.path.isfile(p))
export.exist.dir = (lambda p: os.path.isdir(p))
export.exist.link = (lambda p: os.path.islink(p))
export.exist.mount = (lambda p: os.path.ismount(p))
