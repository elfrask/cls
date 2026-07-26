@echo off
REM Build: ecls — compila el ejecutor
cd /d "%~dp0.."
cargo build --bin ecls %*
