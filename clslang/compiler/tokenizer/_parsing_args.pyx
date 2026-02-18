from ...libs cimport _lib as lib
from .. cimport tokens
from .. cimport cls_compiler


cdef list[tokens.DeclareToken] _parsing_args(
    cls_compiler.ClsCompiler self, 
    list[tokens.tokenTemplate] declaresCode, 
    list[tokens.FunctionToken] Environment = []
  ):

    cdef str mode = "name"
    cdef int index = 0
    cdef list[tokens.DeclareToken] output = []
    cdef str var_name = ""
    cdef list[tokens.tokenTemplate] var_type = []
    cdef list[tokens.tokenTemplate] var_default_value = []

    

    for i in declaresCode:

        if isinstance(i, tokens.SymbolToken):
            if i.symbol == ",":
                if not var_name:
                    self.Catch(i.index, "no se esperaba un ',' en esta ubicación")
                    continue
                
                output.append(
                    tokens.DeclareToken(
                        index, 
                        var_name,
                        var_type,
                        self._structureExpression(var_default_value, Environment)
                    )
                )

                mode = "name"
                index = 0
                var_name = ""
                var_type = []
                var_default_value = []
                continue
        
        if mode == "name":

            if not var_name:
                if isinstance(i, tokens.NameValue):
                    if i.Value.count("."):
                        self.Catch(i.index + i.Value.find("."), f"no se esperaba '.' en una declaración")
                        continue
                    
                    var_name = i.Value
                    index = i.index
                    continue
                else:
                    self.Catch(i.index, f"no se esperaba '{lib.token2SimpleString(i)}' en una declaración")
                    continue
            else:
                if isinstance(i, tokens.OperatorToken):

                    if i._operator == ":":

                        mode = "type"
                        continue
                    elif i._operator == "=":

                        mode = "default"
                        continue
                    elif i._operator == ":=":
                        var_type = [tokens.NameValue("auto", i.index)]
                        mode = "default"
                        continue
                    else:
                        self.Catch(i.index, f"no se esperaba '{i._operator}' en una declaración")
                        continue
                else:
                    self.Catch(i.index, f"no se esperaba '{lib.token2SimpleString(i)}' en una declaración")
                    continue
        elif mode == "type":

            if isinstance(i, tokens.OperatorToken):
                if i._operator == "=":
                    if not var_type:
                        var_type = [tokens.NameValue("auto", i.index)]
                    mode = "default"
                    # print("set: ", i)
                    continue
                elif i._operator in ["<", ">", ">>"]:
                    var_type.append(i)

                    if i._operator == ">>":
                        var_type.pop()
                        var_type.append(tokens.OperatorToken(">", i.index))
                        var_type.append(tokens.OperatorToken(">", i.index+1))
                    continue
                else:
                    self.Catch(i.index, f"no se esperaba '{i._operator}' en una declaración de tipos para '{var_name}'")
                    continue
            elif isinstance(i, tokens.SymbolToken):
                self.Catch(i.index, f"no se esperaba '{i.symbol}' en una declaración de tipos para '{var_name}'")
                continue
            else:
                var_type.append(i)
                continue
        elif mode == "default":

            var_default_value.append(i)
            pass
        
        pass

    if var_name:
        output.append(
            tokens.DeclareToken(
                index, 
                var_name,
                var_type,
                self._structureExpression(var_default_value, Environment)
            )
        )

    return output
