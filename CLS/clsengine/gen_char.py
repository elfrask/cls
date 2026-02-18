

class gen_char:
    def names(name:str, i:int, mod:bool=False) -> (#name
        dict["name":str, "i":int, "tipo":str, "notmod":bool]
        ):

        return {
            "tipo":"name",
            "name":name,
            "i": i,
            "notmod":mod
        }
    def val(value:str, type:str, i:int, lim:str="'", byte:str="") -> (#value
        dict["value":str, "tipo":str, "i":int, "l":str, "type":str, "byte":str]
        ):

        return {
            "value":value,
            "tipo":"value",
            "i":i,
            "l":lim,
            "type":type,
            "byte":byte
        }
    def ope(char:str, i:int, ope:bool =False) -> (#ope
        dict["char":str, "i":int, "ope":bool, "tipo":str]
        ):

        return {
            "char":char,
            "i":i,
            "ope":ope,
            "tipo":"ope"
        }
    def sim(char:str, i:int) -> (#sim
        dict["char":str, "i":int, "tipo":str]
        ):

        return {
            "char":char,
            "i":i,
            "tipo":"sim"
        }
    def cml(data, i) -> dict["tipo":str, "i":int, "data":str]:
        return {
            "tipo":"cml",
            "i":i,
            "data":data
        }
