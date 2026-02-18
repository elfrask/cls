from ..compiler cimport tokens


cdef class ClsBlock():
    cdef list[tokens.tokenTemplate] ByteCodeScript
    cdef list[tokens.FunctionToken] EnvironmentFunctions

    cpdef list[tokens.tokenTemplate] getCode(self)
    cpdef list[tokens.tokenTemplate] getEnvironment(self)