from ..compiler cimport tokens
from ..compiler cimport cls_compiler

cdef class subjectFind():

    cdef object tokenType
    cdef dict params
    cpdef bint checkEval(self, tokens.tokenTemplate token)



cdef autoToken(str string, int i)
cdef es_decimal_cython(str s)
cdef compare(list[tokens.tokenTemplate] Expression, list[subjectFind] check)
cdef token2SimpleString(tokens.tokenTemplate token)

cpdef list[tokens.FromModuleToken] getListName(cls_compiler.ClsCompiler self, list[tokens.tokenTemplate] lista = *)