

cdef class StackParsingEviroment():

    # cdef public list[list[tokens.tokenTemplate]] output
    # cdef public list[tokens.tokenTemplate] line
    # cdef public int stackLevel
    # cdef public int index
    # cdef public str close
    # cdef public str format

    def __init__(self, str close = "", str format = "", int stackLevel = 0, int index = 0) -> None:

        self.line = []
        self.output = []
        self.stackLevel = stackLevel
        self.index = index
        self.close = close
        self.format = format

        pass
    
    cpdef get_data_returning(self):

        return self.output
    
    cdef set_next_line(self):

        # print("nextline: ", self.line)

        if self.line:
            self.output.append(self.line)
            self.line = []

        pass

    
    

    pass
