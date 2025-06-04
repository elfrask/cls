from . import _tokens as tokens
# from . import _tokens as tokens

# from clslang cimport _tokens as tokens
# cimport clslang._tokens as tokens
from cpython cimport list as cy_list
# cimport numpy as np





# lib_path = []


_toks = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^", ":"],
    "multi-ope":["++", "--", "//", "**", "!=", "||", "==", "<<", ">>", "^^", "::", ":=", "<=", ">=", "->"],
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
    "thread":["sync", "async"],
    "nombre":[
            "func", "function", "class", "module", "with", "for", "if", "while", "define",
            "from", "import", "global", "try", "def", "fub", "method", "include", "using", "var",
            "template", "switch", "struct", "case", "return", "setrule"
        ],
    "codi":["or", "in", "and", "is"],
    "bucle":["break", "continue"]
}

# TokenTemplate = tokens.tokenTemplate

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

cdef class ClsBlock():

    cdef list[tokens.tokenTemplate] ByteCodeScript
    cdef list[tokens.FunctionToken] EnvironmentFunctions

    def __init__(self, list[tokens.tokenTemplate] _ByteCodeScript, list EnvironmentFunctions) -> None:
        
        self.ByteCodeScript = _ByteCodeScript
        self.EnvironmentFunctions = EnvironmentFunctions
        pass
    
    cpdef list[tokens.tokenTemplate] getCode(self):

        return self.ByteCodeScript
    cpdef list[tokens.tokenTemplate] getEnvironment(self):

        return self.EnvironmentFunctions
    

    pass


