from .. cimport tokens
from ...workspace cimport cls_block
from ...libs cimport _lib as lib
from .. cimport cls_compiler

cdef list[tokens.tokenTemplate] _structureExpression(
    cls_compiler.ClsCompiler self, 
    list[tokens.tokenTemplate] ExpressionCode, 
    list[tokens.FunctionToken] Environment, 
    str Param_mode = "normal"
  ):

    cdef list[tokens.tokenTemplate] blockExpressions = []

    cdef list[tokens.tokenTemplate] TypeFunction = []
    cdef list[tokens.FunctionToken] ContextFunctionEnvironment = []
    cdef list[tokens.DeclareToken] Params = []
    cdef int FunctionIndex = 0
    cdef str mode = "normal"

    cdef cls_block.ClsBlock BlockCode
    
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
                
            
            self.Catch(i.index, f"no se esperaba '{lib.token2SimpleString(i)}' en una declaración de tipos para una función flecha")
            

            pass

        pass


    return blockExpressions

    