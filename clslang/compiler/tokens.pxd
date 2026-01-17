# =========================================================================================
# === Declaración de la Jerarquía de Tokens para Tipado Estático ===
# =========================================================================================

# Declaración de funciones C a nivel de módulo
cdef str _repr(self, str added = ?)

# -----------------------------------------------------------------------------------------
# Primitive Tokens
# -----------------------------------------------------------------------------------------

cdef class tokenTemplate: # Token Padre
    
    # Atributos de C/C++
    cdef public int index 
    

cdef class NameValue(tokenTemplate):
    cdef public str Value
    cdef bint noMutable


cdef class NumberValue(tokenTemplate):
    cdef public str Value
    cdef public bint isFloat


cdef class SymbolToken(tokenTemplate):
    cdef public str symbol


cdef class OperatorToken(tokenTemplate):
    cdef public str _operator


cdef class StringToken(tokenTemplate):
    cdef public str content
    cdef public str _operator
    cdef public str format


cdef class NodeToken(tokenTemplate):
    # Nota: Los contenedores se declaran como 'list' sin el tipado genérico
    cdef public list content
    cdef public list ContentComplex
    cdef public str format
    cdef public bint Multiline

    # Declaración del método cpdef (con tipado Cython 'list')
    cpdef void _set_content(self, list allContent)



# -----------------------------------------------------------------------------------------
# Sentences Tokens (Usando otras clases cdef como tipado)
# -----------------------------------------------------------------------------------------

cdef class DeclareToken(tokenTemplate): # Declaraciones
    cdef public str VarName
    cdef public list ContextType  # Contiene tokenTemplate
    cdef public list DefaultExpression # Contiene tokenTemplate


cdef class FunctionToken(tokenTemplate): # Funciones
    # Usamos list porque contienen referencias a tokenTemplate, DeclareToken, o FunctionToken (que son cdef class)
    cdef public list ContentSentence
    cdef public list ContextFunctionsAnonymous
    cdef public list ParamsFunction # Contiene DeclareToken
    cdef public list ReturnType
    cdef public bint AnonymousFunction
    cdef public bint AsyncFunction
    cdef public str Scope
    cdef public str FunctionName 


cdef class WhileToken(tokenTemplate): # Ciclos While
    cdef public list ContentSentence
    cdef public list Condition


cdef class IfToken(tokenTemplate): # Sentencia If, Else y Else If
    cdef public list ContentSentence
    cdef public list Condition
    cdef public bint isElse


cdef class IfSequence(tokenTemplate): # Listas y secuencias condicionales If
    cdef public list listIfs 
    cpdef add(self, IfToken IfUnit)


cdef class ForToken(tokenTemplate): # For manuales
    cdef public list Declare # Contiene DeclareToken
    cdef public list Condition
    cdef public list IteratorSentence
    cdef public list ContentSentence


cdef class ForEachToken(tokenTemplate): # For automáticos "Each"
    cdef public str iteratorName
    cdef public str indexName
    cdef public list ArrayElement
    cdef public list ContentSentence



cdef class TryToken(tokenTemplate): # Sentencia Try, Catch y Finally
    cdef public list TryBlock
    cdef public list ExceptBlock
    cdef public list FinallyBlock
    cdef public list ExceptDeclare # Contiene DeclareToken



cdef class ClassToken(tokenTemplate): # Clases
    cdef public str ClassName
    cdef public list Extends
    cdef public list Body



cdef class ReturnToken(tokenTemplate): # Sentencia Return
    cdef public list Expression




cdef class SwitchToken(tokenTemplate): # Sentencias Switch
    cdef public list Expression
    cdef public list Cases



cdef class CaseToken(tokenTemplate): # Sentencias Case
    cdef public list Values
    cdef public list Body
    cdef public bint isDefault



cdef class ModuleToken(tokenTemplate): # Módulo
    cdef public str ModuleName
    cdef public list Body



cdef class StructureToken(tokenTemplate): # Estructuras e interfaces
    cdef public str StructureName
    cdef public list Fields # Contiene DeclareToken
    cdef public list Extends
    cdef public bint onlyTypeInterface



cdef class ImportToken(tokenTemplate): # Importación de librería
    cdef public str ModuleName
    cdef public str ImportedRoute


cdef class FromModuleToken(tokenTemplate): # Abstracción de módulos
    cdef public str NameElementModule
    cdef public str RenameModule


cdef class FromImportToken(tokenTemplate): # Importación from ... import ...
    # ModulesNames contiene FromModuleToken (cdef class)
    # cdef public list ModulesNames
    cdef public list[FromModuleToken] ModulesNames
    cdef public str ImportedRoute



cdef class IncludeToken(tokenTemplate): # Importación include
    cdef public str ImportedRoute



cdef class VarToken(tokenTemplate): # Variables/Constantes
    cdef public bint isConst
    cdef public list Declares # Contiene DeclareToken

    
cdef class WithToken(tokenTemplate): # Sentencia With
    cdef public str VarName
    cdef public list Values
    cdef public list Body


cdef class ExpressionSentence(tokenTemplate): # Expression secuencial / imperativa
    cdef public list[tokenTemplate] Body

cdef class AssignVarValue(tokenTemplate): # Asignación declarativa
    cdef public list[tokenTemplate] Expression
    cdef public str VarName

    cdef public bint complexAssignVar
    cdef public list[tokenTemplate] AssignVar
