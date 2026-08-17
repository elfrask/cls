@echo off
REM Build: clx
cd /d "%~dp0.."
cargo build --bin clx %*
