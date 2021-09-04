import requests
import os
import shutil
import sys
import git
import json
import pickle

pack = os.path.dirname(__file__)

N="""
"""

class etiquetas:
    def categoria(title="Titulo", cmd=[]) -> list["title":str, "cmds":list]:
        return {"title":title, "cmds":cmd}
    def cmd(cmd="", des=""):
        if isinstance(des, str): des= [des]
        return {"cmd":cmd, "des":des}


ayuda = {
    "head":[
        'CLS - Frask - Vinestar Studio (C) (2021)',
        'CPKG el gestor de paquetes de CLS',
        '',
        'El gestor de paquetes de CPKG es un herramienta que te ayuda a administrar',
        'tus proyectos, compilar, adquirir librerias de CLS, publicar tus librerias',
        'de CLS, administrar tus dominios, configurar el compilador, actualizar tus',
        'librerias, administrar tu cuenta de desarrollador, entre otras cosas para',
        'darte facilidad a la hora de emplear CLS en tu trabajo',
        '',
        'puedes visitar: https://www.test-url.com/',
        'para obtener mas informacion en la pagina oficial'
    ],
    "lista":[
        etiquetas.categoria(
            "Administrar paquetes",
            [
                etiquetas.cmd("cpkg install [paquete]", "instalar un paquete"),
                etiquetas.cmd("cpkg uninstall [paquete]", "desinstalar un paquete"),
                etiquetas.cmd("cpkg list", "listar paquetes instalados"),#
                etiquetas.cmd("cpkg update [paquete]", "actualizar un paquete"),
                etiquetas.cmd("cpkg update", "actualizar todos los paquetes"),
            ]
        ),
        etiquetas.categoria(
            "Administrar cuenta de desarrollador",
            [
                etiquetas.cmd("cpkg user-login", "iniciar session"),#
                etiquetas.cmd("cpkg user", "ver nombre de usuario"),#
                etiquetas.cmd("cpkg user-logout", "cerrar session"),#
                etiquetas.cmd("cpkg user-register", "registrar"),#
                etiquetas.cmd("cpkg user-save", "recuperar cuenta"),#
                etiquetas.cmd("cpkg user-repass", "cambiar contraseña"),#
            ]
        ),
        etiquetas.categoria(
            "Administrar tus paquetes publicos",
            [
                etiquetas.cmd("cpkg is-domain [paquete]", "verifica si el dominio esta tomado"),#
                etiquetas.cmd("cpkg get-domain [nuevo paquete]", "reclamar el dominio libre"),#
                etiquetas.cmd("cpkg list-domain", "listar mis dominios"),#
                etiquetas.cmd("cpkg del-domain [paquete]", "liberar un dominio"),#
                etiquetas.cmd("cpkg set-domain [paquete] [repositorio]", "establecer repositorio en un dominio"),#
                etiquetas.cmd("cpkg set-domain-info [paquete] [info]", "establecer descripcion al dominio"),#
                etiquetas.cmd("cpkg set-domain-page [paquete] [url]", "establecer pagina del dominio"),#
                etiquetas.cmd("cpkg mark-update [paquete] [ver/1.0]", "marcar como actualizado y nombrar la version"),
                etiquetas.cmd("cpkg info-domain [paquete]", "mostrar informacion del dominio"),
            ]
        ),
        etiquetas.categoria(
            "Administrar tu proyecto",
            [
                etiquetas.cmd("cpkg gen-app", [
                    "genera un archivo app.json fundamental para administrar o compilar tu proyecto"
                ]),
                etiquetas.cmd("cpkg init", [
                    "crea un proyecto a partir de una plantilla estandar"
                ]),
                etiquetas.cmd("cpkg init [plantilla]", [
                    "crea un proyecto a partir de una plantilla seleccionada"
                ]),
                etiquetas.cmd("cpkg plix [archivo inicial] [modo]", "compila tu proyecto de CLS"),
                etiquetas.cmd("cpkg mode [modo]", [
                    "establecer el modo de interpretar CLS",
                    "los modos que posee cls es:",
                    "",
                    "CLS: CLS comun y corriente",
                    "CLSS: CLS de tipado estatico",
                ]),
                etiquetas.cmd("cpkg conf-datapack [nuevo/editar datapack]", "configurar el datapack de un modo"),
                etiquetas.cmd("cpkg run [play]", "ejecucion de argumentos rapidos asignados por el nombre"),
                etiquetas.cmd("cpkg info", "dar toda la informacion del proyecto"),
            ]
        ),
        
    ]
}

