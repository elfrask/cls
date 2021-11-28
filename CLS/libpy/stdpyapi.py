from clsengine.clsengine import ObjectCls as Api

class Async_Error():
    def __init__(self, v=False, c=0, d="") -> None:
        self.is_error = bool(v)
        self.detailed = str(d)
        self.code = int(c)
        pass
    def set_error(self, v):
        self.is_error = bool(v)
        pass
    def set_code(self, v):
        self.code = int(v)
        pass
    def set_detailed(self, v):
        self.detailed = str(v)
        pass
    def __bool__(self):
        return self.is_error
    def __str__(self):
        return self.detailed
    def __int__(self):
        return self.code
            
    pass

Module = Api.Module