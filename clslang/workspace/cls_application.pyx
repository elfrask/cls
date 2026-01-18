
cdef class ClsApplication():


    def __init__(self, str cwd, int pid):

        self.cwd = cwd
        self.pid = pid
        self.StacksErrors = []
        # self.AppModules = {}

        pass
    pass