import requests
import os
import shutil
import sys
import git
import json
import pickle

#pack = os.path.dirname(__file__)

N="""
"""

class etiquetas:
    def categoria(title="Titulo", cmd=[]) -> list["title":str, "cmds":list]:
        return {"title":title, "cmds":cmd}
    def cmd(cmd="", des=""):
        if isinstance(des, str): des= [des]
        return {"cmd":cmd, "des":des}

na = "cpkg"

ayuda = {
    "head":[
        'CLS - Frask - Vinestar Studio (C) (2024)',
        'CPKG el gestor de paquetes de CLS',
        '',
        'El gestor de paquetes de CPKG es un herramienta que te ayuda a administrar',
        'tus proyectos, compilar, adquirir librerías de CLS, publicar tus librerías',
        'de CLS, administrar tus dominios, configurar el compilador, actualizar tus',
        'librerías, administrar tu cuenta de desarrollador, entre otras cosas para',
        'darte facilidad a la hora de emplear CLS en tu trabajo',
        '',
        'puedes visitar: https://www.test-url.com/',
        'para obtener mas información en la pagina oficial'
    ],
    "lista":[
        etiquetas.categoria(
            "Administrar paquetes",
            [
                etiquetas.cmd(f"{na} install [paquete]", "instalar un paquete"),
                etiquetas.cmd(f"{na} uninstall [paquete]", "desinstalar un paquete"),
                etiquetas.cmd(f"{na} list", "listar paquetes instalados"),#
                etiquetas.cmd(f"{na} update [paquete]", "actualizar un paquete"),
                etiquetas.cmd(f"{na} update", "actualizar todos los paquetes"),
            ]
        ),
        etiquetas.categoria(
            "Administrar cuenta de desarrollador",
            [
                etiquetas.cmd(f"{na} user-login", "iniciar session"),#
                etiquetas.cmd(f"{na} user", "ver nombre de usuario"),#
                etiquetas.cmd(f"{na} user-logout", "cerrar session"),#
                etiquetas.cmd(f"{na} user-register", "registrar"),#
                etiquetas.cmd(f"{na} user-save", "recuperar cuenta"),#
                etiquetas.cmd(f"{na} user-repass", "cambiar contraseña"),#
            ]
        ),
        etiquetas.categoria(
            "Administrar tus paquetes publicos",
            [
                etiquetas.cmd(f"{na} is-domain [paquete]", "verifica si el dominio esta tomado"),#
                etiquetas.cmd(f"{na} get-domain [nuevo paquete]", "reclamar el dominio libre"),#
                etiquetas.cmd(f"{na} list-domain", "listar mis dominios"),#
                etiquetas.cmd(f"{na} del-domain [paquete]", "liberar un dominio"),#
                etiquetas.cmd(f"{na} set-domain [paquete] [repositorio]", "establecer repositorio en un dominio"),#
                etiquetas.cmd(f"{na} set-domain-info [paquete] [info]", "establecer descripción al dominio"),#
                etiquetas.cmd(f"{na} set-domain-page [paquete] [url]", "establecer pagina del dominio"),#
                etiquetas.cmd(f"{na} mark-update [paquete] [ver/1.0]", "marcar como actualizado y nombrar la version"),#
                etiquetas.cmd(f"{na} info-domain [paquete]", "mostrar información del dominio"),#
            ]
        ),
        etiquetas.categoria(
            "Administrar tu proyecto",
            [
                etiquetas.cmd(f"{na} gen-app", [
                    "genera un archivo app.json fundamental para administrar o compilar tu proyecto"
                ]),#
                etiquetas.cmd(f"{na} init [plantilla]", [
                    "crea un proyecto a partir de una plantilla seleccionada"
                ]),#
                etiquetas.cmd(f"{na} gen-template [nueva plantilla]", [
                    "crea una plantilla a partir del directorio actual"
                ]),#
                etiquetas.cmd(f"{na} save-template [nueva plantilla]", [
                    "crea una plantilla a partir del directorio actual",
                    "y guardarlo en la lista de plantillas"
                ]),#
                etiquetas.cmd(f"{na} use-template [nombre de la plantilla]", [
                    "hacer uso de una plantilla ya instalada"
                ]),#
                etiquetas.cmd(f"{na} install-template [path].json [nombre de la plantilla]", [
                    "crea una plantilla a partir del directorio actual",
                    "y guardarlo en la lista de plantillas"
                ]),#
                etiquetas.cmd(f"{na} plix [databuild]", "compila tu proyecto de CLS"),
                etiquetas.cmd(f"{na} conf-databuild [nuevo/editar databuild]", "configurar el databuild de un modo"),#
                etiquetas.cmd(f"{na} run [databuild]", "ejecución de argumentos rápidos asignados por databuild"),#
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

def gen_init(data={}, path="./"):

    if not os.path.exists(path):
        os.makedirs(path)
        pass

    for i in data:
        if isinstance(data[i], dict):
            gen_init(data[i], path + i + "/")
            pass
        else:
            t = open(path + i, "w")
            t.write(str(data[i]))
            t.close()
        pass

    pass

def pack_init(path="./"):
    
    salida = {}

    li = os.listdir(path)

    for i in li:
        if os.path.isdir(path + i):
            salida[i] = pack_init(path+i+"/")
        else:
            t = open(path+i, "r")
            salida[i] = t.read()
            t.close()
    

    return salida

def un_pack_init(datacode = {}, path="."):

    salida= {}
    
    for i in datacode:
        dd = datacode[i]
        pati = f"{path}/{i}"

        if isinstance(dd, dict):
            if not os.path.exists(pati):
                os.makedirs()
            un_pack_init(dd, pati)
        elif isinstance(dd, str):

            open(pati, "w").write(dd)
            
    

    return salida

err_synx = f"""
Error de sintaxis...

para ver la interfaz de ayuda
dar: {na} /h
"""



def main(argv = []):

    
    if (len(argv) == 1) or (tobj(argv).get(1, None) in ["-h", "/h", "-H", "/H", "help", "--help", "-help"]):
        
        _ayuda()

        pass
    else:
        #arg = tobj(argv)
        #print("llego")


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
            elif argv[1] == "init":
                if os.path.isfile(work+"/examples/"+argv[2]+".json"):

                    gen_init(
                        json.loads(
                            open(work+"/examples/"+argv[2]+".json", "r").read()
                        ),
                        "./"
                    )

                    print("se ha generado el proyecto exitosamente")
                    pass
                else:
                    print(f"la plantilla seleccionada '{argv[2]}' no existe")
                pass
            elif argv[1] == "gen-template":
                w = open(argv[2]+".json", "w")

                w.write(json.dumps(pack_init("./")))
                print("se ha generado la plantilla exitosamente")
                pass
            elif argv[1] == "save-template":
                w = open(work+"/project_templates/"+argv[2]+".json", "w")

                w.write(json.dumps(pack_init("./")))
                print("se ha generado la plantilla exitosamente")
                pass
            elif argv[1] == "use-template":
                w = open(work+"/project_templates/"+argv[2]+".json", "r").read()

                k = (json.dumps(w))

                un_pack_init(k, ".")

                print("hecho")
                pass
            elif argv[1] == "conf-databuild":
                if not os.path.isfile("app.json"):
                    print("debes de generar el archivo 'app.json'")
                    print("usa el siguiente comando para generarlo:")
                    print(f"    -{na} app-gen")
                    sys.exit(1)
                
                app = open("app.json", "r")

                data = json.loads(app.read())

                data["run"][argv[2]] = {
                    "x64":False,
                    "main":"main.scls",
                    "run":"ccls main.scls",
                    "platform":[sys.platform],
                    "output":"main.cle"
                }

                appout = open("app.json", "w")

                appout.write(
                    json.dumps(
                        data,
                        indent=4
                    )
                )

                print(f"se ha generado la configuracion '{argv[2]}' en el proyecto")


                pass
            elif argv[1] == "run":
                if not os.path.isfile("app.json"):
                    print("debes de generar el archivo 'app.json'")
                    print("usa el siguiente comando para generarlo:")
                    print(f"    -{na} app-gen")
                    sys.exit(1)
                
                app = open("app.json", "r").read()
                data = json.loads(app)
                
                if data.get("run", {}).get(argv[2], {}).get("run", False):
                    os.system(data["run"][argv[2]]["run"])
                
                pass
            else:
                print(err_synx)
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
                    "email":"Correo electrónico: ",
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
            elif argv[1] == "gen-app":
                if os.path.isfile("app.json"):
                    print("el proyecto ya tiene un archivo de configuracion 'app.json'")
                    sys.exit(1)
                
                
                app = open("app.json", "w")
                app.write(json.dumps({
                    "name":os.path.basename(os.getcwd()),
                    "ver":"1.0",
                    "mode":"CLS",
                    "depends":[],
                    "run":{}
                }, indent=4))
                print("se ha generado el archivo 'app.js' para administrar tu proyecto")
                print("ahora ejecuta el siguiente comando para configurar el databuild")
                print(f"    -{na} conf-databuild [nuevo/editar databuild]")
                
                pass
            else:
                print(err_synx)
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
                        print("se ha establecido la descripción exitosamente")
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
                        print("se ha marcado la actualización!")
                    else:
                        print("Hubo problemas para marcar una actualización")
                    pass
                else:
                    show_error()
                
                pass
            elif argv[1] == "install-template":
                w = open(work+"/project_templates/"+argv[3]+".json", "w")
                d = open(argv[2]+".json", "r").read()

                w.write(d)
                print("se ha generado la plantilla exitosamente")
                pass
            else:
                print(err_synx)
            
            
            pass
        if len(argv) > 3:
            pass

        pass
    pass

if __name__ == "__main__":
    main(sys.argv)