from .. cimport tokens
from ...workspace cimport stack_parsing
from .. cimport tokens_reserve
from .. cimport cls_compiler

cdef dict _toks = tokens_reserve._toks


cdef list[list[tokens.tokenTemplate]] _parsing(cls_compiler.ClsCompiler self, list[list[tokens.tokenTemplate]] _byte_tokenize):


    cdef list[stack_parsing.StackParsingEviroment] stack = []
    cdef stack_parsing.StackParsingEviroment _current_level = stack_parsing.StackParsingEviroment("", "", 0, 0)
    cdef stack_parsing.StackParsingEviroment _before_current_level = stack_parsing.StackParsingEviroment("", "", 0, 0)
    cdef NodeToken = None

    
    # stack.append(_current_level)
    
    cdef int index_stack = 0


    
    
    for x in _byte_tokenize:
        
        for y in x:

            if isinstance(y, tokens.SymbolToken):

                if y.symbol in ["(", "[", "{"]:

                    stack.append(_current_level)

                    _current_level = stack_parsing.StackParsingEviroment(
                        _toks["open-close"].get(y.symbol)[1],
                        _toks["open-close"].get(y.symbol), 
                        index_stack+1, 
                        y.index
                    )

                    index_stack += 1


                    pass
                elif y.symbol == _current_level.close:

                    _current_level.set_next_line()

                    # print("Cerrando con:", y)

                    _before_current_level = stack.pop()

                    NodeToken = tokens.NodeToken(
                        _current_level.format, 
                        _current_level.format in ["{}"], 
                        _current_level.index
                    )

                    NodeToken._set_content(_current_level.get_data_returning())

                    _before_current_level.line.append(
                        NodeToken
                    )

                    _current_level = _before_current_level

                    index_stack -=1

                    

                    pass
                else:
                    _current_level.line.append(
                        y
                    )


                pass
            else:
                _current_level.line.append(
                    y
                )


            pass
        
        _current_level.set_next_line()


        pass
    
    _current_level.set_next_line()
    



    return _current_level.get_data_returning()

