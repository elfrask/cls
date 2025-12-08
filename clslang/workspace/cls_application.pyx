from . cimport cls_script

cdef class ClsApplication():

    # cdef str cwd
    # cdef int pid
    # cdef public dict[cls_script.ClsScript] AppModules
    _api_base = {

    }

    def __init__(self, str cwd, int pid):

        self.cwd = cwd
        self.pid = pid
        self.AppModules = {}

        pass
    pass