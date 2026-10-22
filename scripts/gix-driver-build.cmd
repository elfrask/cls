@echo off
REM Build: driver de git (gix) para clx -> clxgix.dll
REM Crate standalone (NO en el workspace): mantiene gix fuera del binario clx.
cd /d "%~dp0.."
cargo build --manifest-path drivers\gix-driver\Cargo.toml --release %*
