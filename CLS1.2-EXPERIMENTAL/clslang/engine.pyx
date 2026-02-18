from . import _tokens as tokens

# from clslang cimport _tokens as tokens
# cimport clslang._tokens as tokens
from cpython cimport list as cy_list
# cimport numpy as np





# lib_path = []


_toks = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^", ":"],
    "multi-ope":["++", "--", "//", "**", "!=", "||", "==", "<<", ">>", "^^", "::", "<=", ">=", "->"],
    "sim":["{", "}", "(", ")", "[", "]", ","],
    "cond":["==", "<", ">", "!=", "<=", ">=", "!"],
    "open-close":{
        "{":"{}",
        "[":"[]",
        "(":"()",

    },
    "convert":{
        "condi":{"&":"and", "|":"or", "!":"not", "?":"in", "^":"**"},
        "expre-eval":{"++":"+1", "--":"-1"},
        "expre-exec":{"++":"+=1", "--":"-=1"},
    },
    "metodos":{"main":"__init__", "_call":"__call__", "_getitem":"__getitem__", "_setitem":"__setitem__",
        "_add":"__add__", "_sub":"__sub__", "_div":"__div__", "_delitem":"__delitem__", "_mul":"__mul__",
        "_mod":"__mod__", "_or":"__or__", "_and":"__and__", "_xor":"__xor__", "_or":"__or__", "_len":"__len__",
        "_repr":"__repr__", "_str":"__str__", "_int":"__int__", "_float":"__float__", "_array":"__list__", 
        "_dict":"__dict__"
    },
    "to_c":{"String":"str", "Array":"list", "Int":"int", "Float":"float", "Dictionary":"dict"}
}

# B9 = int(b"9")
# B0 = int(b"0")

_nombre_reservados = {
    "visible":["export", "static", "private", "public", "global"], 
    "nombre":[
            "func", "function", "class", "module", "with", "for", "if", "while", "define",
            "from", "import", "global", "try", "def", "fub", "method", "include", "using", "var",
            "template", "switch", "struct", "case", "return", "setrule"
        ],
    "codi":["or", "in", "and", "is"],
    "bucle":["break", "continue"]
}

# TokenTemplate = tokens.tokenTemplate

cdef class spfunction():

    def autoToken(str string, i):

        _m_tk = "name"

        # if spfunction.es_decimal_cython(bytes(string, "utf8")):
        if spfunction.es_decimal_cython(string):
            _m_tk = "int"
            if string.count("."):
                _m_tk = "float"

        if _m_tk == "int":
            return tokens.NumberValue(string, False, i)
        elif _m_tk == "float":
            return tokens.NumberValue(string, True, i)
        else:
            return tokens.NameValue(string, i)

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

    
    

    pass


cdef class StackParsingEviroment():

    cdef public list[list[tokens.tokenTemplate]] output
    cdef public list[tokens.tokenTemplate] line
    cdef public int stackLevel
    cdef public int index
    cdef public str close
    cdef public str format

    def __init__(self, str close = "", str format = "", int stackLevel = 0, int index = 0) -> None:

        self.line = []
        self.output = []
        self.stackLevel = stackLevel
        self.index = index
        self.close = close
        self.format = format

        pass
    
    def get_data_returning(self):

        return self.output
    
    cdef set_next_line(self):

        # print("nextline: ", self.line)

        if self.line:
            self.output.append(self.line)
            self.line = []

        pass

    
    

    pass
    



cdef class ClsScript():

    cdef public str _code
    cdef public str name_module
    cdef public int id
    cdef public list[list[tokens.tokenTemplate]] result

    def __init__(self, str code, str name_module, int ID = 0):

        self._code = code
        self.name_module = name_module
        self.id = ID

        pass
    

    


    pass

cdef class ClsApplication():

    cdef str cwd
    cdef int pid
    cdef public dict[ClsScript] AppModules
    _api_base = {

    }

    def __init__(self, str cwd, int pid):

        self.cwd = cwd
        self.pid = pid
        self.AppModules = {}

        pass
    pass

