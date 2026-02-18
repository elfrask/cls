# cython: autogen_pxd=True
# =========================================================================================
# =================================== Primitive Tokens ====================================
# =========================================================================================


cdef str _repr(self, str added = ""):
        
    if added:
        added = " "+added

    return str(f"<TOKEN:{self.TypeToken} index={self.index}{added}>")


cdef class tokenTemplate(): #Token Padre

    TypeToken = "TokenTemplate"
    # cdef public int index 

    def __init__(self, int index = 0):

        self.index = index

        pass
    def __repr__(self) -> str:

        return _repr(self, "")


    pass


cdef class NameValue(tokenTemplate): #Nombres, variables
    TypeToken = "NameValue"
    # cdef public str Value
    # cdef public str Value
    # cdef bint noMutable

    def __init__(self, str _value, index: int = 0, bint noMutable = False):
        super().__init__(index)
        self.Value = _value
        self.noMutable = _value

    def __repr__(self) -> str:

        return _repr(self, str(f"name='{self.Value}'"))

    pass


cdef class NumberValue(tokenTemplate): # Números enteros y flotante
    TypeToken = "NumberValue"
    # cdef public str Value
    # cdef public bint isFloat

    
    # Value = ""
    # isFloat = 0

    def __init__(self, str _value, bint isFloat = False, index: int = 0):
        super().__init__(index)
        self.Value = _value
        self.isFloat = isFloat

    def __repr__(self) -> str:

        return _repr(self, str(f"value='{self.Value}' isFloat={str(self.isFloat)}"))

    pass


cdef class SymbolToken(tokenTemplate): # Símbolos () [] {} ","
    TypeToken = "SymbolToken"
    # cdef public str symbol

    def __init__(self, str symbol, int index = 0):
        super().__init__(index)

        self.symbol = symbol
    def __repr__(self) -> str:

        return _repr(self, str(f"symbol='{self.symbol}'"))


cdef class OperatorToken(tokenTemplate): # Operadores aritméticos y lógicos
    TypeToken = "OperatorToken"
    # cdef public str _operator

    def __init__(self, str _operator, int index = 0):
        super().__init__(index)

        self._operator = _operator
    def __repr__(self) -> str:

        return _repr(self, str(f"operator='{self._operator}'"))



cdef class StringToken(tokenTemplate): # Cadenas de texto
    TypeToken = "StringToken"
    # cdef public str content
    # cdef public str _operator
    # cdef public str format

    def __init__(self, str content, str _operator, str format, int index = 0):
        super().__init__(index)

        self.content = content
        self._operator = _operator
        self.format = format
    def __repr__(self) -> str:

        return _repr(self, str(f"string: {self.format}{self._operator}{self.content}{self._operator}"))


cdef class NodeToken(tokenTemplate): # Nodos anidados ([], {}, {[], ()}, {[]})

    TypeToken = "NodeToken"

    # cdef public list[tokenTemplate] content
    # cdef public list[list[tokenTemplate]] ContentComplex
    # cdef public str format
    # cdef public bint Multiline

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


# =========================================================================================
# =================================== Sentences Tokens ====================================
# =========================================================================================


cdef class DeclareToken(tokenTemplate): # Declaraciones

    TypeToken = "DeclareToken"

    # cdef public str VarName
    # cdef public list[tokenTemplate] ContextType
    # cdef public list[tokenTemplate] DefaultExpression


    def __init__(self, int index, str VarName, list[tokenTemplate] ContextType = [], list[tokenTemplate] DefaultExpression = []):
        super().__init__(index)

        self.DefaultExpression = DefaultExpression
        self.VarName = VarName
        self.ContextType = ContextType
    
    def __repr__(self) -> str:

        

        return str(f"index-{self.index}: {self.VarName}: ({self.ContextType}) = {self.DefaultExpression}")