cdef class ClsScript():

    cdef public str _code
    cdef public str name_module
    cdef public int id
    cdef public ClsBlock result

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
    
    cdef void Catch(self, int i, str message = ""):

        print(f"Hubo un error en el index {i}: {message}")
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
    
    cdef list[tokens.DeclareToken] _parsing_args(self, list[tokens.tokenTemplate] declaresCode, list[tokens.FunctionToken] Environment = []):

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
                        continue
                    else:
                        self.Catch(i.index, f"no se esperaba '{spfunction.token2SimpleString(i)}' en una declaración")
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
                        self.Catch(i.index, f"no se esperaba '{spfunction.token2SimpleString(i)}' en una declaración")
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
    
    cdef list[tokens.tokenTemplate] _structureExpression(self, list[tokens.tokenTemplate] ExpressionCode, list[tokens.FunctionToken] Environment, Param_mode = "normal"):

        cdef list[tokens.tokenTemplate] blockExpressions = []

        cdef list[tokens.tokenTemplate] TypeFunction = []
        cdef list[tokens.FunctionToken] ContextFunctionEnvironment = []
        cdef list[tokens.DeclareToken] Params = []
        cdef int FunctionIndex = 0
        cdef str mode = "normal"

        cdef ClsBlock BlockCode
        
        for i in ExpressionCode:

            if mode == "normal":

                if isinstance(i, tokens.OperatorToken):
                    if i._operator == "->":
                        if isinstance(blockExpressions[-1], tokens.NodeToken):
                            if blockExpressions[-1].format == "()":
                                FunctionIndex = blockExpressions[-1].index
                                Params = self._parsing_args(blockExpressions.pop().content)
                                mode = "arrow_function"


                                continue
                if isinstance(i, tokens.NodeToken):

                    # if i.Multiline:
                    blockExpressions.append(
                        tokens.NodeToken(
                            i.format,
                            i.Multiline,
                            i.index
                        )
                    )

                    if isinstance(blockExpressions[-1], tokens.NodeToken): # Forzar tipado

                        blockExpressions[-1].content = self._structureExpression(i.content, Environment, Param_mode)

                        if len(i.ContentComplex) > 1:
                            blockExpressions[-1].ContentComplex = self._structureExpression(i.ContentComplex, Environment, Param_mode)
                        else:
                            blockExpressions[-1].ContentComplex = [blockExpressions[-1].content]

                    continue
                
                blockExpressions.append(i)
            elif mode == "arrow_function":

                if isinstance(i, tokens.NodeToken):
                    if i.format == "{}":

                        blockExpressions.append(
                            tokens.NameValue(
                                "_tmp_index" + str(len(Environment)),
                                FunctionIndex,
                                True
                            )
                        )

                        BlockCode = self._structureSentence(i.ContentComplex, ContextFunctionEnvironment)

                        Environment.append(
                            tokens.FunctionToken(
                                FunctionIndex,
                                BlockCode.ByteCodeScript,
                                "",
                                "Anonymous",
                                Params,
                                TypeFunction,
                                BlockCode.EnvironmentFunctions,
                                True
                                
                            )
                        )

                        Params = []
                        TypeFunction = []
                        ContextFunctionEnvironment = []
                        FunctionIndex = 0
                        mode = "normal"

                        continue
                    TypeFunction.append(i)
                
                if isinstance(i, tokens.NameValue):
                    TypeFunction.append(i)

                    continue
                if isinstance(i, tokens.OperatorToken):
                    if i._operator in ["<", ">", ">>"]:
                        TypeFunction.append(i)

                        if i._operator == ">>":
                            TypeFunction.pop()
                            TypeFunction.append(tokens.OperatorToken(">", i.index))
                            TypeFunction.append(tokens.OperatorToken(">", i.index+1))
                        continue
                    
                
                self.Catch(i.index, f"no se esperaba '{spfunction.token2SimpleString(i)}' en una declaración de tipos para una función flecha")
                

                pass

            pass


        return blockExpressions
    
    cdef ClsBlock _structureSentence(self, list[list[tokens.tokenTemplate]] SentenceCode, list[tokens.FunctionToken] Environment = [], mode = "normal"):
        
        cdef list[tokens.tokenTemplate] blockSentence = []

        cdef list[tokens.tokenTemplate] sentence = []

        cdef list[tokens.FunctionToken] ContextFunctionEnvironment = []
        cdef str scope = "unknown"
        cdef _asyncFunction = False
        cdef onlyFunction = False
        cdef onlyDeclaration = False
        cdef ClsBlock BlockCode 

        # cdef tokens.tokenTemplate currect_sentence

        


        for i in SentenceCode:

            sentence = [*i]
            
            scope = "unknown"
            _asyncFunction = False
            onlyFunction = False
            onlyDeclaration = False
            ContextFunctionEnvironment = []

            if isinstance(sentence[0], tokens.NameValue):


                    
                # if 

                if sentence[0].Value in _nombre_reservados["visible"]:
                    scope = sentence[0].Value
                    onlyDeclaration = True
                    sentence = sentence[1:]
                    pass


                if sentence[0].Value in _nombre_reservados["thread"]:
                    _asyncFunction = sentence[0].Value == "async"
                    onlyDeclaration = True
                    onlyFunction = True
                    sentence = sentence[1:]
                    pass

                if not sentence[0].Value in _nombre_reservados["nombre"]:
                    # print("llego")
                    
                    if spfunction.compare(sentence, [ # si tiene estructura de función C lo reestructura
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NodeToken, {"format": "()"}), 
                        subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        sentence = [
                            tokens.NameValue("function", sentence[0].index),
                            sentence[1],
                            sentence[2],
                            tokens.OperatorToken("->", 0),
                            sentence[0],
                            sentence[3]

                        ]

                        pass
                    pass

                if sentence[0].Value in ["function", "func", "fub", "method"]:
                    # print("llego a function", sentence)
                    # sentence = sentence[1:]

                    if spfunction.compare(sentence, [ # Funcion normal sin tipado de retorno
                        subjectFind(tokens.NameValue), 
                        subjectFind(tokens.NameValue), 
                        subjectFind(tokens.NodeToken, {"format": "()"}), 
                        subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        BlockCode = self._structureSentence(sentence[3].ContentComplex, ContextFunctionEnvironment)

                        blockSentence.append(
                            tokens.FunctionToken(
                                sentence[0].index,
                                BlockCode.ByteCodeScript,
                                sentence[1].Value,
                                scope,
                                self._parsing_args(sentence[2].content),
                                [], # sin tipo de retorno
                                BlockCode.EnvironmentFunctions,
                                False
                            )
                        )

                        ContextFunctionEnvironment = []


                        continue
                    elif spfunction.compare(sentence, [ # Funcion normal con tipado de retorno
                        subjectFind(tokens.NameValue), 
                        subjectFind(tokens.NameValue), 
                        subjectFind(tokens.NodeToken, {"format": "()"}), 
                        subjectFind(tokens.OperatorToken, {"_operator": "->"})
                        ]):

                        # print("funcion tipada")

                        if isinstance(sentence[-1], tokens.NodeToken):
                            if sentence[-1].format == "{}":
                                

                                BlockCode = self._structureSentence(sentence.pop().ContentComplex, ContextFunctionEnvironment)

                                blockSentence.append(
                                    tokens.FunctionToken(
                                        sentence[0].index,
                                        BlockCode.ByteCodeScript,
                                        sentence[1].Value,
                                        scope,
                                        self._parsing_args(sentence[2].content),
                                        sentence[4:],
                                        BlockCode.EnvironmentFunctions,
                                        False
                                    )
                                )

                                ContextFunctionEnvironment = []
                    else:
                        self.Catch(sentence[3].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[3])}' esta expresión para una función")
                        continue

                    pass
                elif sentence[0].Value == "if":

                    if spfunction.compare(sentence, [ # Funcion normal sin tipado de retorno
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NodeToken, {"format": "()"}), 
                            subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        blockSentence.append(
                            tokens.IfSequence(sentence[0].index, [
                                tokens.IfToken(
                                    sentence[0].index, 
                                    self._structureExpression(sentence[1].content, Environment, mode), 
                                    self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript
                                )
                            ])
                        )

                        if isinstance(blockSentence[-1], tokens.IfSequence): # Fuerza el tipado
                            
                            sentence = sentence[3:]
                            while sentence:
                                if spfunction.compare(sentence, [ # Caso elif/elseif (condition) { ... }
                                        subjectFind(tokens.NameValue), 
                                        subjectFind(tokens.NodeToken, {"format": "()"}), 
                                        subjectFind(tokens.NodeToken, {"format": "{}"})
                                    ]):

                                    if not sentence[0].Value in ["elif", "elseif"]:

                                        self.Catch(sentence[0].index, f"no se esperaba '{sentence[0].Value}' para una secuencia de if, elseif/else if, else")
                                        continue

                                    blockSentence[-1].add(
                                        tokens.IfToken(
                                            sentence[0].index, 
                                            self._structureExpression(sentence[1].content, Environment, mode), 
                                            self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript
                                        )
                                    )

                                    sentence = sentence[3:]

                                    continue
                                elif spfunction.compare(sentence, [ # Caso else if (condition) { ... }
                                        subjectFind(tokens.NameValue),
                                        subjectFind(tokens.NameValue),
                                        subjectFind(tokens.NodeToken, {"format": "()"}), 
                                        subjectFind(tokens.NodeToken, {"format": "{}"})
                                    ]):

                                    if sentence[0].Value != "else":

                                        self.Catch(sentence[0].index, f"no se esperaba '{sentence[0].Value}' para una secuencia de if, elseif/else if, else")
                                        continue
                                    if sentence[1].Value != "if":

                                        self.Catch(sentence[1].index, f"no se esperaba '{sentence[1].Value}' para una secuencia de if, elseif/else if, else")
                                        continue
                                    

                                    blockSentence[-1].add(
                                        tokens.IfToken(
                                            sentence[0].index, 
                                            self._structureExpression(sentence[2].content, Environment, mode), 
                                            self._structureSentence(sentence[3].ContentComplex, Environment, mode).ByteCodeScript
                                        )
                                    )

                                    sentence = sentence[4:]

                                    continue
                                elif spfunction.compare(sentence, [ # Caso else { ... }
                                        subjectFind(tokens.NameValue),
                                        subjectFind(tokens.NodeToken, {"format": "{}"})
                                    ]):

                                    if sentence[0].Value != "else":

                                        self.Catch(sentence[0].index, f"no se esperaba '{sentence[0].Value}' para una secuencia de if, elseif/else if, else")
                                        continue

                                    blockSentence[-1].add(
                                        tokens.IfToken(
                                            sentence[0].index, 
                                            [],
                                            self._structureSentence(sentence[1].ContentComplex, Environment, mode).ByteCodeScript,
                                            True
                                        )
                                    )

                                    break
                                else:
                                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para una secuencia de if, elseif/else if, else")
                                    pass
                                pass
                            pass

                        pass
                    else:
                        self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para una secuencia de if, elseif/else if, else")
                        pass

                    pass
                elif sentence[0].Value == "while":

                    if spfunction.compare(sentence, [ # While
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NodeToken, {"format": "()"}), 
                            subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        blockSentence.append(
                            tokens.WhileToken(
                                sentence[0].index, 
                                self._structureExpression(sentence[1].content, Environment, mode), 
                                self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript
                            )
                        )

                        pass
                    else:
                        self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para while")
                        pass

                    pass
                elif sentence[0].Value == "for":

                    if spfunction.compare(sentence, [ # Manual "For"
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NodeToken, {"format": "()"}), 
                            subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        if isinstance(sentence[1], tokens.NodeToken):
                            if len(sentence[1].ContentComplex) < 3:
                                self.Catch(sentence[1].index, f"error de sintaxis en la iteración manual")
                                continue
                        

                        blockSentence.append(
                            tokens.ForToken(
                                sentence[0].index,
                                self._parsing_args(sentence[1].ContentComplex[0], Environment),
                                self._structureExpression(sentence[1].ContentComplex[1], Environment, mode),
                                self._structureSentence(sentence[1].ContentComplex[2:], Environment, mode).ByteCodeScript,
                                self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript
                            )
                        )

                        pass
                    elif spfunction.compare(sentence, [ # Auto "For"
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NameValue, {"Value": "each"}), 
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NameValue, {"Value": "in"}), 
                            subjectFind(tokens.NodeToken, {"format": "()"}), 
                            subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        blockSentence.append(
                            tokens.ForEachToken(
                                sentence[0].index,
                                sentence[2].Value,
                                self._structureExpression(sentence[4].content, Environment, mode),
                                self._structureSentence(sentence[5].ContentComplex, Environment, mode).ByteCodeScript
                                
                            )
                        )

                        pass
                    elif spfunction.compare(sentence, [ # Auto "For" con index
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NameValue, {"Value": "each"}), 
                            subjectFind(tokens.NameValue),  
                            subjectFind(tokens.NameValue, {"Value": "and"}), 
                            subjectFind(tokens.NameValue), 
                            subjectFind(tokens.NameValue, {"Value": "in"}), 
                            subjectFind(tokens.NodeToken, {"format": "()"}), 
                            subjectFind(tokens.NodeToken, {"format": "{}"})
                        ]):

                        blockSentence.append(
                            tokens.ForEachToken(
                                sentence[0].index,
                                sentence[2].Value,
                                self._structureExpression(sentence[6].content, Environment, mode),
                                self._structureSentence(sentence[7].ContentComplex, Environment, mode).ByteCodeScript,
                                sentence[4].Value
                            )
                        )

                        pass
                    else:
                        self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para for")
                        pass

                    pass
                pass
                
                pass
            

            pass

        return ClsBlock(blockSentence, Environment)
    
    cpdef ClsScript Compile(self, ClsScript _script):

        cdef list[list[tokens.tokenTemplate]] tokenize = self._tokenizer(_script)
        cdef list[list[tokens.tokenTemplate]] tokenize_node_parsed = self._parsing(tokenize)

        cdef list[tokens.FunctionToken] EnvironmentFunction = []
        cdef ClsBlock block_content = self._structureSentence(tokenize_node_parsed, EnvironmentFunction)


        _script.result = block_content

        return _script
