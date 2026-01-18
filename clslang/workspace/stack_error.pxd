from . cimport cls_script
from . cimport cls_application


cdef class StackError:
  cdef int index
  cdef cls_script.ClsScript script
  cdef cls_application.ClsApplication ClsApp
  cdef str message

  cpdef str generateLog(self)