cdef class FunctionToken(tokenTemplate): # Funciones

    TypeToken = "FunctionToken"

    # cdef public list[tokenTemplate] ContentSentence
    # cdef public list[FunctionToken] ContextFunctionsAnonymous
    # cdef public list[DeclareToken] ParamsFunction
    # cdef public list[tokenTemplate] ReturnType
    # cdef public bint AnonymousFunction
    # cdef public bint AsyncFunction
    # cdef public str Scope
    # cdef public str FunctionName 


    def __init__(
        self, 
        int index, 
        list[tokenTemplate] contentSentence, 
        str FunctionName = "",
        str Scope = "public",
        list[DeclareToken] ParamsFunction = [], 
        list[tokenTemplate] ReturnType = [],
        list[FunctionToken] ContextFunctionsAnonymous = [],
        bint AnonymousFunction = False,
        bint AsyncFunction = False,


    ):
        super().__init__(index)
        
        self.ContentSentence = contentSentence
        self.FunctionName = FunctionName
        self.ParamsFunction = ParamsFunction
        self.Scope = Scope
        self.ReturnType = ReturnType
        self.AnonymousFunction = AnonymousFunction
        self.AnonymousFunction = AsyncFunction


    def __repr__(self) -> str:

        

        # return str(f"NodeToken({self.index}, {self.format}, {show})")
        return str(f"index-{self.index}: function {self.FunctionName}({self.ParamsFunction}) -> {self.ReturnType} " + "{ ... }")


cdef class WhileToken(tokenTemplate): # Ciclos While

    TypeToken = "WhileToken"

    # cdef public list[tokenTemplate] ContentSentence
    # cdef public list[tokenTemplate] Condition


    def __init__(
        self, 
        int index,
        list[tokenTemplate] Condition,
        list[tokenTemplate] ContentSentence
    ):
        super().__init__(index)

        self.Condition = Condition
        self.ContentSentence = ContentSentence
        


    def __repr__(self) -> str:

        

        return str(f"index-{self.index}: while ({self.Condition}) " + "{ ... }")


cdef class IfToken(tokenTemplate): # Sentencia If, Else y Else If

    TypeToken = "IfToken"

    # cdef public list[tokenTemplate] ContentSentence
    # cdef public list[tokenTemplate] Condition
    # cdef public bint isElse


    def __init__(
        self, 
        int index,
        list[tokenTemplate] Condition,
        list[tokenTemplate] ContentSentence,
        bint isElse = False
    ):
        super().__init__(index)

        self.Condition = Condition
        self.ContentSentence = ContentSentence
        self.isElse = isElse
        


    def __repr__(self) -> str:

       

        if self.isElse:
            return str(f"index-{self.index}: else " + "{ ... }")

        return str(f"index-{self.index}: if ({self.Condition}) " + "{ ... }")

cdef class IfSequence(tokenTemplate): # Listas y secuencias condicionales If

    TypeToken = "IfSequence"

    # cdef public list[IfToken] listIfs


    def __init__(
        self, 
        int index, 
        list[tokenTemplate] listIfs = [] 

    ):
        super().__init__(index)
        
        self.listIfs = listIfs
    
    cpdef add(self, IfToken IfUnit):

        self.listIfs.append(IfUnit)


    def __repr__(self) -> str:

        

        # return str(f"NodeToken({self.index}, {self.format}, {show})")
        return str(f"index-{self.index}: IfSequence: {self.listIfs} ")


cdef class ForToken(tokenTemplate): # For manuales

    TypeToken = "ForToken"

    # cdef public list[DeclareToken] Declare
    # cdef public list[tokenTemplate] Condition
    # cdef public list[tokenTemplate] IteratorSentence
    # cdef public list[tokenTemplate] ContentSentence


    def __init__(
        self, 
        int index,
        list[DeclareToken] Declare,
        list[tokenTemplate] Condition,
        list[tokenTemplate] IteratorSentence,
        list[tokenTemplate] ContentSentence
    ):
        super().__init__(index)

        self.Condition = Condition
        self.Declare = Declare
        self.ContentSentence = ContentSentence
        self.IteratorSentence = IteratorSentence
        


    def __repr__(self) -> str:

        

        return str(f"index-{self.index}: for ({self.Declare} ; ... ; ...) " + "{ ... }")

cdef class ForEachToken(tokenTemplate): # For automáticos "Each"

    TypeToken = "ForEachToken"

    # cdef public str iteratorName
    # cdef public str indexName
    # cdef public list[tokenTemplate] ArrayElement
    # cdef public list[tokenTemplate] ContentSentence


    def __init__(
        self, 
        int index,
        str iteratorName,
        list[tokenTemplate] ArrayElement,
        list[tokenTemplate] ContentSentence,
        str indexName = "",
    ):
        super().__init__(index)

        self.iteratorName = iteratorName
        self.indexName = indexName
        self.ContentSentence = ContentSentence
        self.ArrayElement = ArrayElement
        


    def __repr__(self) -> str:

        if self.indexName:
            return str(f"index-{self.index}: for each {self.iteratorName} and {self.indexName} in (...) " + "{ ... }")


        return str(f"index-{self.index}: for each {self.iteratorName} in (...) " + "{ ... }")







