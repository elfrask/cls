from setuptools import setup, Extension, find_packages
from Cython.Build import cythonize
import sys
import subprocess
import glob
import os, sys

files = [*glob.glob("./clslang/**/*.pyx", recursive=True)]

def Exp(file: str):
  
  point = os.path.splitext(file)[0].replace("\\", "/").replace("/", ".")

  while point[0] == ".":
    point = point[1:]
  
  return Extension(point, [file])

Extenciones = [Exp(file) for file in files] 
_i = 0
for i in files:
  print("Select: ", _i, i)
  _i+=1

extensions = [
    # Extension("clslang._tokens", ["clslang/_tokens.pyx"]),
    # Extension("clslang.engine", ["clslang/engine.pyx"], depends=["clslang/_tokens.pyx"]),
    # Extension("clslang._lib", ["clslang/_lib.pyx"], depends=["clslang/_lib.pyx"]),
    *Extenciones

]

setup(
    name="mi_paquete",
    ext_modules=cythonize(
        extensions, 
        language_level="3"
    ),
    packages=["clslang"], 
    package_dir={"": "."},
    options={"build": {"build_lib": "."}}, 
)