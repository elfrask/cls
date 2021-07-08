import clsengine as cls
import sys
import os
import json
import pickle
import shutil

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

    open("./cache/log.json", "w").write(
        json.dumps(
            {
                "data":estructurado,
                "func":f
            }
        )
    )
    input("test run...")

    pass



if __name__ == "__main__":
    main()