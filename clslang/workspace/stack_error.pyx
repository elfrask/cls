from . cimport cls_script
from . cimport cls_application

cdef class StackError:

  def __init__(self, int index, cls_script.ClsScript script, cls_application.ClsApplication ClsApp, str message = "") -> None:

    self.index = index
    self.script = script
    self.ClsApp = ClsApp
    self.message = message
    pass
  cpdef str generateLog(self):

    cdef int column = 0
    cdef int row = 0
    cdef int count = 0
    cdef int index = self.index
    cdef str code = self.script._code

    cdef list[str] code_split = code.split("\n")

    for i in code_split:

      if (len(i) + count) > index:
        break  

      row += 1
      count += len(i) + 1
    

    column = index - count


    return "\n".join([
      f"====================================================",
      # f"DEBUG: col: {column} row: {row} index: {index} count: {count} repr zone: {repr(code[count-5:count + 5])}",
      # f"code spliter: {code_split}",
      f"file: {self.ClsApp.cwd}/{self.script.name_module}".replace("\\", "/"),
      f"line: {row+1} column: {column+1}",
      *code_split[row-2: row],
      f"{code_split[row]}",
      f"{' '*column}^",
      *code_split[row+1: row+2],
      f"",
      f"message: {self.message}",
      f""
    ])