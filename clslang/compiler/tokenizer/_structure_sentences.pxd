from .. cimport tokens
from .. cimport cls_compiler
from ...workspace cimport cls_block

cdef cls_block.ClsBlock _structureSentence(
    cls_compiler.ClsCompiler self, 
    list[list[tokens.tokenTemplate]] SentenceCode, 
    list[tokens.FunctionToken] Environment, 
    str mode
  )