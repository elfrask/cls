@echo off
REM Build: ccls — compila el CLI principal
cd /d "%~dp0.."
cargo build --bin ccls %*
