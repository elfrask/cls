import shlex
import pickle
import sys
import os
import json



def get_value(v):
    if callable(v): return v()
    
    return v


def take_value(v, sets:dict) -> int:
    if callable(v): return v()
    elif isinstance(v, name):
        sal = take_value(sets.get(str(v), 0), sets)
        #print("llego", sal, type(sal), sets)
        return sal
    return v

def a2dict(e:list) -> dict:
    salida = {}
    for i in range(0, len(e)):
        salida[i] = e[i]
        pass
    return salida

class vm_casm:
    def __init__(self, code, memorysize=1024, arg=[]) -> None:
        self.memory = [None] * memorysize
        self.code = code
        self.todo = ""
        pass
    def cmd_set(self, arg, sets={}):

        sets[str(arg[0])] = take_value(arg[1], sets)
        pass
    def cmd_move(self, arg, sets={}):
        ara = a2dict(arg)
        self.memory[take_value(ara.get(1, 0), sets)] = take_value(ara.get(0, 0), sets)
        #print("move: ",arg, "; result: ", self.memory, sets)
        pass
    def cmd_add(self, arg, sets={}):
        ara = a2dict(arg)


        index = take_value(ara.get(1, 0), sets)
        value = take_value(ara.get(0, 0), sets)

        #print("add: ", arg)
        self.memory[index] += value
        pass
    def cmd_sub(self, arg, sets={}):
        ara = a2dict(arg)


        index = take_value(ara.get(1, 0), sets)
        value = take_value(ara.get(0, 0), sets)

        #print("add: ", arg)
        self.memory[index] -= value
        pass
    def cmd_mul(self, arg, sets={}):
        ara = a2dict(arg)


        index = take_value(ara.get(1, 0), sets)
        value = take_value(ara.get(0, 0), sets)

        #print("add: ", arg)
        self.memory[index] *= value
        pass
    def cmd_div(self, arg, sets={}):
        ara = a2dict(arg)


        index = take_value(ara.get(1, 0), sets)
        value = take_value(ara.get(0, 0), sets)

        #print("add: ", arg)
        self.memory[index] /= value
        pass
    def cmd_goto(self, arg, sets={}):
        ara = a2dict(arg)


        name_go = ara.get(0, "")

        if name_go == "": return 0

        sets["IS_TODO"] = 1
        self.todo = name_go

        pass
    def cmd_delete(self, arg, sets={}):
        ara = a2dict(arg)


        index = take_value(ara.get(0, 0), sets)

        #print("add: ", arg)
        self.memory[index] = None
        pass
    def cmd_print(self, arg, sets={}):
        out = []
        #print(arg)
        for i in arg:
            if isinstance(i, str) or isinstance(i, int):
                out.append(str(i))
            elif isinstance(i, name):
                out.append(
                    str(take_value(i, sets))
                )
                pass
            pass
        sys.stdout.write("".join(out))
        pass

    def cmd_call(self, func, sets={}):
        if len(func) == 0: func= [name("MAIN")]
        
        sets = {
            "FREE":self.libre,
            "MAX":self.MaxMemory,
            "THREAD":0,
            "NOT_THREAD":0,
            "IS_TODO":0
        }

        fun = self.code.get(str(func[0]), [])
        
        def to_arg(args):
            salida = []
            for i in args:
                if isinstance(i, int):
                    salida.append(i)
                elif isinstance(i, str):
                    if i[0:5] == "name:":
                        salida.append(
                            name(i[5:])
                        )
                    else:
                        salida.append(i)
                    pass
                elif isinstance(i, dict):
                    _exp = i["exp"]
                    _name = i["name"]

                    residuo = eval(f"sel.ref_{_name}(args, setis)", {
                        "sel":self,
                        "args":to_arg(_exp),
                        "setis":sets
                    })

                    salida.append(
                        residuo
                    )
                    pass
                
                pass
            return salida
        
        while(len(fun) > sets["THREAD"]):
            sets["NOT_THREAD"] = 0
            residuo = 0

            exe = fun[sets["THREAD"]]
            if (ins[exe[1]] in ["return", "ret"]):
                break

            if sets["IS_TODO"]:
                #print(self.todo, exe)
                if (ins[exe[1]] in ["to"]):
                    ara = a2dict(to_arg(exe[2]))
                    block = str(ara.get(0, 1))
                    #print(self.todo, block, str(self.todo) == str(block))
                    if (str(block) == str(self.todo)):
                        sets["IS_TODO"] = 0
                        #print("roto")
                    pass
                pass
            else:
                residuo = eval(f"sel.cmd_{ins[exe[1]]}(args, setis)", {
                    "sel":self,
                    "args":to_arg(exe[2]),
                    "setis":sets
                })

            if not sets["NOT_THREAD"]:
                sets["THREAD"]+=1
            else:
                #print(ins[exe[1]], exe)
                pass
            
            pass

        pass
    def cmd_if(self, arg, sets:dict={}):
        
        ara = a2dict(arg)        

        line_if = take_value(ara.get(1, 0), sets)
        line_else = take_value(ara.get(2, 1), sets)
        cond = take_value(ara.get(0, 0), sets)
        #print(line_if, line_else, cond)
        if (cond):
            #print("es if ._.", line_if)
            sets["THREAD"] += line_if
            pass
        else:
            #print("es else ._.", line_else)
            sets["THREAD"] += line_else
            pass
        sets["THREAD"] += 1
        sets["NOT_THREAD"] = 1
        #print("THREAD", sets["THREAD"])

        pass
    def call(self, func):
        self.cmd_call([name(func)])
        pass
    def libre(self) -> int:
        val = len(self.memory)-1
        try:
            while (self.memory[val] == None):
                val -= 1
        except IndexError as e:
            return 0
        return val+1
    def MaxMemory(self):
        return len(self.memory)
    def ref_var(self, arg, sets:dict={}):
        ara = a2dict(arg)
        return (    
            take_value(ara.get(0, 0), sets) + take_value(ara.get(1, 0), sets)
        )
    def ref_get(self, arg, sets:dict):
        ara = a2dict(arg)
        
        modo = str(ara.get(0, "int"))
        pos = take_value(ara.get(1, 0), sets)
        long = take_value(ara.get(2, 1), sets)
        #print(arg)
        if modo == "int":
            return a2dict(self.memory).get(pos, 0)
        elif modo in ["String", "str"]:
            codes = self.memory[pos:long]
            #print(codes)
            return bytes(codes).decode("utf8")
        pass
    def ref_ord(self, arg, sets:dict):
        return ord(a2dict(arg).get(0, "\x00"))
    def ref_eq(self, arg, sets:dict):
        ara = a2dict(arg)

        return int(
            take_value(ara.get(0, 0), sets) 
                == 
            take_value(ara.get(1, 0), sets) 

        )
    pass

