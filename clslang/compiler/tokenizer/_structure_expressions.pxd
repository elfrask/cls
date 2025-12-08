from .. cimport cls_compiler
from .. cimport tokens


cdef list[tokens.tokenTemplate] _structureExpression(
    cls_compiler.ClsCompiler self, 
    list[tokens.tokenTemplate] ExpressionCode, 
    list[tokens.FunctionToken] Environment, 
    str Param_mode
  )