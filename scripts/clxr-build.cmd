@echo off
REM Build: clxr
cd /d "%~dp0.."
cargo build --bin clxr %*
