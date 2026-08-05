@echo off
REM run-all.cmd — Ejecuta los tres benchmarks y deja que cada uno imprima sus tiempos.
REM Cada script usa los mismos parámetros (N_*) y la misma lógica.

echo.
echo ============================================
echo  1/3  CLS (tree-walker)
echo ============================================
"%~dp0..\..\scripts\clx.cmd" run benchmark.clsx
echo.

echo ============================================
echo  2/3  Node.js
echo ============================================
node benchmark.js
echo.

echo ============================================
echo  3/3  Python
echo ============================================
python benchmark.py
echo.
