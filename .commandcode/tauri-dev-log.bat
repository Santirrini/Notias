@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "C:\Users\Jose Diaz\Documents\Notias"
echo Starting tauri dev at %DATE% %TIME% > "C:\Users\Jose Diaz\Documents\Notias\.commandcode\dev-out.log"
pnpm tauri dev >> "C:\Users\Jose Diaz\Documents\Notias\.commandcode\dev-out.log" 2>&1