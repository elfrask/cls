cdef dict _toks = {
    "ope":["+", "-", "/", "*", "!", "|", "@", "&", "%", "=", "?", "<", ">", "^", ":"],
    "multi-ope":["++", "--", "//", "**", "!=", "||", "==", "<<", ">>", "^^", "::", ":=", "<=", ">=", "->"],
    "sim":["{", "}", "(", ")", "[", "]", ","],
    "cond":["==", "<", ">", "!=", "<=", ">=", "!"],
    "open-close":{
        "{":"{}",
        "[":"[]",
        "(":"()",

    },
    "convert":{
        "condi":{"&":"and", "|":"or", "!":"not", "?":"in", "^":"**"},
        "expre-eval":{"++":"+1", "--":"-1"},
        "expre-exec":{"++":"+=1", "--":"-=1"},
    },
    "metodos":{"main":"__init__", "_call":"__call__", "_getitem":"__getitem__", "_setitem":"__setitem__",
        "_add":"__add__", "_sub":"__sub__", "_div":"__div__", "_delitem":"__delitem__", "_mul":"__mul__",
        "_mod":"__mod__", "_or":"__or__", "_and":"__and__", "_xor":"__xor__", "_or":"__or__", "_len":"__len__",
        "_repr":"__repr__", "_str":"__str__", "_int":"__int__", "_float":"__float__", "_array":"__list__", 
        "_dict":"__dict__"
    },
    "to_c":{"String":"str", "Array":"list", "Int":"int", "Float":"float", "Dictionary":"dict"}
}


cdef dict _nombre_reservados = {
    "visible":["export", "static", "private", "public", "global"],
    "thread":["sync", "async"],
    "nombre":[
            "func", "function", "class", "module", "with", "for", "if", "while", "define",
            "from", "import", "global", "try", "def", "fub", "method", "include", "using", "var", "const",
            "template", "switch", "structure", "case", "return", "setrule", "interface"
        ],
    "codi":["or", "in", "and", "is"],
    "bucle":["break", "continue"]
}