import os
import shutil
from libpy.stdpyapi import *
import threading

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
        if not os.path.isdir(p):
            os.mkdir(p)
            pass
        pass
    def read(self):
        out = os.listdir(self.path)
        
        return Api.Array(out)
    def __len__(self):
        out = os.listdir(self.path)
        
        return Api.Integer(len(out))
    count = __len__
    def write(self, d:list=[]):
        
        for x in d:
            open(self.path + "/" + x, "w").close()
            pass
        pass
    
    pass




def isbyte(p):
    salida = False
    if not os.path.isfile(p):
        raise Exception('Of path not is file')

    try: open(p, "r").read()
    except: salida = True

    return Api.Boolean(salida)

def getcwd():
    return Api.String(os.getcwd())

def readA(path, callback=(), byte = "r"):
    
    if not callable(callback):
        raise Exception("the callback not is callable")
    
    def fileread():
        o = ""
        e = Async_Error()
        if os.path.isfile(path):
            try:
                f = open(path, byte)
                o = f.read()
                f.close()
            except UnicodeEncodeError as x:
                e.set_error(True)
                e.set_detailed(f"the file '{path}' can't read")
                e.set_code(1)
        else:
            e.set_error(True)
            e.set_detailed(f"the file '{path}' not found")
            e.set_code(2)
            if os.path.exists(path):
                e.set_detailed(f"the path '{path}' not is file")
                e.set_code(3)
                pass
            pass      
        
        callback(Api.String(o), e)
        pass

    a = threading.Thread(
        target=fileread,
    ).run()
    

    pass
def writeA(path, data="", callback=(), byte = "w"):
    
    if not callable(callback):
        raise Exception("the callback not is callable")
    
    def filewrite():
        o = ""
        e = Async_Error()

        try:
            f = open(path, byte)
            f.write(data)
            f.close()
        except UnicodeEncodeError as x:
            e.set_error(True)
            e.set_detailed(f"the file '{path}' can't write")
            e.set_code(1)
                
        
        callback(e)
        pass

    a = threading.Thread(
        target=filewrite,
    ).run()
    

    pass

def mkfile(path):
    open(path, "w").close()
    pass


export.open = OpenFile
export.dir = OpenDir

export.exist = Module()
export.Async = Module()

export.Async.read = (lambda *x: readA(byte="r", *x))
export.Async.readb = (lambda *x: readA(byte="rb", *x))
export.Async.write = (lambda *x: writeA(byte="w", *x))
export.Async.writeb = (lambda *x: writeA(byte="wb", *x))

export.exist.path = (lambda p: os.path.exists(p))
export.exist.file = (lambda p: os.path.isfile(p))
export.exist.dir = (lambda p: os.path.isdir(p))
export.exist.link = (lambda p: os.path.islink(p))
export.exist.mount = (lambda p: os.path.ismount(p))

def remover(p):
    if os.path.isfile(p):
        os.remove(p)
    elif os.path.isdir(p):
        os.removedirs(p)
    else:
        raise Exception(f"the path at remove '{p}' not is file or dir")
    pass

export.mkfile = mkfile
export.mkdir = os.mkdir
export.delete = remover
export.copy = shutil.copy
export.move = shutil.move