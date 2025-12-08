from .. cimport tokens
from .. cimport cls_compiler


cdef list[tokens.DeclareToken] _parsing_args(
    cls_compiler.ClsCompiler self, 
    list[tokens.tokenTemplate] declaresCode, 
    list[tokens.FunctionToken] Environment
)