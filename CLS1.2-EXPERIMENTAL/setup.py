from setuptools import setup, Extension, find_packages
from Cython.Build import cythonize
import sys
import subprocess


# subprocess.run(["autopxd", "clslang/_tokens.pyx", "clslang/_tokens.pxd"], check=True)


extensions = [
    Extension("clslang._tokens", ["clslang/_tokens.pyx"]),
    Extension("clslang.engine", ["clslang/engine.pyx"], depends=["clslang/_tokens.pyx"]),
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