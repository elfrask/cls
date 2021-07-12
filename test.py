code = open("./py/libtest.py", "r").read()
from copy import copy as c
N="""
"""

vars = {"__x":{"out":{}}, "__c":c}

before = """
def __CODE():
"""

after ="""
    return __c(locals())
__x["out"] = __CODE()


"""

saliente = before+ (N+code).replace(N, N+"    ") +after
print(saliente)
print("============================")

exec(saliente, vars)
#exec(code + after, vars)

print(vars["__x"]["out"])