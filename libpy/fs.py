import os
class Openfile:
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



export.open = Openfile
export.exist = Module()
export.exist.path = (lambda p: os.path.exists(p))
export.exist.file = (lambda p: os.path.isfile(p))
export.exist.dir = (lambda p: os.path.isdir(p))
export.exist.link = (lambda p: os.path.islink(p))
export.exist.mount = (lambda p: os.path.ismount(p))
