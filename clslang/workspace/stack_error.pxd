from . cimport cls_script


cdef class StackError:
  cdef int index
  cdef cls_script.ClsScript script

  cdef str generateLog(self)
