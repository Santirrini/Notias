@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "C:\Users\Jose Diaz\Documents\Notias"
pnpm tauri build 2>&1