cdef class TryToken(tokenTemplate): # Sentencia Try, Catch y Finally
    TypeToken = "TryToken"
    # cdef public list[tokenTemplate] TryBlock
    # cdef public list[tokenTemplate] ExceptBlock
    # cdef public list[tokenTemplate] FinallyBlock

    # cdef public list[DeclareToken] ExceptDeclare

    def __init__(
        self, 
        int index, 
        list[tokenTemplate] TryBlock = [], 
        list[DeclareToken] ExceptDeclare = [], 
        list[tokenTemplate] ExceptBlock = [], 
        list[tokenTemplate] FinallyBlock = []
    ):
        super().__init__(index)
        self.TryBlock = TryBlock
        self.ExceptDeclare = ExceptDeclare
        self.ExceptBlock = ExceptBlock
        self.FinallyBlock = FinallyBlock

    def __repr__(self) -> str:
        return str(f"index-{self.index}: try {{ ... }} except (...) {{ ... }} finally {{ ... }}")


cdef class ClassToken(tokenTemplate): # Plantillas de objetos, clases y encapsular métodos y atributos de objetos
    TypeToken = "ClassToken"
    # cdef public str ClassName
    # cdef public list[tokenTemplate] Extends
    # cdef public list[tokenTemplate] Body

    def __init__(self, int index, str ClassName, list[tokenTemplate] Extends = [], list[tokenTemplate] Body = []):
        super().__init__(index)
        self.ClassName = ClassName
        self.Extends = Extends
        self.Body = Body

    def __repr__(self) -> str:
        return str(f"index-{self.index}: class {self.ClassName}({self.BaseClasses}) {{ ... }}")


cdef class ReturnToken(tokenTemplate): # Sentencia para devolver valores al finalizar una función
    TypeToken = "ReturnToken"
    # cdef public list[tokenTemplate] Expression

    def __init__(self, int index, list[tokenTemplate] Expression = []):
        super().__init__(index)
        self.Expression = Expression

    def __repr__(self) -> str:
        return str(f"index-{self.index}: return ({self.Expression})")


cdef class SwitchToken(tokenTemplate): # Sentencias Switch
    TypeToken = "SwitchToken"
    # cdef public list[tokenTemplate] Expression
    # cdef public list[tokenTemplate] Cases

    def __init__(self, int index, list[tokenTemplate] Expression = [], list[tokenTemplate] Cases = []):
        super().__init__(index)
        self.Expression = Expression
        self.Cases = Cases

    def __repr__(self) -> str:
        return str(f"index-{self.index}: switch ({self.Expression}) {{ ... }}")


cdef class CaseToken(tokenTemplate): # Sentencias Case
    TypeToken = "CaseToken"
    # cdef public list[tokenTemplate] Values
    # cdef public list[tokenTemplate] Body
    # cdef public bint isDefault

    def __init__(self, int index, list[tokenTemplate] Values = [], list[tokenTemplate] Body = [], bint isDefault = False):
        super().__init__(index)
        self.Values = Values
        self.Body = Body
        self.isDefault = isDefault

    def __repr__(self) -> str:
        if self.isDefault:
            return str(f"index-{self.index}: case default: {{ ... }}")
        return str(f"index-{self.index}: case ({self.Values}): {{ ... }}")


cdef class ModuleToken(tokenTemplate): # Método de organización y agrupación de métodos y variables fuera del top-level
    TypeToken = "ModuleToken"
    # cdef public str ModuleName
    # cdef public list[tokenTemplate] Body

    def __init__(self, int index, str ModuleName, list[tokenTemplate] Body = []):
        super().__init__(index)
        self.ModuleName = ModuleName
        self.Body = Body

    def __repr__(self) -> str:
        return str(f"index-{self.index}: module {self.ModuleName} {{ ... }}")


cdef class StructureToken(tokenTemplate): # Sentencias e interfaces para tipado estático
    TypeToken = "StructureToken"
    # cdef public str StructureName
    # cdef public list[DeclareToken] Fields
    # cdef public list[tokenTemplate] Extends
    # cdef public bint onlyTypeInterface

    def __init__(self, int index, str StructureName, list[DeclareToken] Fields = [], list[DeclareToken] Extends = [], bint onlyTypeInterface = False):
        super().__init__(index)
        self.StructureName = StructureName
        self.Fields = Fields
        self.onlyTypeInterface = onlyTypeInterface
        self.Extends = Extends

    def __repr__(self) -> str:
        return str(f"index-{self.index}: structure {self.StructureName} {{ {self.Fields} }}")


