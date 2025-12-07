from . import _tokens as tokens
from . import engine


cdef class subjectFind():

    cdef object tokenType
    cdef dict[str, str] params

    def __init__(self, type tokenType, dict[str, str] params = {}):

        self.tokenType = tokenType
        self.params = params
        
        pass
    cpdef bint checkEval(self, token: tokens.tokenTemplate):

        if isinstance(token, self.tokenType):

            # if not self.params:
            #     return True
            
            for i in self.params:
                
                if hasattr(token, i):
                    if getattr(token, i) != self.params[i]:
                        return False
                    
                else:
                    return False

            return True

        return False
    pass




cdef class spfunction():

    def autoToken(str string, i):

        _m_tk = "name"

        # if spfunction.es_decimal_cython(bytes(string, "utf8")):
        if spfunction.es_decimal_cython(string):
            _m_tk = "int"
            if string.count("."):
                _m_tk = "float"

        if _m_tk == "int":
            return tokens.NumberValue(string, False, i - len(string))
        elif _m_tk == "float":
            return tokens.NumberValue(string, True, i - len(string))
        else:
            return tokens.NameValue(string, i - len(string))

    # def es_decimal_cython(char* s):
    def es_decimal_cython(str s):
        cdef bint tiene_punto = False
        cdef bint tiene_digito = False
        
        
        
        # Recorre los caracteres
        for i in s:
            if i in ("0", "1", "2", "3", "4", "5", "6", "7", "8", "9"):
            # if (i <= B9) and (i >= B0):
                tiene_digito = True
            elif i == b'.' and not tiene_punto:
                tiene_punto = True
            else:
                # print(f"no es digito en '{s}' el: {i}")
                return False  # Carácter inválido
            
        
        return tiene_digito  # Asegura que haya al menos un dígito

    
    def compare(list[tokens.tokenTemplate] Expression, list[subjectFind] check):

        if len(check) > len(Expression):
            return False
        
        for i in range(0, len(check)):

            if not check[i].checkEval(Expression[i]):
                return False

        return True
    
    def token2SimpleString(token: tokens.tokenTemplate):

        if isinstance(token, tokens.NameValue):
            return token.Value
        elif isinstance(token, tokens.SymbolToken):
            return token.symbol
        elif isinstance(token, tokens.StringToken):
            return f"{token.format}{token.content}{token.format}"
        elif isinstance(token, tokens.NumberValue):
            return str(token.Value)
        elif isinstance(token, tokens.NodeToken):
            if len(token.ContentComplex) > 1:
                return  f"{token.format[0]} ({len(token.ContentComplex)}) Sentences {token.format[1]}"
            
            return f"{token.format[0]} ({len(token.content)}) Elements {token.format[1]}"
        elif isinstance(token, tokens.OperatorToken):
            return token._operator
        # elif isinstance(token, tokens.):
        #     return token.symbol
        

        return f"no reconocido ({token.index}) '{token.TypeToken}'"
    

    pass


cdef list[tokens.FromModuleToken] getListName(engine.ClsCompiler self, list[tokens.tokenTemplate] lista = []):

    cdef list[str] names = []
    cdef str mode = "name"

    cdef currentName = ""
    cdef newName = ""
    cdef index = 0

    for i in lista:

        if mode == "name":
            if isinstance(i, tokens.NameValue):

                currentName = i.Value
                mode = "separator"
                index = i.index
            else:
                self.Catch(i.index, f"No se esperaba '{spfunction.token2SimpleString(i)}'")
                pass
            pass
        elif mode == "rename":
            if isinstance(i, tokens.NameValue):
                newName = i.Value
                mode = "separator"
            else:
                self.Catch(i.index, f"No se esperaba '{spfunction.token2SimpleString(i)}'")
                pass
            pass
        elif mode == "separator":
            if isinstance(i, tokens.NameValue):

                if i.Value == "as": 
                    if newName == "":
                        mode = "rename"
                    else:
                        self.Catch(i.index, f"No se esperaba '{spfunction.token2SimpleString(i)}'")
                else:
                    self.Catch(i.index, f"No se esperaba '{spfunction.token2SimpleString(i)}'")
            elif isinstance(i, tokens.SymbolToken):

                if i.TypeToken == ",": 
                    names.append(
                        tokens.FromModuleToken(
                            index,
                            currentName,
                            newName
                        )
                    )
                    mode = "name"
                    currentName = ""
                    index = 0
                    newName = ""
                else:
                    self.Catch(i.index, f"No se esperaba '{spfunction.token2SimpleString(i)}'")
            else:
                self.Catch(i.index, f"No se esperaba '{spfunction.token2SimpleString(i)}'")
                pass

        pass
    
    if currentName:
        names.append(
            tokens.FromModuleToken(
                index,
                currentName,
                newName
            )
        )

    return names