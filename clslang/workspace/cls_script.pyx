


cdef class ClsScript():

    # cdef public str _code
    # cdef public str name_module
    # cdef public int id
    # cdef public cls_block.ClsBlock result

    def __init__(self, str code, str name_module, int ID = 0):

        self._code = code
        self.name_module = name_module
        self.id = ID

        pass

    pass
