@echo off
REM Empaqueta la extension VS Code CLS en un .vsix (via vsce) e instala con `code`.
REM Uso: release-vsix.cmd [--install] [--out <archivo>]
REM   --install   instala el vsix despues de empaquetar (code --install-extension --force)
REM   --out       nombre del vsix de salida (default: cls-lang.vsix en la raiz del repo)
setlocal

set "EXT_DIR=%~dp0..\.vscode\extensions\ccls-lang"
set "OUT=%~dp0..\cls-lang.vsix"
set "INSTALL=0"

:loop
if "%~1"=="" goto endloop
if /i "%~1"=="--install" set "INSTALL=1"
if /i "%~1"=="--out" (
    set "OUT=%~2"
    shift
)
shift
goto loop
:endloop

if not exist "%EXT_DIR%\package.json" (
    echo ERROR: no se encontro la extension en %EXT_DIR%
    exit /b 1
)

cd /d "%EXT_DIR%"
call npx --yes @vscode/vsce package --out "%OUT%"
if errorlevel 1 exit /b %errorlevel%

echo.
echo === Empaquetado: %OUT% ===
if "%INSTALL%"=="1" (
    call code --install-extension "%OUT%" --force
    if errorlevel 1 exit /b %errorlevel%
    echo Instalada.
)

endlocal