def cwd():
    app_path = ""

    if getattr(sys, 'frozen', False):
        app_path = os.path.dirname(sys.executable)
    elif __file__:
        app_path = os.path.dirname(__file__)

    return app_path

work = cwd()
server = "http://localhost:9000/api/cpkg"

conf = {
    "user":"",
    "pass":"",
    "login":False
}

def save_conf(): 
    pickle.dump(conf, open(work +"/cpkg.conf", "wb"))

if not os.path.isdir(work +"/cpkg"):
    os.makedirs(work +"/cpkg")
    pass
cpkg_dirs = ["pkg", "data", "cache", "bin"]

for i in cpkg_dirs:
    if not os.path.isdir(work +"/cpkg/"+i):
        os.makedirs(work +"/cpkg/"+i)
    pass


if os.path.exists(work +"/cpkg.conf"):
    try:
        conf = pickle.load(open(work +"/cpkg.conf", "rb"))
    except:
        print("el archivos de configuraciones fue corrompido por lo que se ha arreglado") 
        save_conf()
    pass
else:
    save_conf()
    pass




def tobj(ob = []):
    itera = 0
    salida = {}
    for i in ob: 
        salida[itera] = i
        itera+=1
    return salida

def _ayuda():
    
    for i in ayuda["head"]:
        print("  "+i)
        pass
    for i in ayuda["lista"]:
        print(N*2)
        print(f'  +{i["title"]}')
        print()
        for x in i["cmds"]:
            print("    -"+x["cmd"])
            
            for y in x["des"]:
                print("        " + y)
                pass
            print()
            pass
        
        pass
    

    pass

def show_error(code=0):
    
    print("hubo un error al recibir una respuesta del servidor")
    pass


