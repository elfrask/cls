import clsengine as cls
import sys
import os
import json
import pickle
import shutil
import platform

if not os.path.isdir("cache"):
    os.makedirs("cache")
    pass

def main():
    app:cls.appcls = cls.appcls(0)
    aplicacion = open(sys.argv[1], "r").read()
    crude:list =app.desline(aplicacion, sys.argv[1])
    parseado:list = app.parselex(crude)
    f:list = []
    estructurado:list = app.estructuration(parseado, f)
    generado:dict = app.generator({
                "data":estructurado,
                "func":f
    })
    listo:str = app.jump(generado, 0)

    open("./cache/log.json", "w").write(
        json.dumps(
            {
                "data":estructurado,
                "func":f
            }
        )
    )
    open("./cache/final.py", "w").write(
        listo
    )

    app.exec(listo)
    #input("test run...")

    pass

def execute(code, app):
    #app:cls.appcls = cls.appcls(0)
    #aplicacion = open(sys.argv[1], "r").read()
    crude:list =app.desline(code, "<CLS:stdin>")
    parseado:list = app.parselex(crude)
    f:list = []
    estructurado:list = app.estructuration(parseado, f)
    generado:dict = app.generator({
                "data":estructurado,
                "func":f
    })
    listo:str = app.jump(generado, 0)


    app.exec(listo)
    #input("test run...")

    pass



if __name__ == "__main__":
    if len(sys.argv)>1:
        main()
        try:
            pass
        except Exception as e:
            print(e)
            print("fallo")
        pass
    else:
        cont = 0
        la_app = cls.appcls(0)
        addcode = ""
        o_s = platform.system()
        print(f"Cls 1.0.0 - Build for {o_s} platforms, CLS 2016-2021")
        print("Vinestar Studio 2021 (C) Todos los derechos recervados")
        while True:
            if cont==0: cmd = input("> ")
            else: cmd = input("· " + ("  "*cont))
            addcode+=cmd+cls.N

            cont += cmd.count("{") + cmd.count("(") + cmd.count("[")
            cont += -cmd.count("}") - cmd.count("]") - cmd.count(")")
            #print(cont)
            cmd = ""
            cont = 0
            if cont==0:
                try:
                    execute(addcode, la_app)
                except Exception as e:
                    print(e)
                    pass
                addcode = ""
            pass