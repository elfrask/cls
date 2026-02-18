from . cimport cls_block


cdef class ClsScript():

    cdef public str _code
    cdef public str name_module
    cdef public int id
    cdef public cls_block.ClsBlock result
    # def __init__(self, str code, str name_module, int ID = 0):
