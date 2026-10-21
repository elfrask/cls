@echo off
pushd "%~dp0.."
call scripts\clx.cmd run cls-repositories\tombstone\src\seed.clsx %*
set RC=%ERRORLEVEL%
popd
exit /b %RC%
