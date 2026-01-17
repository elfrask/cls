from ..workspace cimport cls_application
from ..workspace cimport cls_script
from ..workspace cimport cls_block
from ..workspace cimport stack_error
from .tokenizer cimport _tokenizer
from .tokenizer cimport parsing
from .tokenizer cimport _parsing_args
from .tokenizer cimport _structure_expressions
from .tokenizer cimport _structure_sentences
from . cimport tokens
import sys

cdef class ClsCompiler():

    # cdef cls_application.ClsApplication ClsApp
    # cdef public list[str] lib_path

    def __init__(self, list[str] lib_path, cls_application.ClsApplication ClsApp):

        self.ClsApp = ClsApp
        self.lib_path = []
        pass
    
    cpdef void Catch(self, int i, str message = ""):

        cdef str msg = f"Hubo un error en el index {i}: {message}"

        if "-debug" in sys.argv:
            raise
        else:
            self.ClsApp.StacksErrors.append(
                stack_error.StackError(i, self.script, message)
            )
            raise message
        # pass

    cdef list _tokenizer(self, cls_script.ClsScript _script):
        
        return _tokenizer._tokenizer(self, _script)
    
    cdef list[list[tokens.tokenTemplate]] _parsing(self, list[list[tokens.tokenTemplate]] _byte_tokenize):

        return parsing._parsing(self, _byte_tokenize)
    
    cdef list[tokens.DeclareToken] _parsing_args(self, list[tokens.tokenTemplate] declaresCode, list[tokens.FunctionToken] Environment = []):

        return _parsing_args._parsing_args(self, declaresCode, Environment)
    
    cdef list[tokens.tokenTemplate] _structureExpression(self, list[tokens.tokenTemplate] ExpressionCode, list[tokens.FunctionToken] Environment, str Param_mode = "normal"):

        return _structure_expressions._structureExpression(self, ExpressionCode, Environment, Param_mode)
    
    cdef cls_block.ClsBlock _structureSentence(self, list[list[tokens.tokenTemplate]] SentenceCode, list[tokens.FunctionToken] Environment = [], str mode = "normal"):
        
        return _structure_sentences._structureSentence(self, SentenceCode, Environment, mode)
    
    cpdef cls_script.ClsScript Compile(self, cls_script.ClsScript _script):

        cdef list[list[tokens.tokenTemplate]] tokenize = self._tokenizer(_script)
        cdef list[list[tokens.tokenTemplate]] tokenize_node_parsed = self._parsing(tokenize)

        cdef list[tokens.FunctionToken] EnvironmentFunction = []
        cdef cls_block.ClsBlock block_content = self._structureSentence(tokenize_node_parsed, EnvironmentFunction)


        _script.result = block_content

        return _script