def main(argv = []):
    
    if (len(argv) == 1) or (tobj(argv).get(1, None) in ["-h", "/h", "-H", "/H", "help", "--help", "-help"]):
        
        _ayuda()

        pass
    else:
        #arg = tobj(argv)

        if len(argv) == 3:
            if argv[1] == "install":

                res = requests.post(server+ "/infopkg", {"name":argv[2]})

                if res.status_code == 200:
                    data = res.json()

                    if data.get("exist", False):
                        #descargar desde el git
                        pass
                    else:
                        print(f"el paquete '{argv[2]}' no a sido encontrado")

                    pass
                else:
                    show_error()

                pass
            elif argv[1] == "unistall":

                pass
            elif argv[1] == "update":

                pass
            elif argv[1] == "is-domain":
                res = requests.post(server+"/isdom", {"name":argv[2]})

                if res.status_code == 200:
                    yei = res.json()
                    if yei.get("free", False):
                        print(f"el dominio '{argv[2]}' esta tomado")
                        pass
                    else:
                        print(f"el dominio '{argv[2]}' esta libre")
                    pass
                else:
                    show_error()
                    pass
                pass
            elif argv[1] == "get-domain":

                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)

                res = requests.post(server+"/getdom", {
                    "name":argv[2],
                    "user":conf["user"],
                    "pass":conf["pass"],
                })

                if res.status_code == 200:
                    yei = res.json()
                    if yei.get("done", False):
                        print(f"el dominio '{argv[2]}' a sido tomado exitosamente")
                        pass
                    else:
                        print(f"el dominio '{argv[2]}' ya esta ocupado")
                    pass
                else:
                    show_error()
                    pass
                pass
            elif argv[1] == "del-domain":

                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)

                res = requests.post(server+"/deldom", {
                    "name":argv[2],
                    "user":conf["user"],
                    "pass":conf["pass"],
                })

                if res.status_code == 200:
                    yei = res.json()
                    if yei.get("done", False):
                        print(f"el dominio '{argv[2]}' a sido eliminado")
                        pass
                    else:
                        print(f"el dominio '{argv[2]}' no esta en tu lista de dominios")
                    pass
                else:
                    show_error()
                    pass
                pass
            elif argv[1] == "info-domain":
                
                res = requests.post(server + "/infodom", {"name":argv[2]})

                if res.status_code == 200:
                    yei = res.json()
                    if yei.get("done", False):

                        print(f"nombre: {yei['data']['name']}")
                        print(f"pagina: {yei['data']['page']}")
                        print(f"repositorio: {yei['data']['git']}")
                        print(f"autor: {yei['data']['user']}")
                        print()
                        print(f"version: {yei['data']['ver']}")
                        print(f"descripcion: {yei['data']['desc']}")
                        


                        pass
                    else:
                        print(f"el dominio '{argv[2]}' no existe")
                        pass
                    pass
                else:
                    show_error()
                pass

            pass
        if len(argv) == 2:
            if argv[1] == "list":
                packs = os.listdir(work+"/cpkg/pkg")
                
                print("Paquetes instalados:")
                
                if len(packs) == 0: print("Ups... esto esta vacio")
                
                for i in packs: 
                    print(i)
                
                pass
            elif argv[1] == "update":

                pass
            elif argv[1] == "user-login":

                data = {
                    "user":"Usuario: ",
                    "pass":"Contraseña: ",
                }

                for i in data:
                    data[i] = input(data[i])
                
                res= requests.post(server+"/isuser", data)

                if res.status_code == 200:
                    yeu = res.json()

                    if yeu.get("user", False):
                        if yeu.get("pass", False):
                            conf["user"] = data["user"]
                            conf["pass"] = data["pass"]
                            conf["login"] = True
                            save_conf()
                            print("Has iniciado session!")
                            pass
                        else:
                            print("Contraseña invalida")
                            pass
                        pass
                    else:
                        print("Usuario invalido")
                        pass
                    pass
                else:
                    show_error()
                    

                pass
            elif argv[1] == "user-logout":
                conf["user"] = ""
                conf["pass"] = ""
                conf["login"] = False
                save_conf()
                pass
            elif argv[1] == "user-register":
                data = {
                    "user":"Usuario: ",
                    "email":"Correo electronico: ",
                    "pass":"Contraseña: ",
                    "repass":"Contraseña otra vez: ",
                }

                for i in data:
                    msg =data[i]
                    data[i] =""
                    while data[i]=="":
                        data[i] = input(msg)
                        if data[i]=="":
                            print("debes de rellenar el formulario")
                
                

                if data["pass"] != data["repass"]:
                    print("Las contraseñas no coinciden")
                    sys.exit(1)
                
                res= requests.post(server+"/register", data)

                if res.status_code == 200:
                    yeu = res.json()

                    if yeu.get("reg", False):
                        print("El usuario a sido registrado exitosamente")
                    else:
                        print("El usuario esta tomado")
                        pass
                    pass
                else:
                    show_error()
                    

                pass
            elif argv[1] == "user-repass":
                data = {
                    "user":"Usuario: ",
                    "oldpass":"Vieja contraseña: ",
                    "pass":"Nueva contraseña: ",
                    "repass":"Nueva contraseña otra vez: ",
                }

                for i in data:
                    msg =data[i]
                    data[i] =""
                    while data[i]=="":
                        data[i] = input(msg)
                        if data[i]=="":
                            print("debes de rellenar el formulario")
                
                

                if data["pass"] != data["repass"]:
                    print("Las contraseñas no coinciden")
                    sys.exit(1)
                
                res= requests.post(server+"/repass", data)

                if res.status_code == 200:
                    yeu = res.json()

                    if yeu.get("repass", False):
                        print("La contraseña a sido cambiada exitosamente")
                    else:
                        if yeu.get("user", False):
                            print("Usuario invalid")
                            pass
                        else:
                            print("Contraseña invalida")
                        
                        pass
                    pass
                else:
                    show_error()
                    

                pass
            elif argv[1] == "user-save":
                data = {
                    "user":input("Usuario a recuperar: ")
                }

                res = requests.post(server + "/usersave", data)

                if res.status_code == 200:
                    yei = res.json()

                    if yei.get("save", False):
                        print(f"se ha enviado los datos al correo:", yei.get("email", "Sin Correo"))
                        pass
                    else:
                        print("El usuario no existe")
                        pass

                    
                    pass
                else:
                    show_error()
                pass
            elif argv[1] == "user":
                if conf["login"]:
                    print(conf["user"])
                else:
                    print("no has iniciado session")
                pass
            elif argv[1] == "list-domain":
                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)
                
                res = requests.post(server+ "/listdom", {"user":conf["user"]})

                if res.status_code == 200:
                    yei = res.json()
                    for i in yei["list"]:
                        print(i)
                    if len(yei["list"]) == 0:
                        print("no hay nada en tu lista de dominios")
                    pass
                else:
                    show_error()

                pass
            

            pass
        if len(argv) == 4:
            if argv[1] == "set-domain":
                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)
                
                res = requests.post(server + "/editdom", {
                    "name":argv[2],
                    "data":argv[3],
                    "user":conf["user"],
                    "pass":conf["pass"],
                    "edit":"git"
                })

                if res.status_code==200:
                    yei = res.json()
                    if yei.get("done", False):
                        print("se ha establecido el repositorio exitosamente")
                    else:
                        print("Hubo problemas para establecer el repositorio")
                    pass
                else:
                    show_error()
                
                pass
            elif argv[1] == "set-domain-info":
                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)
                
                res = requests.post(server + "/editdom", {
                    "name":argv[2],
                    "data":argv[3],
                    "user":conf["user"],
                    "pass":conf["pass"],
                    "edit":"desc"
                })

                if res.status_code==200:
                    yei = res.json()
                    if yei.get("done", False):
                        print("se ha establecido la descripcion exitosamente")
                    else:
                        print("Hubo problemas al escribir la descripcion")
                    pass
                else:
                    show_error()
                
                pass
            elif argv[1] == "set-domain-page":
                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)
                
                res = requests.post(server + "/editdom", {
                    "name":argv[2],
                    "data":argv[3],
                    "user":conf["user"],
                    "pass":conf["pass"],
                    "edit":"page"
                })

                if res.status_code==200:
                    yei = res.json()
                    if yei.get("done", False):
                        print("se ha establecido el link de la pagina")
                    else:
                        print("Hubo problemas establecer el link de la pagina")
                    pass
                else:
                    show_error()
                
                pass
            elif argv[1] == "mark-update":
                if not conf["login"]:
                    print("debes iniciar session para poder tomar un dominio")
                    sys.exit(1)
                
                res = requests.post(server + "/editdom", {
                    "name":argv[2],
                    "data":argv[3],
                    "user":conf["user"],
                    "pass":conf["pass"],
                    "edit":"ver"
                })

                if res.status_code==200:
                    yei = res.json()
                    if yei.get("done", False):
                        print("se ha marcado la actualizacion!")
                    else:
                        print("Hubo problemas para marcar una actualizacion")
                    pass
                else:
                    show_error()
                
                pass
            
            
            pass

        pass
    pass

if __name__ == "__main__":
    main(sys.argv)