

cdef str _repr(self, str added = ""):
        
    if added:
        added = " "+added

    return str(f"<TOKEN:{self.TypeToken} index={self.index}{added}>")


cdef class tokenTemplate():

    TypeToken = "TokenTemplate"
    cdef public int index 

    def __init__(self, int index = 0):

        self.index = index

        pass
    def __repr__(self) -> str:

        return _repr(self, "")


    pass


cdef class NameValue(tokenTemplate):
    TypeToken = "NameValue"
    # cdef public str Value
    cdef public str Value

    def __init__(self, str _value, index: int = 0):
        super().__init__(index)
        self.Value = _value

    def __repr__(self) -> str:

        return _repr(self, str(f"name='{self.Value}'"))

    pass


cdef class NumberValue(tokenTemplate):
    TypeToken = "NumberValue"
    cdef public str Value
    cdef public bint isFloat

    
    # Value = ""
    # isFloat = 0

    def __init__(self, str _value, bint isFloat = False, index: int = 0):
        super().__init__(index)
        self.Value = _value
        self.isFloat = isFloat

    def __repr__(self) -> str:

        return _repr(self, str(f"value='{self.Value}' isFloat={str(self.isFloat)}"))

    pass


cdef class SymbolToken(tokenTemplate):
    TypeToken = "SymbolToken"
    cdef public str symbol

    def __init__(self, str symbol, int index = 0):
        super().__init__(index)

        self.symbol = symbol
    def __repr__(self) -> str:

        return _repr(self, str(f"symbol='{self.symbol}'"))


cdef class OperatorToken(tokenTemplate):
    TypeToken = "OperatorToken"
    cdef public str _operator

    def __init__(self, str _operator, int index = 0):
        super().__init__(index)

        self._operator = _operator
    def __repr__(self) -> str:

        return _repr(self, str(f"operator='{self._operator}'"))



cdef class StringToken(tokenTemplate):
    TypeToken = "StringToken"
    cdef public str content
    cdef public str _operator
    cdef public str format

    def __init__(self, str content, str _operator, str format, int index = 0):
        super().__init__(index)

        self.content = content
        self._operator = _operator
        self.format = format
    def __repr__(self) -> str:

        return _repr(self, str(f"string: {self.format}{self._operator}{self.content}{self._operator}"))


cdef class NodeToken(tokenTemplate):

    TypeToken = "NodeToken"

    cdef public list[tokenTemplate] content
    cdef public list[list[tokenTemplate]] ContentComplex
    cdef public str format
    cdef public bint Multiline

    def __init__(self, str format, bint Multiline, int index = 0):
        super().__init__(index)

        self.format = format
        self.Multiline = Multiline

    cpdef void _set_content(self, list[list[tokenTemplate]] allContent):

        if allContent:
            self.content = allContent[0]
        else:
            self.content = []
        
        self.ContentComplex = allContent

        pass
    
    def __repr__(self) -> str:

        cdef str show

        if self.Multiline:
            show = str(self.ContentComplex)
        else:
            show = str(self.content)

        return str(f"NodeToken({self.index}, {self.format}, {show})")











        






# class tokenTemplate():

#     TypeToken = "TokenTemplate"
#     index = 0 

#     def __init__(self, index: int = 0):

#         self.index = index

#         pass
#     def __repr__(self) -> str:

#         return self.__c_repr()
#     def __c_repr(self, added: str = "") -> str:
        
#         if added:
#             added = " "+added

#         return str(f"<TOKEN:{self.TypeToken} index={self.index}{added}>")


#     pass


# class NameValue(tokenTemplate):
#     TypeToken = "NameValue"
#     Value = ""

#     def __init__(self, _value, index: int = 0):
#         super().__init__(index)
#         self.Value = _value

#     def __repr__(self) -> str:

#         return self.__c_repr(f"name='{self.Value}'")

#     pass


# class NumberValue(tokenTemplate):
#     TypeToken = "NumberValue"
#     Value = ""
#     isFloat = False

#     def __init__(self, _value, isFloat: bool = False, index: int = 0):
#         super().__init__(index)
#         self.Value = _value
#         self.isFloat = isFloat

#     def __repr__(self) -> str:

#         return self.__c_repr(f"value='{self.Value}' isFloat={str(self.isFloat)}")

#     pass