cdef class ImportToken(tokenTemplate): # importa una librería y dale nombre
    TypeToken = "ImportToken"
    # cdef public str ModuleName
    # cdef public str ImportedRoute

    def __init__(self, int index, str ModuleName, str ImportedRoute = ""):
        super().__init__(index)
        self.ModuleName = ModuleName
        self.ImportedRoute = ImportedRoute

    def __repr__(self) -> str:
        return str(f"index-{self.index}: import '{self.ImportedRoute}' as {self.ModuleName}")

cdef class FromModuleToken(tokenTemplate): # parámetros de abstracción de módulos para from "..." import at1, at2 as _at, ...
    TypeToken = "FromModuleToken"
    # cdef public str NameElementModule
    # cdef public str RenameModule

    def __init__(self, int index, str NameElementModule, str RenameModule = ""):
        super().__init__(index)
        self.NameElementModule = NameElementModule
        self.RenameModule = RenameModule

    def __repr__(self) -> str:
        if self.RenameModule:
            return str(f"{self.NameElementModule} as {self.RenameModule}")
        else:
            return str(f"{self.NameElementModule}")
    def __str__(self) -> str:
        return self.__repr__()

cdef class FromImportToken(tokenTemplate): # importa una librería pero solo importa lo seleccionado y renombrados
    TypeToken = "FromImportToken"
    # cdef public list[FromModuleToken] ModulesNames
    # cdef public str ImportedRoute

    def __init__(self, int index, list[FromModuleToken] ModulesNames, str ImportedRoute = ""):
        super().__init__(index)
        self.ModulesNames = ModulesNames
        self.ImportedRoute = ImportedRoute

        

    def __repr__(self) -> str:

        names = []

        for i in self.ModulesNames:
            names.append(str(i))
            pass

        return str(f"index-{self.index}: from '{self.ImportedRoute}' import {', '.join(names)}")


cdef class IncludeToken(tokenTemplate): # importa una librería y incluye todos sus atributos al top-level
    TypeToken = "IncludeToken"
    # cdef public str ImportedRoute

    def __init__(self, int index, str ImportedRoute):
        super().__init__(index)
        self.ImportedRoute = ImportedRoute

    def __repr__(self) -> str:
        return str(f"index-{self.index}: include '{self.ImportedRoute}'")


cdef class VarToken(tokenTemplate):
    TypeToken = "VarToken"
    # cdef public bint isConst
    # cdef public list[DeclareToken] Declares

    def __init__(self, int index, list[DeclareToken] Declares = [], bint isConst = False):
        super().__init__(index)
        self.Declares = Declares
        self.isConst = isConst

    def __repr__(self) -> str:

        _v = "var"
        if self.isConst:
            _v = "const"

        return str(f"index-{self.index}: {_v} {self.Declares}")
        
cdef class WithToken(tokenTemplate): # Sentencia With
    TypeToken = "WithToken"

    # cdef public str VarName
    # cdef public list[tokenTemplate] Values
    # cdef public list[tokenTemplate] Body

    def __init__(self, int index, str VarName, list[tokenTemplate] Values, list[tokenTemplate] Body):
        super().__init__(index)
        self.VarName = VarName
        self.Values = Values
        self.Body = Body
    def __repr__(self) -> str:
        return str(f"index-{self.index}: with {self.VarName} in ( ... ) {{ ... }}")
    pass

cdef class ExpressionSentence(tokenTemplate): # Expression secuencial / imperativa
    TypeToken = "ExpressionSentence"
    def __init__(self, int index, list[tokenTemplate] body):
        super().__init__(index)
        self.Body = body

cdef class AssignVarValue(tokenTemplate): # Asignación declarativa

    def __init__(self, int index, list[tokenTemplate] AssignVar, list[tokenTemplate] Expression):
        super().__init__(index)
        self.AssignVar = AssignVar
        self.Expression = Expression
        self.complexAssignVar = True

        if len(AssignVar) == 1:
            if isinstance(AssignVar[0], NameValue):
                self.complexAssignVar = False
                self.VarName = AssignVar[0].Value
        