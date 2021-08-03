def estructuration_one(self, code:list, func:list=[]) -> list:
        salida =[]

        #print("llego")
        def sub_group(data:list) -> list:
            salida_out=  []
            out = []
            if data["tipo"] in ["()", "[]"]:
                for i in data["complet"]:
                    salida_out.append(
                        self.estructuration_one(i, func)
                    )
                    pass

                if len(salida_out)>0:
                    out=salida_out[0]
                
                return {
                    "tipo":data["tipo"],
                    "complet":salida_out,
                    "data":out,
                    "i":data["i"]
                }
            elif data["tipo"] in ["code"]:
                
                for i in data["data"]:
                    salida_out.append(
                        self.estructuration_one(i, func)
                    )
                    pass

                if len(salida_out)>0:
                    out=salida_out[0]
                
                return {
                    "tipo":data["tipo"],
                    "data":salida_out,
                    "one":out,
                    "i":data["i"]
                }

                pass
        
        #e = 0
        #print("llamada")
        for i in code:#while(len(code)>e):
            #print("code", i)
            

            if i["tipo"] == "code":
                is_func:bool = False
                asyncrono:bool = False
                arg:list = []
                codigo:list = []
                devolver:str =self.tydef
                #print(salida[-2:])
                if len(code)== 2:
                    print("funcion")
                    if compara(["()", {"tipo":"ope", "char":"->"}], salida[-2:]):
                        codigo = i
                        salida.pop()
                        arg= salida.pop()
                        #e+=2
                        is_func=True
                        pass
                    else:
                        #salida.append(i)
                        continue
                    pass
                if len(code)== 3:
                    if compara(["()", {"tipo":"ope", "char":"->"}, "name"], salida[-3:]):
                        devolver = salida.pop()["name"]
                        salida.pop()
                        arg=salida.pop()
                        codigo = i
                        #e+=3
                        is_func=True

                        pass
                    else:
                        #salida.append(i)
                        continue
                    pass
                
                if is_func:
                    if len(salida)>0:
                        if salida[0]["tipo"]=="name":
                            if salida[0]["name"] in ["sync", "async"]:
                                asyncrono = salida[0]["name"] == "async"
                                salida.pop()
                                pass
                            pass
                        pass
                    funciones = []
                    #print("salida")
                    f_A = {
                        "tipo":"func-a",
                        "arg":self.argparse(arg["data"]),
                        "code":{
                            "data":self.estructuration(codigo["data"], funciones),
                            "func":funciones
                        },
                        "return":devolver,
                        #"func-a":funciones,
                        "async":asyncrono,
                        "name":"t_tmp" + str(len(func))
                    }

                    salida.append(gen_char.names("t_tmp"+str(len(func)), i["i"], True))
                    func.append(["t_tmp" + str(len(func)), f_A])
                    #salida["func"]+=1
                    pass
                else:
                    salida.append(
                        sub_group(i)
                    )
                    pass
                pass
            elif i["tipo"] in ["[]", "()"]:
                salida.append(
                    sub_group(i)
                )
                pass
            else:
                salida.append(i)
                pass

            #e=1+e; 
            #print("llego")
            
            pass

        return salida
    