class errores:
    syntax="syntax"
class name:
    def __init__(self, value) -> None:
        self.value = value
        pass
    def __str__(self) -> str:
        return self.value
    def __repr__(self) -> str:
        return f"name({self.value})"
    def __len__(self) -> int:
        return len(self.value)
        
    pass
ins = [
    "set",   "move",   "add",     "return", 
    "call",  "print",  "input",   "if",
    "goto",  "to",     "delete",  "sub", 
    "mul",   "div",
]

def trim(s:str):
    while(s[0:1] == " "): s=s[1:]
    while(s[-1:] == " "): s=s[0:-1]
    return s

def error(tipo, des=""):
    return{
        "tipo":tipo,
        "des":des
    }



def pf_exp(exp):
    salida = []
    cadena = ""
    count = 0
    def todata(value:str): # expreciones, enteros, cadenas, nombres
        #print("llego", value, ( str(value)[0] + str(value)[-1] ))
        if isinstance(value, dict):
            return value
        elif value.isnumeric():
            return int(value)
        elif (value[0] + value[-1]) == '""':
            return eval(value)
        else:
            if value[0:2] == "0x":
                return int(value, 16)
            else:
                #return name(value)
                return f"name:{value}"
        pass
    
    #print("llego 1", f"'{exp}'")
    for i in exp:
        if (i != ","):
            cadena += i
            count += str(i).count("(") - str(i).count(")")

            if count == -1:
                raise error(errores.syntax, "the token ')' is invalid in this case")
            pass
        else:
            if count == 0:
                cadena = trim(cadena)
                #print("llego")
                if cadena[-1:] == ")":
                    exp_ = cadena.split("(", 1)
                    e_ = exp_[1][0:-1]
                    #print(e_)
                    cadena = {
                        "name":exp_[0],
                        "exp":pf_exp(e_)
                    }
                    pass
                salida.append(todata(cadena))
                cadena = ""
            else:
                cadena+=i
            pass
        pass

    if cadena != "":
        cadena = trim(cadena)
        if cadena[-1:] == ")":
            exp_ = cadena.split("(", 1)
            e_ = exp_[1][0:-1]
            cadena = {
                "name":exp_[0],
                "exp":pf_exp(e_)
            }
            pass
        salida.append(todata(cadena))
        cadena = ""

    return salida

def main():
    argv = sys.argv

    if len(argv)>3:

        if argv[1] == "compile":
            app = compilar(open(argv[2], "r").read())
            file = pickle.dumps(app)
            open(argv[3], "wb").write(file)
            pass
        elif argv[1] == "tojson":
            app = compilar(open(argv[2], "r").read())
            file = json.dumps(app)
            open(argv[3], "w").write(file)
            pass
        elif argv[1] == "exe":
            file = (open(argv[3], "rb").read())
            app = pickle.loads(file)
            run(app, int(argv[2]))
            pass
        elif argv[1] == "run":
            app = compilar(open(argv[3], "r").read())
            run(app, int(argv[2]))
            pass
        

        pass
    else:
        print("casm compile [in-file].casm [out-file].cobj")
        print("casm exe     [size memory]  [in-file].cobj")
        print("casm run     [size memory]  [in-file].casm")
        
        pass
    pass

def stop(code:str, line:int, tipo:str, des=""):
    print(f"Errortype: {tipo} in the line:", line)
    print()
    print("- ",code.split("\n")[line-1])
    print()
    print(des)
    exit(0)
    pass

def compilar(code:str) -> list:

    incode = code.replace("\t", "").split("\n")
    line = 0
    func = {}
    f_current = ""
    #print(incode)
    for i in incode:
        line+=1


        bit = trim(i).split("//")[0]
        if bit=="": 
            continue
        #print(f"'{bit}'")
        if f_current == "":
            if bit[-1:] == ":":
                f_current = bit[0:-1]
                func[f_current] = []
                pass     
            pass
        else:
            funci:list = func[f_current]
            linea = bit.split(" ", 1)
            cmd = linea[0]
            exp = ""
            #print(linea)
            if len(linea) == 2:
                exp = linea[1]
            
            if not cmd in ins:
                stop(code, line, "ErrorCommand", f"the command '{cmd}' not is valid")
            
            try: exp_end = pf_exp(exp)
            except Exception as e:
                stop(code, line, e["tipo"], e["des"])


            funci.append([line, ins.index(cmd), exp_end])
            
            if cmd == "return":
                f_current = ""
            
            pass
        pass
    #print(func)
    return func

def run(code, size):

    app = vm_casm(code, size)

    app.call("MAIN")

    pass


if __name__ == "__main__":
    main()