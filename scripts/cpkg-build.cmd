@echo off
REM Build: cpkg — compila el gestor de paquetes
cd /d "%~dp0.."
cargo build --bin cpkg %*
