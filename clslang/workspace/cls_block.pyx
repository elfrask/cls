from ..compiler cimport tokens


cdef class ClsBlock():

    # cdef list[tokens.tokenTemplate] ByteCodeScript
    # cdef list[tokens.FunctionToken] EnvironmentFunctions

    def __init__(self, list[tokens.tokenTemplate] _ByteCodeScript, list EnvironmentFunctions) -> None:
        
        self.ByteCodeScript = _ByteCodeScript
        self.EnvironmentFunctions = EnvironmentFunctions
        pass
    
    cpdef list[tokens.tokenTemplate] getCode(self):

        return self.ByteCodeScript
    cpdef list[tokens.tokenTemplate] getEnvironment(self):

        return self.EnvironmentFunctions
    

    pass

