@echo off
set CC=sccache cl.exe
call py setup_e.py build_ext --embed 