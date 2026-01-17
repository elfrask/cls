from . cimport cls_script

cdef class StackError:

  def __init__(self, int index, cls_script.ClsScript script, str message = "") -> None:

    self.index = index
    self.script = script
    pass
  cdef str generateLog(self):

    pass