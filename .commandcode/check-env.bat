@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
where cl
where link
where cargo
where rustc
echo --- LIB env var ---
echo %LIB%
echo --- INCLUDE env var ---
echo %INCLUDE%