from ... cimport _lib as lib
from .. cimport tokens
from .. cimport tokens_reserve
from ...workspace cimport cls_script
from .. cimport cls_compiler

cdef dict _toks = tokens_reserve._toks


cdef list _tokenizer(cls_compiler.ClsCompiler self, cls_script.ClsScript _script):
    cdef list[list] output = [] 
    cdef list[tokens.tokenTemplate] line = []

    cdef str code = _script._code

    code = code.replace("\t", " ")
    code = code.replace("\r", " ")

    cdef str string = ""
    cdef int iterator = -1
    cdef str modo = "normal"

    cdef str string_format = ""
    cdef str string_operator = ""




    for character in code:
        iterator += 1

        if modo == "normal":

            if not character in [" ", "\n"]:

                if character in _toks["sim"]:
                    if string:
                        line.append(
                            lib.autoToken(string, iterator)
                        )
                        string = ""
                    
                    line.append(
                        tokens.SymbolToken(character, iterator)
                    )
                
                    pass
                elif character in _toks["ope"]:

                    if string:
                        line.append(
                            lib.autoToken(string, iterator)
                        )
                        string = ""
                    
                    if line:

                        if isinstance(line[len(line) - 1], tokens.OperatorToken):
                            if (iterator - line[len(line) - 1].index) == 1:
                                if f"{line[len(line) - 1]._operator}{character}" in _toks["multi-ope"]:

                                    before_token = line.pop()

                                    if f"{before_token._operator}{character}" == "//":

                                        modo = "comment"
                                        continue

                                    line.append(
                                        tokens.OperatorToken(
                                            f"{before_token._operator}{character}", before_token.index
                                        )
                                    )


                                    continue

                                pass

                            pass
                        

                        pass
                    
                    line.append(
                        tokens.OperatorToken(character, iterator)
                    )
                    



                    pass
                elif character in ["'", '"']:
                    modo = "string"
                    string_format = string
                    string_operator = character

                    string = ""
                    pass
                elif character == "#":
                    if string:
                        line.append(
                            lib.autoToken(string, iterator)
                        )

                        string = ""

                        pass
                    
                    modo = "comment"
                    
                    pass
                elif character == ";":
                    if string:
                        line.append(
                            lib.autoToken(string, iterator)
                        )

                        string = ""

                        pass
                    
                    if line:

                        output.append(line)
                        line = []
                    
                    pass
                else:
                    # print(character)
                    string += character
                    pass

                pass
            else:
                if string:
                    line.append(
                        lib.autoToken(string, iterator)
                    )

                    string = ""

                    pass


            pass
        elif modo == "string":

            if character != string_operator:

                string += character
            else:

                modo = "normal"

                line.append(
                    tokens.StringToken(
                        string,
                        string_operator,
                        string_format,
                        iterator - len(string_format) - len(string) - 2
                    )
                )

                string = ""

                pass


            pass
        elif modo == "comment":

            if character == "\n":

                modo = "normal"
            pass
        pass
    
    if string:
        line.append(
            lib.autoToken(string, iterator)
        )
        # string = ""
        pass
    
    if line:

        output.append(line)

        pass
    

    return output


