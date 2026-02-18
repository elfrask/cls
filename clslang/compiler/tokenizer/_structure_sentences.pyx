from ...workspace cimport cls_block
from ...libs cimport _lib as spfunction
from .. cimport tokens
from ...workspace cimport cls_block
from .. cimport tokens_reserve
from .. cimport cls_compiler

cdef dict _nombre_reservados = tokens_reserve._nombre_reservados
cdef subjectFind = spfunction.subjectFind

cdef cls_block.ClsBlock _structureSentence(
    cls_compiler.ClsCompiler self, 
    list[list[tokens.tokenTemplate]] SentenceCode, 
    list[tokens.FunctionToken] Environment = [], 
    str mode = "normal"
  ):
        
    cdef list[tokens.tokenTemplate] blockSentence = []

    cdef list[tokens.tokenTemplate] sentence = []

    cdef list[tokens.FunctionToken] ContextFunctionEnvironment = []
    cdef str scope = "unknown"
    cdef _asyncFunction = False
    cdef onlyFunction = False
    cdef onlyDeclaration = False
    cdef cls_block.ClsBlock BlockCode 
    cdef list[any] Params = []
    cdef int _indexUse = -1

    # cdef tokens.tokenTemplate currect_sentence

    


    for i in SentenceCode:

        sentence = [*i]
        
        scope = "unknown"
        _asyncFunction = False
        onlyFunction = False
        onlyDeclaration = False
        ContextFunctionEnvironment = []
        _indexUse = -1
        Params = []

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

                    BlockCode = self._structureSentence(sentence[3].ContentComplex, ContextFunctionEnvironment, "function")

                    blockSentence.append(
                        tokens.FunctionToken(
                            sentence[0].index,
                            BlockCode.ByteCodeScript,
                            sentence[1].Value,
                            scope,
                            self._parsing_args(sentence[2].content),
                            [], # sin tipo de retorno
                            BlockCode.EnvironmentFunctions,
                            False,
                            _asyncFunction,
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
                            

                            BlockCode = self._structureSentence(sentence.pop().ContentComplex, ContextFunctionEnvironment, "function")

                            blockSentence.append(
                                tokens.FunctionToken(
                                    sentence[0].index,
                                    BlockCode.ByteCodeScript,
                                    sentence[1].Value,
                                    scope,
                                    self._parsing_args(sentence[2].content),
                                    sentence[4:],
                                    BlockCode.EnvironmentFunctions,
                                    False,
                                    _asyncFunction,

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
            elif sentence[0].Value == "with":
                
                if spfunction.compare(sentence, [
                    subjectFind(tokens.NameValue), 
                    subjectFind(tokens.NameValue),  
                    subjectFind(tokens.NameValue, {"Value": "in"}), 
                    subjectFind(tokens.NodeToken, {"format": "()"}), 
                    subjectFind(tokens.NodeToken, {"format": "{}"})
                ]):
                    blockSentence.append(
                        tokens.WithToken(
                            sentence[0].index,
                            sentence[1].Value,
                            self._structureExpression(sentence[3].content, Environment, mode),
                            self._structureSentence(sentence[4].ContentComplex, Environment, mode).ByteCodeScript,
                        )
                    )
                    pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para with")
                pass
            elif sentence[0].Value == "switch":

                if spfunction.compare(sentence, [
                    subjectFind(tokens.NameValue),
                    subjectFind(tokens.NodeToken, {"format": "()"}),
                    subjectFind(tokens.NodeToken, {"format": "{}"}),
                    ]):
                        blockSentence.append(
                            tokens.SwitchToken(
                                sentence[0].index,
                                self._structureExpression(sentence[1].content, Environment, mode),
                                self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript
                            )
                        )
                        pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para switch")

                pass
            elif sentence[0].Value == "case":

                if spfunction.compare(sentence, [ # case ("[Value]") {...}
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NodeToken, {"format": "()"}),
                        subjectFind(tokens.NodeToken, {"format": "{}"}),
                    ]):
                        blockSentence.append(
                            tokens.CaseToken(
                                sentence[0].index,
                                self._structureExpression(sentence[1].content, Environment, mode),
                                self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript,
                                True
                            )
                        )
                        pass
                elif spfunction.compare(sentence, [ # case default {...}
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NameValue, {"Value": "default"}),
                        subjectFind(tokens.NodeToken, {"format": "{}"}),
                    ]):
                    blockSentence.append(
                        tokens.CaseToken(
                            sentence[0].index,
                            [],
                            self._structureSentence(sentence[2].ContentComplex, Environment, mode).ByteCodeScript,
                            False
                        )
                    )
                    pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para switch")

                pass
            elif sentence[0].Value == "try":

                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NodeToken, {"format": "{}"}),
                        subjectFind(tokens.NameValue, {"Value": "catch"}),
                        subjectFind(tokens.NodeToken, {"format": "()"}),
                        subjectFind(tokens.NodeToken, {"format": "{}"}),
                    ]):

                    
                    if spfunction.compare(sentence[5:], [
                            subjectFind(tokens.NameValue, {"Value": "finally"}),
                            subjectFind(tokens.NodeToken, {"format": "{}"}),
                        ]):
                        Params = self._structureSentence(sentence[6].ContentComplex, Environment, mode).ByteCodeScript
                        pass

                    blockSentence.append(
                        tokens.TryToken(
                            sentence[0].index,
                            self._structureSentence(sentence[1].ContentComplex, Environment, mode).ByteCodeScript,
                            self._parsing_args(sentence[3].content, Environment),
                            self._structureSentence(sentence[4].ContentComplex, Environment, mode).ByteCodeScript,
                            Params
                        )
                    )
                    pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para try")

                pass
            elif sentence[0].Value in ["structure", "interface"]:

                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NodeToken, {"format": "()"}),
                        subjectFind(tokens.NodeToken, {"format": "{}"}),
                    ]):
                        blockSentence.append(
                            tokens.StructureToken(
                                sentence[0].index,
                                sentence[1].Value,
                                self._parsing_args(sentence[3].content, Environment),
                                self._structureExpression(sentence[2].content, Environment, mode),
                                sentence[0].Value == "interface"
                            )
                        )
                        pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para esta estructura/interface")

                pass
            elif sentence[0].Value == "class":

                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NodeToken, {"format": "()"}),
                        subjectFind(tokens.NameValue, {"format": "{}"}),
                    ]):
                        blockSentence.append(
                            tokens.ClassToken(
                                sentence[0].index,
                                sentence[1].Value,
                                self._structureExpression(sentence[2].content, Environment, "class"),
                                self._structureSentence(sentence[3].ContentComplex, Environment, "class").ByteCodeScript,
                                
                            )
                        )
                        pass
                
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para class")
            elif sentence[0].Value == "module":

                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.NameValue, {"format": "{}"}),
                    ]):
                        blockSentence.append(
                            tokens.ModuleToken(
                                sentence[0].index,
                                sentence[1].Value,
                                self._structureSentence(sentence[3].ContentComplex, Environment, "module").ByteCodeScript,
                            )
                        )
                        pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para module")


                pass
            elif sentence[0].Value == "import":

                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.StringToken),
                        subjectFind(tokens.NameValue, {"Value": "as"}),
                        subjectFind(tokens.NameValue),
                    ]):
                        blockSentence.append(
                            tokens.ImportToken(
                                sentence[0].index,
                                sentence[1].content,
                                sentence[3].Value
                            )
                        )
                        pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para import")


                pass
            elif sentence[0].Value == "from":

                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.StringToken),
                        subjectFind(tokens.NameValue, {"Value": "import"}),
                        subjectFind(tokens.NameValue),
                    ]):
                        blockSentence.append(
                            tokens.FromImportToken(
                                sentence[0].index,
                                spfunction.getListName(self, sentence[3:]),
                                sentence[1].content
                            )
                        )
                        pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para from/import")


                pass
            elif sentence[0].Value == "include":
                if spfunction.compare(sentence, [
                        subjectFind(tokens.NameValue),
                        subjectFind(tokens.StringToken),
                    ]):
                        blockSentence.append(
                            tokens.IncludeToken(
                                sentence[0].index,
                                sentence[1].content
                            )
                        )
                        pass
                else:
                    self.Catch(sentence[0].index, f"no se esperaba '{spfunction.token2SimpleString(sentence[1])}' para include")

                pass
            elif sentence[0].Value == "return":
                blockSentence.append(
                    tokens.ReturnToken(
                        sentence[0].index,
                        self._structureExpression(sentence[1:], Environment, mode)
                    )
                )
                pass
            elif sentence[0].Value in ["var", "const"]:
                blockSentence.append(
                    tokens.VarToken(
                        sentence[0].index,
                        self._parsing_args(sentence[1:], Environment),
                        sentence[0].Value == "const"
                    )
                )
                pass
            
            pass
        else: 

            _indexUse = spfunction.FindToken(subjectFind(tokens.OperatorToken, {"_operator": "="}), sentence)

            if _indexUse > -1:
                if _indexUse == 0:
                    self.Catch(sentence[0], f"No se esperaba '{spfunction.token2SimpleString(sentence[0])}'")
                    continue

                blockSentence.append(
                    tokens.AssignVarValue(
                        sentence[0].index,
                        self._structureExpression(sentence[0:_indexUse], Environment, mode),
                        self._structureExpression(sentence[_indexUse+1:], Environment, mode),
                    )
                )
                pass
            else:
                blockSentence.append(
                    tokens.ExpressionSentence(
                        sentence[0].index,
                        self._structureExpression(sentence, Environment, mode)
                    )
                )

            pass
        

        pass

    return cls_block.ClsBlock(blockSentence, Environment)
