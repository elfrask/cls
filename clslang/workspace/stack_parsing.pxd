from ..compiler cimport tokens

cdef class StackParsingEviroment():

    cdef public list[list[tokens.tokenTemplate]] output
    cdef public list[tokens.tokenTemplate] line
    cdef public int stackLevel
    cdef public int index
    cdef public str close
    cdef public str format

    # def __init__(self, str close = "", str format = "", int stackLevel = 0, int index = 0) -> None:
    cpdef get_data_returning(self)
    cdef set_next_line(self)
