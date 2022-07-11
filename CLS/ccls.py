import clsengine.clsengine as cls
import sys
import os
import json
#import dill
#import shutil
import platform

def cwd():
    app_path = ""

    if getattr(sys, 'frozen', False):
        app_path = os.path.dirname(sys.executable)
    elif __file__:
        app_path = os.path.dirname(__file__)

    return app_path

path = cwd()



cls.lib_path += [
    ".",
    "",
    path + "/std/std",
    path + "/lib",
    path + "/template",
    path + "/libpy",
    path + "/dlls",
    os.getcwd(),
    
]

pre_import = """
import copy as _c
__modcls['mod'] = _c.copy(globals())
"""


if not os.path.isdir("cache"):
    os.makedirs("cache")
    pass



def main(file, app:cls.appcls = cls.appcls(0)):
    #app:cls.appcls = cls.appcls(0)
    aplicacion = open(file, "r").read()
    crude:list =app.desline(aplicacion, file)
    parseado:list = app.parselex(crude)
    f:list = []
    estructurado:list = app.estructuration(parseado, f)
    generado:dict = app.generator({
                "data":estructurado,
                "func":f
    }, "normal")
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

def execute(code, app, name = "<CLS:Stdin>"):
    #app:cls.appcls = cls.appcls(0)
    #aplicacion = open(sys.argv[1], "r").read()
    crude:list =app.desline(code, name)
    parseado:list = app.parselex(crude)
    f:list = []
    estructurado:list = app.estructuration(parseado, f)
    generado:dict = app.generator({
                "data":estructurado,
                "func":f
    }, "normal", "console")
    listo:str = app.jump(generado, 0)


    app.exec(listo)
    #input("test run...")

    pass

def ayuda():

    print("""
    ccls
        -ejecuta la consola de CLS

    ccls [file] [args*]
        -ejecutar script [file].scls
    
    ccls //debug [file] [args*]
        -ejecutar script [file].scls y depurar
    
    ccls //process [mode] [file] [args*]
        -transpilar o procesar el script [file].scls
    
""")

    pass

if __name__ == "__main__":
    if len(sys.argv)>1:
        if sys.argv[1] in ["-h", "/h", "-H", "/H", "help", "--help", "-help"]:
            ayuda()
            exit(0)
        if len(sys.argv)>2:
            if sys.argv[1] == "//debug":

                la_app = cls.appcls(0)
                #main(sys.argv[2], la_app)
                a = open(sys.argv[2], "r").read()
                execute(a, la_app, sys.argv[2])


                cont = 0
                addcode = ""
                o_s = platform.system()
                print("")
                print(f"Cls 1.0.0 - Build for {o_s} platforms, CLS 2016-2021")
                print("Vinestar Studio 2021 (C) Todos los derechos recervados")
                print("")
                print("has entrado al modo debug")
                while True:
                    try:
                        if cont==0: 
                            cmd = input("debug > ")
                        else: 
                            cmd = input("debug · " + ("  "*cont))
                    except:
                        print()
                        exit(0)
                        pass
                    addcode+=cmd+cls.N

                    cont += cmd.count("{") + cmd.count("(") + cmd.count("[")
                    cont += -cmd.count("}") - cmd.count("]") - cmd.count(")")
                    #print(cont)
                    if cmd == "":
                        cmd = ""
                        cont = 0
                    if cont==0:
                        try:
                            #print()
                            execute(addcode, la_app, "<CLS:Stdin-Debug>")
                            #print()
                        except Exception as e:
                            print(e)
                            pass
                        addcode = ""
                    pass
            if sys.argv[1] == "//process":

                modcls = {"mod":{}}
                
                file_mod_cls = open(f"{path}/transpiled/{sys.argv[2]}.py", "r")

                data_mod = file_mod_cls.read()
                exec(
                    data_mod +
                    pre_import + 
                    "", 
                    {"__modcls":modcls})

                variables = modcls["mod"]

                Mod:cls.appcls = variables.get("ModCLs", cls.appcls)

                main(sys.argv[3], Mod(0))


                pass
            else:
                main(sys.argv[1])
        else:
            main(sys.argv[1])
            
            pass
    else:
        cont = 0
        la_app = cls.appcls(0)
        addcode = ""
        o_s = platform.system()
        print(f"Cls 1.0.0 - Build for {o_s} platforms, CLS 2016-2021")
        print("Vinestar Studio 2021 (C) Todos los derechos recervados")
        while True:
            
            try:
                if cont==0: 
                    cmd = input("> ")
                else: 
                    cmd = input("· " + ("  "*cont))
            except:
                print()
                exit(0)
            addcode+=cmd+cls.N

            cont += cmd.count("{") + cmd.count("(") + cmd.count("[")
            cont += -cmd.count("}") - cmd.count("]") - cmd.count(")")
            #print(cont)
            if cmd == "":
                cmd = ""
                cont = 0
            if cont==0:
                #print()
                execute(addcode, la_app)
                #print()
                try:
                    pass
                except Exception as e:
                    #print("fallo")
                    print(e)
                    pass
                addcode = ""
            pass