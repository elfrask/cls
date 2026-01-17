from . cimport stack_error

cdef class ClsApplication():
    cdef public str cwd
    cdef public int pid
    cdef public list[stack_error.StackError] StacksErrors

    # cdef public dict[cls_script.ClsScript] AppModules
    # cdef public dict AppModules

