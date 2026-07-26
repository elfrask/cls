@echo off
REM Build: ccls-repl — compila el REPL
cd /d "%~dp0.."
cargo build --bin ccls-repl %*
