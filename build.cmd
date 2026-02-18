@echo off
set CC=sccache cl.exe
call py setup.py build_ext --inplace 