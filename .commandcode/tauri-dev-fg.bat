@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "C:\Users\Jose Diaz\Documents\Notias"
echo === dev started at %DATE% %TIME% ===
pnpm tauri dev
echo === dev exited ===