cdef class ClsCompiler():

    cdef ClsApplication ClsApp
    cdef public list[str] lib_path

    def __init__(self, list[str] lib_path, ClsApplication ClsApp):

        self.ClsApp = ClsApp
        self.lib_path = []
        pass
    
    
    cdef list _tokenizer(self, ClsScript _script):
        cdef list[list] output = [] 
        cdef list[tokens.tokenTemplate] line = []

        cdef str code = _script._code

        code = code.replace("\t", " ")
        #code = code.replace(N, " ")
        code = code.replace("\r", " ")

        cdef str string = ""
        cdef int iterator = -1
        cdef str modo = "normal"
        # cdef bint ope_active = False

        cdef str string_format = ""
        cdef str string_operator = ""


    
        # cdef tokens.OperatorToken before_token

        for character in code:
            iterator += 1

            if modo == "normal":

                if not character in [" ", "\n"]:

                    if character in _toks["sim"]:
                        if string:
                            line.append(
                                spfunction.autoToken(string, iterator)
                            )
                            string = ""
                        
                        line.append(
                            tokens.SymbolToken(character, iterator)
                        )
                    
                        pass
                    elif character in _toks["ope"]:

                        if string:
                            line.append(
                                spfunction.autoToken(string, iterator)
                            )
                            string = ""
                        
                        if line:

                            if isinstance(line[len(line) - 1], tokens.OperatorToken):
                                if (iterator - line[len(line) - 1].index) == 1:
                                    if f"{character}{line[len(line) - 1]._operator}" in _toks["multi-ope"]:

                                        before_token = line.pop()

                                        if f"{character}{before_token._operator}" == "//":

                                            modo = "comment"
                                            continue

                                        line.append(
                                            tokens.OperatorToken(
                                                f"{character}{before_token._operator}", before_token.index
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
                                spfunction.autoToken(string, iterator)
                            )

                            string = ""

                            pass
                        
                        modo = "comment"
                        
                        pass
                    elif character == ";":
                        if string:
                            line.append(
                                spfunction.autoToken(string, iterator)
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
                            spfunction.autoToken(string, iterator)
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
                spfunction.autoToken(string, iterator)
            )
            # string = ""
            pass
        
        if line:

            output.append(line)

            pass
        

        return output
    
    cdef list[list[tokens.tokenTemplate]] _parsing(self, list[list[tokens.tokenTemplate]] _byte_tokenize):


        cdef list[StackParsingEviroment] stack = []
        cdef StackParsingEviroment _current_level = StackParsingEviroment("", "", 0, 0)
        cdef StackParsingEviroment _before_current_level = StackParsingEviroment("", "", 0, 0)
        cdef NodeToken = None

        
        # stack.append(_current_level)
        
        cdef int index_stack = 0


        
        
        for x in _byte_tokenize:
            
            for y in x:

                if isinstance(y, tokens.SymbolToken):

                    if y.symbol in ["(", "[", "{"]:

                        stack.append(_current_level)

                        _current_level = StackParsingEviroment(
                            _toks["open-close"].get(y.symbol)[1],
                            _toks["open-close"].get(y.symbol), 
                            index_stack+1, 
                            y.index
                        )

                        index_stack += 1


                        pass
                    elif y.symbol == _current_level.close:

                        _current_level.set_next_line()

                        # print("Cerrando con:", y)

                        _before_current_level = stack.pop()

                        NodeToken = tokens.NodeToken(
                            _current_level.format, 
                            _current_level.format in ["{}"], 
                            _current_level.index
                        )

                        NodeToken._set_content(_current_level.get_data_returning())

                        _before_current_level.line.append(
                            NodeToken
                        )

                        _current_level = _before_current_level

                        index_stack -=1

                        

                        pass
                    else:
                        _current_level.line.append(
                            y
                        )


                    pass
                else:
                    _current_level.line.append(
                        y
                    )


                pass
            
            _current_level.set_next_line()


            pass
        
        _current_level.set_next_line()
        



        return _current_level.get_data_returning()
    
    
    
    cpdef ClsScript Compile(self, ClsScript _script):

        cdef list[list[tokens.tokenTemplate]] tokenize = self._tokenizer(_script)
        cdef list[list[tokens.tokenTemplate]] tokenize_node_parsed = self._parsing(tokenize)

        _script.result = tokenize_node_parsed

        return _script
