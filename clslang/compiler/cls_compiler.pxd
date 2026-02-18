

from ..workspace cimport cls_application
from ..workspace cimport cls_script
from ..workspace cimport cls_block
from ..workspace cimport cls_script
from . cimport tokens

cdef class ClsCompiler():
    cdef cls_application.ClsApplication ClsApp
    cdef public list[str] lib_path
    cdef public cls_script.ClsScript script

    cpdef void Catch(
      self, 
      int i, 
      str message = ?
    )
    cdef list _tokenizer(
      self, 
      cls_script.ClsScript _script
    )
    cdef list[list[tokens.tokenTemplate]] _parsing(
      self, 
      list[list[tokens.tokenTemplate]] _byte_tokenize
    )    
    cdef list[tokens.DeclareToken] _parsing_args(
      self, 
      list[tokens.tokenTemplate] declaresCode, 
      list[tokens.FunctionToken] Environment = *
    )
    cdef list[tokens.tokenTemplate] _structureExpression(
      self, 
      list[tokens.tokenTemplate] ExpressionCode, 
      list[tokens.FunctionToken] Environment, 
      str Param_mode = ?
    )
    cdef cls_block.ClsBlock _structureSentence(
      self, 
      list[list[tokens.tokenTemplate]] SentenceCode, 
      list[tokens.FunctionToken] Environment = *, 
      str mode = ?
    )
    cpdef cls_script.ClsScript Compile(self, cls_script.ClsScript _script)
