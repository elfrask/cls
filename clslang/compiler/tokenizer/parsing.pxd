from .. cimport tokens
from .. cimport cls_compiler


cdef list[list[tokens.tokenTemplate]] _parsing(cls_compiler.ClsCompiler self, list[list[tokens.tokenTemplate]] _byte_tokenize)